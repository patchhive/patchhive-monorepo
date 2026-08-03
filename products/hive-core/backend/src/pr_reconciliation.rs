use std::{sync::OnceLock, time::Duration};

use crate::{
    db,
    models::{now_rfc3339, PrReconciliationFailure, PrReconciliationState, PrReservationState},
};
use patchhive_product_core::hivecore_kernel::WorkOutcomeKind;

static RECONCILIATION_LOOP_STARTED: OnceLock<()> = OnceLock::new();

pub fn start_background_loop() {
    if RECONCILIATION_LOOP_STARTED.set(()).is_err() {
        return;
    }
    let client = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(20))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            tracing::error!(%error, "could not build PR lifecycle reconciliation client");
            return;
        }
    };
    tokio::spawn(async move {
        loop {
            reconcile(&client).await;
            tokio::time::sleep(Duration::from_secs(reconciliation_interval_seconds())).await;
        }
    });
}

async fn reconcile(client: &reqwest::Client) {
    if !patchhive_github_data::github_token_configured() {
        record(PrReconciliationState::NotConfigured {
            checked_at: now_rfc3339(),
        });
        return;
    }

    let started_at = now_rfc3339();
    if let Err(error) = db::record_pr_reconciliation_state(&PrReconciliationState::Running {
        started_at: started_at.clone(),
    }) {
        tracing::error!(%error, "could not claim PR lifecycle reconciliation");
        return;
    }

    let reservations = match db::committed_pr_reservations() {
        Ok(reservations) => reservations,
        Err(error) => {
            record(PrReconciliationState::Failed {
                started_at,
                failed_at: now_rfc3339(),
                checked: 0,
                open: 0,
                released: 0,
                reason: format!("Could not load committed PR reservations: {error}"),
                failures: Vec::new(),
            });
            return;
        }
    };

    let mut checked = 0_u32;
    let mut open = 0_u32;
    let mut released = 0_u32;
    let mut failures = Vec::new();
    for reservation in reservations {
        let PrReservationState::Committed { pr_url, .. } = &reservation.lifecycle else {
            continue;
        };
        checked += 1;
        let result =
            reconcile_reservation(client, &reservation.id, &reservation.repository, pr_url).await;
        match result {
            Ok(ReconciliationOutcome::Open) => open += 1,
            Ok(ReconciliationOutcome::Released) => released += 1,
            Ok(ReconciliationOutcome::NoLongerCommitted) => {}
            Err(error) => failures.push(PrReconciliationFailure {
                reservation_id: reservation.id,
                pr_url: pr_url.clone(),
                reason: error.to_string(),
            }),
        }
    }

    let completed_at = now_rfc3339();
    if failures.is_empty() {
        record(PrReconciliationState::Succeeded {
            started_at,
            completed_at,
            checked,
            open,
            released,
        });
    } else {
        record(PrReconciliationState::Failed {
            started_at,
            failed_at: completed_at,
            checked,
            open,
            released,
            reason: format!(
                "{} of {checked} committed PR reservations could not be reconciled",
                failures.len()
            ),
            failures,
        });
    }
}

enum ReconciliationOutcome {
    Open,
    Released,
    NoLongerCommitted,
}

async fn reconcile_reservation(
    client: &reqwest::Client,
    reservation_id: &str,
    expected_repository: &str,
    pr_url: &str,
) -> anyhow::Result<ReconciliationOutcome> {
    let (repository, number) = parse_github_pr_url(pr_url)?;
    let normalized_repository =
        patchhive_product_core::scope_policy::normalize_repo_name(&repository)
            .ok_or_else(|| anyhow::anyhow!("PR URL contains an invalid repository"))?;
    anyhow::ensure!(
        normalized_repository == expected_repository,
        "PR URL repository does not match its reservation"
    );
    let pull = patchhive_github_data::fetch_pull_request(client, &repository, number).await?;
    match pull.state.as_str() {
        "open" => Ok(ReconciliationOutcome::Open),
        "closed" => {
            let merged = pull.merged_at.is_some();
            let reason = if merged {
                "GitHub reconciliation observed the pull request was merged."
            } else {
                "GitHub reconciliation observed the pull request was closed."
            };
            let released = db::release_reconciled_pr_reservation(
                reservation_id,
                pr_url,
                reason,
                &now_rfc3339(),
            )?;
            if released {
                let reservation = db::pr_budget_reservation(reservation_id)?
                    .ok_or_else(|| anyhow::anyhow!("released reservation disappeared"))?;
                let work = db::record_reconciled_pr_outcome(
                    &reservation,
                    pr_url,
                    if merged {
                        WorkOutcomeKind::Merged
                    } else {
                        WorkOutcomeKind::ClosedUnmerged
                    },
                    reason,
                    &now_rfc3339(),
                )?;
                if !merged {
                    if let Some(work) = work {
                        let candidate = patchhive_product_core::repo_memory::FailGuardCandidateRequest {
                            repo: reservation.repository.clone(),
                            source_type: "HiveCore PR outcome".into(),
                            source_ref: work.id.clone(),
                            title: "PatchHive pull request closed without merge".into(),
                            outcome: reason.into(),
                            lesson: "Review the public maintainer outcome before similar autonomous work is attempted again.".into(),
                            prevention: "Use the promoted lesson, owner cooldown, and reputation governor to narrow future work.".into(),
                            affected_paths: Vec::new(),
                            evidence: vec![pr_url.into(), format!("work_item:{}", work.id)],
                            confidence: Some(1.0),
                        };
                        match patchhive_product_core::repo_memory::submit_failguard_candidate(
                            client, &candidate,
                        )
                        .await
                        {
                            Ok(Some(response)) => {
                                db::record_suite_event(
                                    "work_item",
                                    &work.id,
                                    "failguard_feedback_submitted",
                                    &serde_json::to_value(response)
                                        .unwrap_or(serde_json::Value::Null),
                                )?;
                            }
                            Ok(None) => {
                                db::record_suite_event(
                                    "work_item",
                                    &work.id,
                                    "failguard_feedback_not_configured",
                                    &serde_json::json!({"repository": reservation.repository}),
                                )?;
                            }
                            Err(error) => {
                                db::record_suite_event(
                                    "work_item",
                                    &work.id,
                                    "failguard_feedback_failed",
                                    &serde_json::json!({"reason": error.to_string()}),
                                )?;
                            }
                        }
                    }
                }
            }
            Ok(if released {
                ReconciliationOutcome::Released
            } else {
                ReconciliationOutcome::NoLongerCommitted
            })
        }
        state => anyhow::bail!("GitHub returned unrecognized pull-request state '{state}'"),
    }
}

fn parse_github_pr_url(value: &str) -> anyhow::Result<(String, u32)> {
    let url = reqwest::Url::parse(value)?;
    anyhow::ensure!(
        url.scheme() == "https" && url.host_str() == Some("github.com"),
        "PR URL must use https://github.com"
    );
    let segments = url
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>())
        .unwrap_or_default();
    anyhow::ensure!(
        segments.len() == 4 && segments[2] == "pull",
        "PR URL must have the form https://github.com/owner/repository/pull/number"
    );
    let repository = format!("{}/{}", segments[0], segments[1]);
    anyhow::ensure!(
        patchhive_github_data::valid_repo(&repository),
        "PR URL contains an invalid repository"
    );
    let number = segments[3]
        .parse::<u32>()
        .map_err(|_| anyhow::anyhow!("PR URL contains an invalid pull-request number"))?;
    anyhow::ensure!(number > 0, "PR URL pull-request number must be positive");
    Ok((repository, number))
}

fn record(state: PrReconciliationState) {
    if let Err(error) = db::record_pr_reconciliation_state(&state) {
        tracing::error!(%error, "could not record PR lifecycle reconciliation state");
    } else if matches!(state, PrReconciliationState::Failed { .. }) {
        tracing::warn!("PR lifecycle reconciliation failed");
    }
}

fn reconciliation_interval_seconds() -> u64 {
    std::env::var("HIVE_CORE_PR_RECONCILE_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(300)
        .clamp(30, 3_600)
}

#[cfg(test)]
mod tests {
    use super::parse_github_pr_url;

    #[test]
    fn parses_only_canonical_github_pull_request_urls() {
        assert_eq!(
            parse_github_pr_url("https://github.com/PatchHive/example/pull/42").unwrap(),
            ("PatchHive/example".into(), 42)
        );
        assert!(parse_github_pr_url("http://github.com/a/b/pull/1").is_err());
        assert!(parse_github_pr_url("https://example.com/a/b/pull/1").is_err());
        assert!(parse_github_pr_url("https://github.com/a/b/issues/1").is_err());
        assert!(parse_github_pr_url("https://github.com/a/b/pull/0").is_err());
    }
}

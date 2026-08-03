use patchhive_product_core::{
    approvals::ApprovalOrigin,
    hivecore_kernel::{evaluate_autonomy, AdmissionDecision, AutonomyLevel},
};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    conductor::{WorkClaim, WorkLifecycle},
    db,
    models::DispatchActionResponse,
    pipeline::{dispatch::dispatch_with_approval, governance::observe_github_rate},
    state::AppState,
};

const WORK_LEASE_SECONDS: u32 = 900;
const ESTIMATED_AI_CENTS: u64 = 100;

#[derive(Debug, Clone, Serialize)]
pub struct WorkCycleReport {
    pub claimed: u32,
    pub settled: u32,
    pub deferred: u32,
    pub failed: u32,
}

pub async fn run_once(state: &AppState, limit: u32) -> WorkCycleReport {
    let mut report = WorkCycleReport {
        claimed: 0,
        settled: 0,
        deferred: 0,
        failed: 0,
    };
    for _ in 0..limit.clamp(1, 10) {
        let claim = match db::claim_next_work(WORK_LEASE_SECONDS) {
            Ok(Some(claim)) => claim,
            Ok(None) => break,
            Err(error) => {
                tracing::error!(%error, "could not claim HiveCore work");
                report.failed += 1;
                break;
            }
        };
        report.claimed += 1;
        match execute_claim(state, claim).await {
            WorkSettlement::Settled => report.settled += 1,
            WorkSettlement::Deferred => report.deferred += 1,
            WorkSettlement::Failed => report.failed += 1,
        }
    }
    report
}

enum WorkSettlement {
    Settled,
    Deferred,
    Failed,
}

async fn execute_claim(state: &AppState, claim: WorkClaim) -> WorkSettlement {
    let item = &claim.item;
    let mandate = match item.proposal.mandate_id.as_deref() {
        Some(id) => match db::mandate(id) {
            Ok(Some(mandate)) if mandate.lifecycle.is_active() => Some(mandate),
            Ok(Some(_)) => {
                return settle_blocked(&claim, "The originating mandate is not active.", true)
            }
            Ok(None) => {
                return settle_blocked(&claim, "The originating mandate no longer exists.", false)
            }
            Err(error) => {
                return settle_blocked(
                    &claim,
                    &format!("The originating mandate could not be read: {error}"),
                    true,
                )
            }
        },
        None => None,
    };
    let requested = mandate
        .as_ref()
        .map(|value| value.config.requested_autonomy)
        .unwrap_or(AutonomyLevel::Propose);
    let mut autonomy = evaluate_autonomy(requested, &db::smoke_authority());
    if db::reputation_summary().is_ok_and(|summary| summary.slowdown_active)
        && autonomy.effective > AutonomyLevel::Propose
    {
        autonomy.effective = AutonomyLevel::Propose;
        autonomy.demotion_reason =
            Some("The rolling PR-outcome reputation governor is slowing autonomous writes.".into());
    }
    if autonomy.effective == AutonomyLevel::Observe {
        return settle_blocked(
            &claim,
            autonomy
                .demotion_reason
                .as_deref()
                .unwrap_or("Smoke evidence has not earned work dispatch authority."),
            true,
        );
    }

    let repository = &item.proposal.identity.repository;
    match db::blocking_pauses(
        Some(&item.proposal.proposed_dispatch.product_slug),
        item.proposal.mandate_id.as_deref(),
        Some(repository),
    ) {
        Ok(pauses) if pauses.is_empty() => {}
        Ok(pauses) => {
            let scopes = pauses
                .iter()
                .map(|pause| pause.target.storage_key())
                .collect::<Vec<_>>()
                .join(", ");
            return settle_blocked(
                &claim,
                &format!("Durable pause authority blocks: {scopes}."),
                true,
            );
        }
        Err(error) => {
            return settle_blocked(
                &claim,
                &format!("Pause authority could not be read: {error}"),
                true,
            )
        }
    }

    let github_rate = observe_github_rate(state, crate::models::now_rfc3339()).await;
    let resources =
        match db::claim_work_resources(item, github_rate, ESTIMATED_AI_CENTS, WORK_LEASE_SECONDS) {
            Ok(Ok(resources)) => resources,
            Ok(Err((admission, evidence))) => {
                let reason = format!(
                    "Resource admission denied: {}",
                    admission_reason(&admission)
                );
                let now = chrono::Utc::now();
                return settle_claim(
                    &claim,
                    WorkLifecycle::Blocked {
                        reason: reason.clone(),
                        blocked_at: now.to_rfc3339(),
                        retryable: true,
                        next_attempt_at: Some((now + chrono::Duration::minutes(5)).to_rfc3339()),
                    },
                    "blocked",
                    json!({"reason": reason, "admission": admission, "evidence": evidence}),
                );
            }
            Err(error) => {
                return settle_blocked(
                    &claim,
                    &format!("Resource admission could not be evaluated: {error}"),
                    true,
                )
            }
        };

    let mut action_id = item.proposal.proposed_dispatch.action_id.clone();
    if autonomy.effective == AutonomyLevel::Propose
        && item.proposal.proposed_dispatch.product_slug == "repo-reaper"
        && action_id == "run"
    {
        action_id = "dry_run".into();
    }
    let (heartbeat_stop, mut heartbeat_stopped) = tokio::sync::oneshot::channel();
    let heartbeat_work_item_id = item.id.clone();
    let heartbeat_claim_id = claim.claim_id.clone();
    let heartbeat_resources = resources.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut heartbeat_stopped => break,
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                    match db::renew_work_claim_resources(
                        &heartbeat_work_item_id,
                        &heartbeat_claim_id,
                        &heartbeat_resources,
                        WORK_LEASE_SECONDS,
                    ) {
                        Ok(true) => {}
                        Ok(false) => break,
                        Err(error) => {
                            tracing::error!(work_item_id = %heartbeat_work_item_id, %error, "could not renew work resource lease");
                            break;
                        }
                    }
                }
            }
        }
    });
    let mut dispatch_input = item.proposal.proposed_dispatch.input.clone();
    if let (Some(mandate_id), Some(input)) = (
        item.proposal.mandate_id.as_ref(),
        dispatch_input.as_object_mut(),
    ) {
        input.insert(
            "hivecore_mandate_id".into(),
            serde_json::Value::String(mandate_id.clone()),
        );
    }
    let response = dispatch_with_approval(
        state,
        &item.proposal.proposed_dispatch.product_slug,
        &action_id,
        dispatch_input,
        ApprovalOrigin::WorkItem {
            work_item_id: item.id.clone(),
        },
        None,
    )
    .await;
    let _ = heartbeat_stop.send(());

    match response {
        Ok(DispatchActionResponse::ApprovalRequired { approval }) => {
            let _ = db::settle_work_resources(&resources, None, "Waiting for operator approval");
            let lifecycle = WorkLifecycle::AwaitingApproval {
                approval_id: approval.id.clone(),
                requested_at: crate::models::now_rfc3339(),
            };
            settle_claim(
                &claim,
                lifecycle,
                "approval_requested",
                json!({"approval": approval, "admission": resources.admission, "evidence": resources.evidence}),
            )
        }
        Ok(DispatchActionResponse::Dispatched { event, .. }) => {
            let normalized = normalized_response(&event.response_json);
            let actual_cents = response_cost_cents(&normalized).or(Some(ESTIMATED_AI_CENTS));
            let _ = db::settle_work_resources(&resources, actual_cents, "Dispatch settled");
            if event.status != "dispatched" {
                return settle_failed(&claim, &event.error, false, json!({"event": event}));
            }
            let lifecycle = lifecycle_after_dispatch(&action_id, &event.id, &normalized);
            settle_claim(
                &claim,
                lifecycle,
                "dispatch_settled",
                json!({"event": event, "normalized_response": normalized, "admission": resources.admission, "evidence": resources.evidence}),
            )
        }
        Err((status, body)) => {
            let _ = db::settle_work_resources(&resources, None, "Dispatch failed");
            settle_failed(
                &claim,
                &format!("HiveCore dispatch returned {status}: {}", body.0.status),
                false,
                serde_json::to_value(body.0).unwrap_or(Value::Null),
            )
        }
    }
}

fn admission_reason(decision: &AdmissionDecision) -> String {
    match decision {
        AdmissionDecision::Admitted { .. } => "admitted".into(),
        AdmissionDecision::Denied { denial } => denial.reasons.join(" "),
    }
}

fn settle_blocked(claim: &WorkClaim, reason: &str, retryable: bool) -> WorkSettlement {
    let now = chrono::Utc::now();
    settle_claim(
        claim,
        WorkLifecycle::Blocked {
            reason: reason.into(),
            blocked_at: now.to_rfc3339(),
            retryable,
            next_attempt_at: retryable.then(|| (now + chrono::Duration::minutes(5)).to_rfc3339()),
        },
        "blocked",
        json!({"reason": reason}),
    )
}

fn settle_failed(
    claim: &WorkClaim,
    reason: &str,
    retryable: bool,
    evidence: Value,
) -> WorkSettlement {
    let now = chrono::Utc::now();
    settle_claim(
        claim,
        WorkLifecycle::Failed {
            reason: reason.into(),
            failed_at: now.to_rfc3339(),
            retryable,
            next_attempt_at: retryable.then(|| (now + chrono::Duration::minutes(5)).to_rfc3339()),
        },
        "failed",
        evidence,
    )
}

fn settle_claim(
    claim: &WorkClaim,
    lifecycle: WorkLifecycle,
    event: &str,
    evidence: Value,
) -> WorkSettlement {
    match db::settle_work_claim(&claim.item.id, &claim.claim_id, lifecycle, event, &evidence) {
        Ok(Some(_)) if event == "blocked" => WorkSettlement::Deferred,
        Ok(Some(_)) => WorkSettlement::Settled,
        Ok(None) => WorkSettlement::Failed,
        Err(error) => {
            tracing::error!(work_item_id = %claim.item.id, %error, "could not settle work claim");
            WorkSettlement::Failed
        }
    }
}

pub(crate) fn normalized_response(value: &Value) -> Value {
    let Some(raw) = value.get("raw").and_then(Value::as_str) else {
        return value.clone();
    };
    raw.lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .filter_map(|data| serde_json::from_str::<Value>(data.trim()).ok())
        .next_back()
        .unwrap_or_else(|| value.clone())
}

pub(crate) fn lifecycle_after_dispatch(
    action_id: &str,
    action_event_id: &str,
    response: &Value,
) -> WorkLifecycle {
    if let Some(pr_url) = find_string(response, &["pr_url", "html_url"]) {
        return WorkLifecycle::Shipped {
            pr_url,
            shipped_at: crate::models::now_rfc3339(),
        };
    }
    let receiving_run_id = find_string(response, &["run_id", "scan_id", "job_id"]);
    let created_changes = response
        .get("total_fixed")
        .and_then(Value::as_u64)
        .is_some_and(|count| count > 0);
    if action_id == "run" && created_changes && receiving_run_id.is_some() {
        return WorkLifecycle::Dispatched {
            action_event_id: action_event_id.to_owned(),
            receiving_run_id,
            dispatched_at: crate::models::now_rfc3339(),
        };
    }
    WorkLifecycle::Completed {
        outcome: if action_id == "dry_run" {
            "assessed"
        } else {
            "completed"
        }
        .into(),
        completed_at: crate::models::now_rfc3339(),
    }
}

pub(crate) fn response_cost_cents(value: &Value) -> Option<u64> {
    ["cost", "cost_usd", "total_cost_usd"]
        .iter()
        .find_map(|key| {
            value
                .get(key)
                .and_then(Value::as_f64)
                .filter(|cost| cost.is_finite() && *cost >= 0.0)
                .map(|cost| (cost * 100.0).ceil() as u64)
        })
}

fn find_string(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(values) => {
            for key in keys {
                if let Some(found) = values.get(*key).and_then(Value::as_str) {
                    if !found.trim().is_empty() {
                        return Some(found.to_owned());
                    }
                }
            }
            values.values().find_map(|value| find_string(value, keys))
        }
        Value::Array(values) => values.iter().find_map(|value| find_string(value, keys)),
        _ => None,
    }
}

static BACKGROUND_LOOP_STARTED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

pub fn start_background_loop() {
    if BACKGROUND_LOOP_STARTED.set(()).is_err() {
        return;
    }
    tokio::spawn(async {
        let state = AppState::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let report = run_once(&state, 3).await;
            if let Err(error) = db::reconcile_pause_drains() {
                tracing::error!(%error, "could not reconcile pause drain state");
            }
            if report.claimed > 0 {
                tracing::info!(?report, "HiveCore work cycle settled");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_repo_reaper_write_waits_for_pr_outcome_reconciliation() {
        let lifecycle = lifecycle_after_dispatch(
            "run",
            "event-1",
            &json!({"run_id": "run-1", "total_fixed": 1}),
        );
        assert!(matches!(
            lifecycle,
            WorkLifecycle::Dispatched {
                receiving_run_id: Some(run_id),
                ..
            } if run_id == "run-1"
        ));
    }

    #[test]
    fn dry_run_settles_as_an_assessment() {
        let lifecycle = lifecycle_after_dispatch(
            "dry_run",
            "event-1",
            &json!({"run_id": "run-1", "dry_run": true, "total_fixed": 0}),
        );
        assert!(matches!(
            lifecycle,
            WorkLifecycle::Completed { outcome, .. } if outcome == "assessed"
        ));
    }
}

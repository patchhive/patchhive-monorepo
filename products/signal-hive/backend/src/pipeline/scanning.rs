// scanning.rs — Scan execution, trend enrichment, report generation, scheduler

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use chrono::Utc;
use tracing::{info, warn};

use crate::models::{RepoSignalTrend, ScanRecord, ScanReport, ScanTrendSummary};
use crate::state::AppState;

use super::analysis::{analyze_repo_issue_draft, collect_marker_counts, finalize_repo_signal};
use super::scoring::MarkerCounts;
use super::utils::{
    clamp_params, format_scope_text, marker_scan_repo_limit, marker_total, recurring_issue_count,
    round1,
};

pub fn repo_trend_status(
    priority_delta: f64,
    stale_delta: i32,
    duplicate_delta: i32,
    marker_delta: i32,
    recurring_delta: i32,
) -> String {
    let worsening = priority_delta >= 5.0
        || stale_delta >= 2
        || duplicate_delta > 0
        || recurring_delta > 0
        || marker_delta >= 3;
    let improving = priority_delta <= -5.0
        || stale_delta <= -2
        || duplicate_delta < 0
        || recurring_delta < 0
        || marker_delta <= -3;

    if worsening && !improving {
        "rising".into()
    } else if improving && !worsening {
        "improving".into()
    } else {
        "steady".into()
    }
}

pub fn enrich_scan_trend(record: &mut ScanRecord, previous: &ScanRecord) {
    let mut previous_repos = previous
        .repos
        .iter()
        .cloned()
        .map(|repo| (repo.full_name.clone(), repo))
        .collect::<HashMap<_, _>>();

    let mut new_repos = 0u32;
    let mut rising_repos = 0u32;
    let mut improving_repos = 0u32;
    let mut steady_repos = 0u32;

    for repo in &mut record.repos {
        if let Some(previous_repo) = previous_repos.remove(&repo.full_name) {
            let marker_delta = marker_total(
                repo.todo_count,
                repo.fixme_count,
                repo.todo_available,
                repo.fixme_available,
            ) as i32
                - marker_total(
                    previous_repo.todo_count,
                    previous_repo.fixme_count,
                    previous_repo.todo_available,
                    previous_repo.fixme_available,
                ) as i32;
            let trend = RepoSignalTrend {
                status: repo_trend_status(
                    round1(repo.priority_score - previous_repo.priority_score),
                    repo.stale_issues as i32 - previous_repo.stale_issues as i32,
                    repo.duplicate_candidates.len() as i32
                        - previous_repo.duplicate_candidates.len() as i32,
                    marker_delta,
                    recurring_issue_count(&repo.recurring_bug_clusters)
                        - recurring_issue_count(&previous_repo.recurring_bug_clusters),
                ),
                compared_to_scan_id: previous.id.clone(),
                compared_to_created_at: previous.created_at.clone(),
                previous_priority_score: round1(previous_repo.priority_score),
                priority_delta: round1(repo.priority_score - previous_repo.priority_score),
                stale_delta: repo.stale_issues as i32 - previous_repo.stale_issues as i32,
                duplicate_delta: repo.duplicate_candidates.len() as i32
                    - previous_repo.duplicate_candidates.len() as i32,
                marker_delta,
                recurring_delta: recurring_issue_count(&repo.recurring_bug_clusters)
                    - recurring_issue_count(&previous_repo.recurring_bug_clusters),
            };

            match trend.status.as_str() {
                "rising" => rising_repos += 1,
                "improving" => improving_repos += 1,
                _ => steady_repos += 1,
            }

            repo.trend = Some(trend);
        } else {
            new_repos += 1;
            repo.trend = Some(RepoSignalTrend {
                status: "new".into(),
                compared_to_scan_id: previous.id.clone(),
                compared_to_created_at: previous.created_at.clone(),
                previous_priority_score: 0.0,
                priority_delta: round1(repo.priority_score),
                stale_delta: repo.stale_issues as i32,
                duplicate_delta: repo.duplicate_candidates.len() as i32,
                marker_delta: marker_total(
                    repo.todo_count,
                    repo.fixme_count,
                    repo.todo_available,
                    repo.fixme_available,
                ) as i32,
                recurring_delta: recurring_issue_count(&repo.recurring_bug_clusters),
            });
        }
    }

    record.trend = Some(ScanTrendSummary {
        compared_to_scan_id: previous.id.clone(),
        compared_to_created_at: previous.created_at.clone(),
        total_repos_delta: record.summary.total_repos as i32 - previous.summary.total_repos as i32,
        total_signals_delta: record.summary.total_signals as i32
            - previous.summary.total_signals as i32,
        new_repos,
        dropped_repos: previous_repos.len() as u32,
        rising_repos,
        improving_repos,
        steady_repos,
    });
}

pub fn build_scan_report(record: &ScanRecord) -> ScanReport {
    let top_repo = record.repos.first();
    let top_stale = record.repos.iter().max_by_key(|repo| repo.stale_issues);
    let top_recurring = record.repos.iter().max_by_key(|repo| {
        repo.recurring_bug_clusters
            .iter()
            .map(|cluster| cluster.issue_count)
            .sum::<u32>()
    });

    let mut lines = vec![
        "# SignalHive by PatchHive".into(),
        String::new(),
        "> Maintenance visibility before automation".into(),
        String::new(),
        format!("## Scan {}", record.id),
        String::new(),
        format!("- Scan ID: `{}`", record.id),
        format!("- Trigger: `{}`", record.trigger_type),
    ];

    if let Some(schedule_name) = &record.schedule_name {
        lines.push(format!("- Schedule: `{schedule_name}`"));
    }

    lines.extend([
        format!("- Scope: {}", format_scope_text(&record.params)),
        format!(
            "- Coverage: {} repos scanned, {} signals found, top repo `{}`",
            record.summary.total_repos, record.summary.total_signals, record.summary.top_repo
        ),
        String::new(),
    ]);

    lines.extend([
        "## Executive Readout".into(),
        String::new(),
        format!(
            "- {} repos scanned and {} maintenance signals surfaced.",
            record.summary.total_repos, record.summary.total_signals
        ),
        match top_repo {
            Some(repo) => format!(
                "- Highest priority repo: `{}` at {:.1} priority.",
                repo.full_name,
                round1(repo.priority_score)
            ),
            None => "- No ranked repos were returned in this scan.".into(),
        },
        match top_stale.filter(|repo| repo.stale_issues > 0) {
            Some(repo) => format!(
                "- Largest stale backlog: `{}` with {} stale issues.",
                repo.full_name, repo.stale_issues
            ),
            None => "- No stale backlog stood out in this scan.".into(),
        },
        match top_recurring.filter(|repo| !repo.recurring_bug_clusters.is_empty()) {
            Some(repo) => format!(
                "- Strongest recurring bug pressure: `{}` with {} recurring clusters covering {} issues.",
                repo.full_name,
                repo.recurring_bug_clusters.len(),
                recurring_issue_count(&repo.recurring_bug_clusters)
            ),
            None => "- No recurring bug cluster stood out in this scan.".into(),
        },
        String::new(),
    ]);

    if !record.warnings.is_empty() {
        lines.extend(["## Scan Warnings".into(), String::new()]);
        for warning in &record.warnings {
            lines.push(format!("- {warning}"));
        }
        lines.push(String::new());
    }

    if let Some(trend) = &record.trend {
        lines.extend([
            "## Trend vs Previous Similar Scan".into(),
            String::new(),
            format!(
                "- Compared to `{}` from {}",
                trend.compared_to_scan_id, trend.compared_to_created_at
            ),
            format!(
                "- Signals delta: {:+}, repos delta: {:+}",
                trend.total_signals_delta, trend.total_repos_delta
            ),
            format!(
                "- Queue movement: {} new, {} dropped, {} rising, {} improving, {} steady",
                trend.new_repos,
                trend.dropped_repos,
                trend.rising_repos,
                trend.improving_repos,
                trend.steady_repos
            ),
            String::new(),
        ]);
    }

    lines.extend(["## Ranked Maintenance Queue".into(), String::new()]);

    for (index, repo) in record.repos.iter().take(10).enumerate() {
        lines.push(format!("### {}. `{}`", index + 1, repo.full_name));
        lines.push(format!("- Priority: {:.1}", round1(repo.priority_score)));
        lines.push(format!("- Summary: {}", repo.summary));
        lines.push(format!(
            "- Stats: stale {} | unlabeled {} | duplicates {} | recurring clusters {} | TODO {} | FIXME {}",
            repo.stale_issues,
            repo.unlabeled_issues,
            repo.duplicate_candidates.len(),
            repo.recurring_bug_clusters.len(),
            if repo.todo_available {
                repo.todo_count.to_string()
            } else {
                "n/a".into()
            },
            if repo.fixme_available {
                repo.fixme_count.to_string()
            } else {
                "n/a".into()
            }
        ));

        if let Some(trend) = &repo.trend {
            lines.push(format!(
                "- Trend: {} (score {:+}, stale {:+}, recurring {:+})",
                trend.status, trend.priority_delta, trend.stale_delta, trend.recurring_delta
            ));
        }

        for signal in repo.signals.iter().take(3) {
            lines.push(format!("- Signal: {signal}"));
        }

        for warning in &repo.warnings {
            lines.push(format!("- Warning: {warning}"));
        }

        if let Some(factor) = repo.score_breakdown.first() {
            lines.push(format!(
                "- Strongest driver: {} (+{}) — {}",
                factor.label,
                round1(factor.impact),
                factor.detail
            ));
        }

        lines.push(String::new());
    }

    ScanReport {
        filename: format!("signalhive-report-{}.md", &record.id[..8]),
        markdown: lines.join("\n"),
        exported_at: Utc::now().to_rfc3339(),
    }
}

pub fn enrich_scan_record(record: &mut ScanRecord) -> Result<()> {
    if let Some(previous) =
        crate::db::previous_scan_for_params(&record.id, &record.created_at, &record.params)?
    {
        enrich_scan_trend(record, &previous);
    }
    Ok(())
}

pub async fn run_scan_record(
    state: &AppState,
    params: crate::models::ScanParams,
    trigger_type: &str,
    schedule_name: Option<&str>,
) -> Result<ScanRecord> {
    let params = clamp_params(params);
    let (allowlist, denylist, opt_out) = crate::db::repo_list_sets()?;
    if params.search_query.is_empty()
        && params.topics.is_empty()
        && params.languages.is_empty()
        && allowlist.is_empty()
    {
        anyhow::bail!(
            "Provide at least a search query, topic, or language, or configure an allowlist."
        );
    }

    let repos =
        crate::github::discover_repositories(&state.http, &params, &allowlist, &denylist, &opt_out)
            .await?;

    let mut drafts = Vec::new();
    let mut scan_warnings = Vec::new();
    let mut seen_scan_warnings = HashSet::new();

    for repo in repos {
        match analyze_repo_issue_draft(&state.http, &repo, &params).await {
            Ok(draft) => drafts.push(draft),
            Err(err) => {
                warn!("failed to analyze {}: {err}", repo.full_name);
                let warning = format!(
                    "SignalHive could not analyze `{}` in this scan: {}",
                    repo.full_name, err
                );
                if seen_scan_warnings.insert(warning.clone()) {
                    scan_warnings.push(warning);
                }
            }
        }
    }

    drafts.sort_by(|a, b| {
        b.issue_only_priority_score
            .partial_cmp(&a.issue_only_priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.repo.stargazers_count.cmp(&a.repo.stargazers_count))
    });

    let marker_repo_limit = marker_scan_repo_limit();
    if drafts.len() > marker_repo_limit {
        let warning = format!(
            "SignalHive only ran TODO/FIXME code search on the top {marker_repo_limit} repos in this scan to stay within GitHub search limits. Marker counts are unavailable for the rest of the queue."
        );
        if seen_scan_warnings.insert(warning.clone()) {
            scan_warnings.push(warning);
        }
    }

    let mut signals = Vec::new();
    let mut code_search_rate_limited = false;

    for (index, draft) in drafts.into_iter().enumerate() {
        let marker_counts = if index < marker_repo_limit {
            collect_marker_counts(
                &state.http,
                &draft.repo.full_name,
                &mut code_search_rate_limited,
            )
            .await
        } else {
            MarkerCounts {
                warnings: vec![
                    "SignalHive capped TODO/FIXME code search to the highest-priority repos in this scan, so the rest of the queue does not include marker counts.".into(),
                ],
                ..MarkerCounts::default()
            }
        };

        for warning in &marker_counts.warnings {
            if seen_scan_warnings.insert(warning.clone()) {
                scan_warnings.push(warning.clone());
            }
        }
        signals.push(finalize_repo_signal(draft, marker_counts));
    }

    signals.sort_by(|a, b| {
        b.priority_score
            .partial_cmp(&a.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.stars.cmp(&a.stars))
    });

    let mut record = crate::db::save_scan(
        &params,
        &signals,
        &scan_warnings,
        trigger_type,
        schedule_name,
    )?;
    enrich_scan_record(&mut record)?;
    Ok(record)
}

pub async fn run_schedule_now(state: &AppState, schedule_name: &str) -> Result<ScanRecord> {
    let schedule = crate::db::get_scan_schedule(schedule_name)?
        .ok_or_else(|| anyhow::anyhow!("Schedule not found"))?;

    let result = run_scan_record(
        state,
        schedule.params.clone(),
        "scheduled",
        Some(&schedule.name),
    )
    .await;
    match result {
        Ok(record) => {
            crate::db::record_scan_schedule_result(&schedule.name, Some(&record.id), "ok", None)?;
            Ok(record)
        }
        Err(err) => {
            crate::db::record_scan_schedule_result(
                &schedule.name,
                None,
                "error",
                Some(&err.to_string()),
            )?;
            Err(err)
        }
    }
}

pub fn start_scheduler(state: AppState) {
    tokio::spawn(async move {
        loop {
            match crate::db::claim_due_scan_schedules(4) {
                Ok(schedules) => {
                    for schedule in schedules {
                        let name = schedule.name.clone();
                        match run_scan_record(
                            &state,
                            schedule.params.clone(),
                            "scheduled",
                            Some(&name),
                        )
                        .await
                        {
                            Ok(record) => {
                                if let Err(err) = crate::db::record_scan_schedule_result(
                                    &name,
                                    Some(&record.id),
                                    "ok",
                                    None,
                                ) {
                                    warn!("failed to store schedule result for {name}: {err}");
                                }
                                info!(
                                    "SignalHive scheduled scan '{name}' completed as {}",
                                    record.id
                                );
                            }
                            Err(err) => {
                                if let Err(write_err) = crate::db::record_scan_schedule_result(
                                    &name,
                                    None,
                                    "error",
                                    Some(&err.to_string()),
                                ) {
                                    warn!("failed to store schedule error for {name}: {write_err}");
                                }
                                warn!("SignalHive scheduled scan '{name}' failed: {err}");
                            }
                        }
                    }
                }
                Err(err) => warn!("SignalHive scheduler poll failed: {err}"),
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::build_scan_report;
    use crate::models::{RepoSignal, ScanParams, ScanRecord, ScanSummary};

    fn sample_repo() -> RepoSignal {
        RepoSignal {
            full_name: "patchhive/example".into(),
            repo_url: "https://github.com/patchhive/example".into(),
            description: "example".into(),
            language: "Rust".into(),
            stars: 10,
            open_issues: 2,
            sampled_issues: 2,
            stale_issues: 0,
            unlabeled_issues: 0,
            stale_bug_issues: 0,
            stale_high_comment_issues: 0,
            duplicate_candidates: Vec::new(),
            recurring_bug_clusters: Vec::new(),
            todo_count: 1,
            fixme_count: 0,
            todo_available: true,
            fixme_available: true,
            priority_score: 9.8,
            score_breakdown: Vec::new(),
            summary: "1 TODO and 0 FIXME markers were found in code search".into(),
            signals: vec!["1 TODO and 0 FIXME markers were found in code search".into()],
            issue_examples: Vec::new(),
            warnings: Vec::new(),
            trend: None,
        }
    }

    #[test]
    fn build_scan_report_uses_clear_scope_labels_and_suppresses_zero_value_callouts() {
        let report = build_scan_report(&ScanRecord {
            id: "124d10c2-ef5b-4584-a315-f137c3237624".into(),
            created_at: "2026-04-19T18:19:55Z".into(),
            params: ScanParams {
                search_query: String::new(),
                topics: Vec::new(),
                languages: vec!["rust".into(), "typescript".into(), "python".into()],
                min_stars: 25,
                max_repos: 4,
                issues_per_repo: 30,
                stale_days: 45,
            },
            summary: ScanSummary {
                total_repos: 1,
                total_signals: 1,
                top_repo: "patchhive/example".into(),
            },
            repos: vec![sample_repo()],
            warnings: Vec::new(),
            trigger_type: "manual".into(),
            schedule_name: None,
            trend: None,
        });

        assert!(report.markdown.contains(
            "- Scope: no search query filter · no topic filter · languages `rust, typescript, python`"
        ));
        assert!(report
            .markdown
            .contains("- No stale backlog stood out in this scan."));
        assert!(report
            .markdown
            .contains("- No recurring bug cluster stood out in this scan."));
        assert!(!report.markdown.contains("*none*"));
        assert!(!report.markdown.contains("with 0 recurring clusters"));
    }
}

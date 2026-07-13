use std::collections::BTreeMap;

use axum::http::StatusCode;
use chrono::Utc;
use patchhive_product_core::repo_memory::RepoMemoryContextRequest;
use tokio::join;
use uuid::Uuid;

use crate::{
    github::{self, GitHubMergeContext},
    integrations,
    models::{
        GitHubAssessmentContext, MergeAssessment, MergeMetrics, MergeSignal,
        RepoMemoryContextPreview, ReviewBeeContext, ReviewerState, TrustGateContext,
    },
    state::AppState,
};

use super::utils::{
    actionable_text, api_error, diff_changed_paths, make_signal, mergeability_posture,
    mergeable_value, normalize_mergeable_state, plural_suffix, truncate, ApiError,
    MergeabilityPosture,
};

pub struct AssessmentRunRequest {
    pub repo: String,
    pub pr_number: i64,
    pub publish_report: bool,
    pub approval_required: bool,
    pub trigger: String,
    pub event: String,
    pub action: String,
}

pub async fn run_github_pr_assessment(
    state: &AppState,
    request: AssessmentRunRequest,
) -> Result<MergeAssessment, ApiError> {
    let context = github::fetch_merge_context(&state.http, &request.repo, request.pr_number)
        .await
        .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;
    let mut assessment = build_assessment(state, &context, request.approval_required).await;
    assessment.github = Some(GitHubAssessmentContext {
        repo: context.pr.repo.clone(),
        pr_number: context.pr.number,
        pr_title: context.pr.title.clone(),
        pr_url: context.pr.html_url.clone(),
        head_sha: context.pr.head_sha.clone(),
        head_repo: context.pr.head_repo.clone(),
        head_ref: context.pr.head_ref.clone(),
        base_ref: context.pr.base_ref.clone(),
        trigger: request.trigger,
        event: request.event,
        action: request.action,
    });
    assessment.github_report = Some(if request.publish_report {
        github::publish_assessment_outcome(&state.http, &assessment).await
    } else {
        github::preview_assessment_outcome(
            &assessment,
            "GitHub publish was skipped for this MergeKeeper run.",
        )
    });
    db::save_assessment(&assessment)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(assessment)
}

use crate::db;

pub fn approval_required_default() -> bool {
    std::env::var("MERGE_KEEPER_REQUIRE_APPROVAL")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
        .unwrap_or(true)
}

pub async fn build_assessment(
    state: &AppState,
    context: &GitHubMergeContext,
    approval_required: bool,
) -> MergeAssessment {
    let created_at = Utc::now().to_rfc3339();
    let reviewer_states = reviewer_states(context);
    let reviewer_count = reviewer_states.len() as u32;
    let (approvals, changes_requested, comment_reviews, changed_requesters) =
        current_review_counts(&reviewer_states);
    let (review_threads, open_review_threads, actionable_open_threads, thread_evidence) =
        review_thread_metrics(context);
    let commit_health = &context.commit_health;

    let metrics = MergeMetrics {
        approvals,
        changes_requested,
        comment_reviews,
        reviewer_count,
        review_threads,
        open_review_threads,
        actionable_open_threads,
        successful_checks: commit_health.successful_contexts + commit_health.successful_checks,
        pending_checks: commit_health.pending_contexts + commit_health.pending_checks,
        failing_checks: commit_health.failing_contexts + commit_health.failing_checks,
        changed_files: context.pr.changed_files,
        additions: context.pr.additions,
        deletions: context.pr.deletions,
    };

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if context.pr.state != "open" {
        blockers.push(make_signal(
            "pr-closed",
            "block",
            "PR is not open",
            format!(
                "GitHub reports this pull request as `{}` instead of `open`.",
                context.pr.state
            ),
            vec![],
        ));
    }

    if context.pr.merged {
        blockers.push(make_signal(
            "already-merged",
            "block",
            "PR is already merged",
            "This pull request is already merged, so MergeKeeper does not need to hold the line anymore.",
            vec![],
        ));
    }

    if context.pr.draft {
        blockers.push(make_signal(
            "draft-pr",
            "block",
            "PR is still draft",
            "Draft pull requests are not merge-ready by definition, even if the rest of the signals look healthy.",
            vec![],
        ));
    }

    let mergeable = mergeable_value(context.pr.mergeable);
    let mergeable_state = normalize_mergeable_state(&context.pr.mergeable_state);
    match mergeability_posture(context.pr.mergeable, &mergeable_state) {
        MergeabilityPosture::Conflict => blockers.push(make_signal(
            "merge-conflict",
            "block",
            "PR has a merge conflict",
            format!(
                "GitHub reports mergeable `{mergeable}` with state `{mergeable_state}`, so the head cannot currently merge cleanly into the base branch."
            ),
            vec![format!("mergeable={mergeable}")],
        )),
        MergeabilityPosture::PolicyHold => warnings.push(make_signal(
            "merge-policy-hold",
            "warn",
            "GitHub merge policy is holding this PR",
            format!(
                "GitHub reports mergeable `{mergeable}` with state `{mergeable_state}`. The diff can merge mechanically, but a branch-protection or repository policy condition is still holding it."
            ),
            vec![format!("mergeable={mergeable}")],
        )),
        MergeabilityPosture::Unsettled => warnings.push(make_signal(
            "merge-state-uncertain",
            "warn",
            "Mergeability is not settled yet",
            format!(
                "GitHub reports mergeable state `{mergeable_state}`, which usually means a rebase, background check, or branch-protection condition still needs to settle."
            ),
            vec![format!("mergeable={mergeable}")],
        )),
        MergeabilityPosture::Clear => {}
    }

    if metrics.failing_checks > 0 {
        blockers.push(make_signal(
            "checks-failing",
            "block",
            "Checks are failing",
            format!(
                "{} status context{} or check run{} are failing.",
                metrics.failing_checks,
                plural_suffix(metrics.failing_checks),
                plural_suffix(metrics.failing_checks)
            ),
            failing_check_evidence(context),
        ));
    } else if metrics.pending_checks > 0 {
        warnings.push(make_signal(
            "checks-pending",
            "warn",
            "Checks are still pending",
            format!(
                "{} status context{} or check run{} still need to finish before this PR looks safely mergeable.",
                metrics.pending_checks,
                plural_suffix(metrics.pending_checks),
                plural_suffix(metrics.pending_checks)
            ),
            pending_check_evidence(context),
        ));
    }

    if changes_requested > 0 {
        blockers.push(make_signal(
            "changes-requested",
            "block",
            "A reviewer is still requesting changes",
            format!(
                "{} reviewer{} currently have `CHANGES_REQUESTED` as their latest review state.",
                changes_requested,
                plural_suffix(changes_requested)
            ),
            changed_requesters,
        ));
    }

    if approval_required && approvals == 0 {
        warnings.push(make_signal(
            "no-approval",
            "warn",
            "No current approval",
            "MergeKeeper did not find an active approval in the latest reviewer state set.",
            vec![],
        ));
    }

    if actionable_open_threads > 0 {
        warnings.push(make_signal(
            "open-review-threads",
            "warn",
            "Actionable review threads are still open",
            format!(
                "{} open thread{} still look actionable enough to delay a clean merge.",
                actionable_open_threads,
                plural_suffix(actionable_open_threads)
            ),
            thread_evidence,
        ));
    } else if open_review_threads > 0 {
        warnings.push(make_signal(
            "open-threads",
            "warn",
            "There are unresolved review threads",
            format!(
                "{} review thread{} remain unresolved even though they may be informational rather than blocking.",
                open_review_threads,
                plural_suffix(open_review_threads)
            ),
            vec![],
        ));
    }

    if context.pr.changed_files >= 20 || context.pr.additions >= 900 {
        warnings.push(make_signal(
            "wide-diff",
            "warn",
            "The diff is getting wide",
            format!(
                "This PR touches {} files with +{} / -{}, which makes merge confidence harder even when individual signals look okay.",
                context.pr.changed_files, context.pr.additions, context.pr.deletions
            ),
            vec![],
        ));
    }

    let changed_paths = diff_changed_paths(&context.diff);
    let diff_summary = format!(
        "{} files, +{} / -{}",
        context.pr.changed_files, context.pr.additions, context.pr.deletions
    );
    let task_summary = format!(
        "Assess merge readiness for PR #{}: {}",
        context.pr.number, context.pr.title
    );
    let repo_memory_request = RepoMemoryContextRequest {
        repo: context.pr.repo.clone(),
        consumer: "merge-keeper".into(),
        changed_paths: changed_paths.clone(),
        task_summary: task_summary.clone(),
        diff_summary: diff_summary.clone(),
        limit: 4,
    };

    let (review_bee_result, trust_gate_result, repo_memory_result) = join!(
        integrations::fetch_review_bee_context(&state.http, &context.pr.repo, context.pr.number),
        integrations::fetch_trust_gate_context(&state.http, &context.pr.repo, context.pr.number),
        integrations::fetch_repo_memory_preview(&state.http, &repo_memory_request),
    );

    let review_bee_context = review_bee_result.ok().flatten();
    let trust_gate_context = trust_gate_result.ok().flatten();
    let repo_memory_context = repo_memory_result.ok().flatten();

    apply_review_bee_signals(&mut blockers, &mut warnings, review_bee_context.as_ref());
    apply_trust_gate_signals(&mut blockers, &mut warnings, trust_gate_context.as_ref());
    apply_repo_memory_signals(&mut warnings, repo_memory_context.as_ref());

    let readiness = if !blockers.is_empty() {
        "blocked"
    } else if !warnings.is_empty() {
        "hold"
    } else {
        "ready"
    }
    .to_string();

    let summary = build_summary(
        &readiness,
        &metrics,
        &blockers,
        &warnings,
        approval_required,
    );

    MergeAssessment {
        id: Uuid::new_v4().to_string(),
        created_at,
        repo: context.pr.repo.clone(),
        pr_number: context.pr.number,
        pr_title: context.pr.title.clone(),
        pr_url: context.pr.html_url.clone(),
        readiness,
        summary,
        approval_required,
        mergeable,
        mergeable_state,
        base_ref: context.pr.base_ref.clone(),
        head_ref: context.pr.head_ref.clone(),
        metrics,
        reviewer_states,
        blockers,
        warnings,
        review_bee: review_bee_context,
        trust_gate: trust_gate_context,
        repo_memory: repo_memory_context,
        github: None,
        github_report: None,
    }
}

pub fn reviewer_states(context: &GitHubMergeContext) -> Vec<ReviewerState> {
    let mut latest = BTreeMap::<String, ReviewerState>::new();
    for review in &context.reviews {
        let login = review.author_login.trim();
        if login.is_empty() {
            continue;
        }
        latest.insert(
            login.into(),
            ReviewerState {
                login: login.into(),
                state: review.state.clone(),
                submitted_at: review.submitted_at.clone(),
            },
        );
    }
    latest.into_values().collect()
}

pub fn current_review_counts(states: &[ReviewerState]) -> (u32, u32, u32, Vec<String>) {
    let mut approvals = 0u32;
    let mut changes_requested = 0u32;
    let mut comment_reviews = 0u32;
    let mut changed_requesters = Vec::new();

    for state in states {
        match state.state.as_str() {
            "APPROVED" => approvals += 1,
            "CHANGES_REQUESTED" => {
                changes_requested += 1;
                changed_requesters.push(state.login.clone());
            }
            "COMMENTED" => comment_reviews += 1,
            _ => {}
        }
    }

    (
        approvals,
        changes_requested,
        comment_reviews,
        changed_requesters,
    )
}

pub fn review_thread_metrics(context: &GitHubMergeContext) -> (u32, u32, u32, Vec<String>) {
    let mut open_threads = 0u32;
    let mut actionable_open_threads = 0u32;
    let mut evidence = Vec::new();

    for thread in &context.threads {
        if thread.is_resolved {
            continue;
        }
        open_threads += 1;
        let actionable = thread
            .comments
            .iter()
            .any(|comment| actionable_text(&comment.body));
        if actionable {
            actionable_open_threads += 1;
            if evidence.len() < 4 {
                let path = if thread.path.trim().is_empty() {
                    "general".to_string()
                } else {
                    thread.path.clone()
                };
                let excerpt = thread
                    .comments
                    .iter()
                    .find(|comment| actionable_text(&comment.body))
                    .map(|comment| truncate(&comment.body, 120))
                    .unwrap_or_else(|| "Open review thread remains active.".into());
                evidence.push(format!("{path}: {excerpt}"));
            }
        }
    }

    (
        context.threads.len() as u32,
        open_threads,
        actionable_open_threads,
        evidence,
    )
}

pub fn apply_review_bee_signals(
    blockers: &mut Vec<MergeSignal>,
    warnings: &mut Vec<MergeSignal>,
    context: Option<&ReviewBeeContext>,
) {
    let Some(context) = context else {
        return;
    };

    if context.open_items == 0 {
        return;
    }

    let evidence = context
        .top_items
        .iter()
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    if context.status == "attention" && context.open_items >= 3 {
        blockers.push(make_signal(
            "review-bee-pressure",
            "block",
            "ReviewBee still sees concentrated review churn",
            format!(
                "ReviewBee found {} open checklist item{} across {} actionable thread{}, which usually means the PR still needs real follow-up before merge.",
                context.open_items,
                plural_suffix(context.open_items),
                context.actionable_threads,
                plural_suffix(context.actionable_threads),
            ),
            evidence,
        ));
    } else {
        warnings.push(make_signal(
            "review-bee-follow-up",
            "warn",
            "ReviewBee still sees unresolved follow-up",
            format!(
                "ReviewBee found {} open checklist item{} across {} actionable thread{}.",
                context.open_items,
                plural_suffix(context.open_items),
                context.actionable_threads,
                plural_suffix(context.actionable_threads),
            ),
            evidence,
        ));
    }
}

pub fn apply_trust_gate_signals(
    blockers: &mut Vec<MergeSignal>,
    warnings: &mut Vec<MergeSignal>,
    context: Option<&TrustGateContext>,
) {
    let Some(context) = context else {
        return;
    };

    let evidence = context
        .top_findings
        .iter()
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    match context.recommendation.as_str() {
        "block" => blockers.push(make_signal(
            "trust-gate-block",
            "block",
            "TrustGate would block this PR",
            format!(
                "{} Risk score: {}. Blocking findings: {}.",
                context.summary, context.risk_score, context.blocked_findings
            ),
            evidence,
        )),
        "warn" => warnings.push(make_signal(
            "trust-gate-warn",
            "warn",
            "TrustGate wants a human review pass",
            format!(
                "{} Risk score: {}. Warning findings: {}.",
                context.summary, context.risk_score, context.warning_findings
            ),
            evidence,
        )),
        _ => {}
    }
}

pub fn apply_repo_memory_signals(
    warnings: &mut Vec<MergeSignal>,
    context: Option<&RepoMemoryContextPreview>,
) {
    let Some(context) = context else {
        return;
    };

    if !context.failguard_warnings.is_empty() {
        warnings.push(make_signal(
            "failguard-guardrail",
            "warn",
            "A promoted FailGuard guardrail matched this PR",
            "FailGuard compiled a human-reviewed failure lesson into merge warning evidence. Confirm the prevention condition before merging.",
            context.failguard_warnings.clone(),
        ));
        return;
    }

    if context.pinned_entries == 0 && context.policy_entries < 2 {
        return;
    }

    warnings.push(make_signal(
        "repo-memory-policy",
        "warn",
        "RepoMemory found repo-specific merge expectations",
        format!(
            "{} Policy entries: {}. Pinned entries: {}.",
            context.summary, context.policy_entries, context.pinned_entries
        ),
        context.top_entries.iter().take(4).cloned().collect(),
    ));
}

fn failing_check_evidence(context: &GitHubMergeContext) -> Vec<String> {
    let mut items = Vec::new();
    for status in &context.commit_health.statuses {
        if matches!(status.state.as_str(), "failure" | "error") {
            items.push(format!(
                "{}: {}",
                status.context,
                if status.description.trim().is_empty() {
                    status.state.clone()
                } else {
                    status.description.clone()
                }
            ));
        }
    }
    for run in &context.commit_health.check_runs {
        if matches!(
            run.conclusion.as_str(),
            "failure" | "timed_out" | "cancelled" | "action_required" | "startup_failure" | "stale"
        ) {
            items.push(format!("{}: {}", run.name, run.conclusion));
        }
    }
    items.into_iter().take(6).collect()
}

fn pending_check_evidence(context: &GitHubMergeContext) -> Vec<String> {
    let mut items = Vec::new();
    for status in &context.commit_health.statuses {
        if status.state == "pending" {
            items.push(status.context.clone());
        }
    }
    for run in &context.commit_health.check_runs {
        if run.status != "completed" {
            items.push(run.name.clone());
        }
    }
    items.into_iter().take(6).collect()
}

fn build_summary(
    readiness: &str,
    metrics: &MergeMetrics,
    blockers: &[MergeSignal],
    warnings: &[MergeSignal],
    approval_required: bool,
) -> String {
    match readiness {
        "ready" => {
            let approval_text = if approval_required {
                "approvals are in place"
            } else if metrics.approvals > 0 {
                "approval is present, though not required for this run"
            } else {
                "approval requirement is disabled for this run"
            };
            format!(
                "This PR looks merge-ready: {approval_text}, no active changes-requested state remains, review pressure is quiet, and the current check picture is green."
            )
        },
        "blocked" => format!(
            "This PR is blocked right now. Biggest blockers: {}. Snapshot: {} approval{}, {} failing check{}, {} active review thread{}.",
            blockers
                .iter()
                .take(2)
                .map(|signal| signal.label.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            metrics.approvals,
            plural_suffix(metrics.approvals),
            metrics.failing_checks,
            plural_suffix(metrics.failing_checks),
            metrics.actionable_open_threads,
            plural_suffix(metrics.actionable_open_threads),
        ),
        _ => format!(
            "This PR is on hold rather than ready. Biggest reasons: {}. Snapshot: {} approval{}, {} pending check{}, {} open review thread{}.",
            warnings
                .iter()
                .take(2)
                .map(|signal| signal.label.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            metrics.approvals,
            plural_suffix(metrics.approvals),
            metrics.pending_checks,
            plural_suffix(metrics.pending_checks),
            metrics.actionable_open_threads,
            plural_suffix(metrics.actionable_open_threads),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::build_summary;
    use crate::models::MergeMetrics;

    #[test]
    fn ready_summary_names_optional_approval_policy() {
        let summary = build_summary("ready", &MergeMetrics::default(), &[], &[], false);
        assert!(summary.contains("approval requirement is disabled"));
    }
}

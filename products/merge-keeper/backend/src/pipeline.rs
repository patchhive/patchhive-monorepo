use std::collections::BTreeMap;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use patchhive_github_pr::verify_github_webhook_signature;
use patchhive_product_core::contract;
use patchhive_product_core::repo_memory::RepoMemoryContextRequest;
use patchhive_product_core::startup::count_errors;
use serde_json::{json, Value};
use tokio::join;
use uuid::Uuid;

use crate::{
    auth::{
        auth_enabled, generate_and_save_key, generate_and_save_service_token,
        service_auth_enabled, service_token_generation_allowed, verify_token,
    },
    db, github,
    github::GitHubMergeContext,
    integrations,
    models::{
        AssessmentRequest, GitHubAssessmentContext, HistoryItem, MergeAssessment, MergeMetrics,
        MergeSignal, OverviewPayload, RepoMemoryContextPreview, ReviewBeeContext, ReviewerState,
        TrustGateContext,
    },
    state::AppState,
    STARTUP_CHECKS,
};

type ApiError = (StatusCode, Json<serde_json::Value>);
type JsonResult<T> = Result<Json<T>, ApiError>;

#[derive(serde::Deserialize)]
pub struct LoginBody {
    api_key: String,
}

pub async fn capabilities() -> Json<contract::ProductCapabilities> {
    Json(contract::capabilities(
        "merge-keeper",
        "MergeKeeper",
        vec![
            contract::action(
                "assess_github_pr",
                "Assess PR readiness",
                "POST",
                "/assess/github/pr",
                "Evaluate whether a GitHub pull request is merge-ready, blocked, or on hold.",
                true,
            ),
            contract::action(
                "github_webhook",
                "Receive GitHub webhook",
                "POST",
                "/webhooks/github",
                "Process a signed GitHub pull request webhook for readiness updates.",
                true,
            ),
        ],
        vec![
            contract::link("overview", "Overview", "/overview"),
            contract::link("history", "History", "/history"),
        ],
    ))
}

pub async fn runs() -> Json<contract::ProductRunsResponse> {
    Json(contract::runs_from_history("merge-keeper", db::history(30)))
}

pub async fn auth_status() -> Json<serde_json::Value> {
    Json(crate::auth::auth_status_payload())
}

pub async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth_enabled() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if !verify_token(&body.api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(
        json!({"ok": true, "auth_enabled": true, "auth_configured": true}),
    ))
}

pub async fn gen_key(
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if auth_enabled() {
        return Err(patchhive_product_core::auth::auth_already_configured_error());
    }
    if !crate::auth::bootstrap_request_allowed(&headers) {
        return Err(patchhive_product_core::auth::bootstrap_localhost_required_error());
    }
    let key = generate_and_save_key()
        .map_err(|err| patchhive_product_core::auth::key_generation_failed_error(&err))?;
    Ok(Json(
        json!({"api_key": key, "message": "Store this — it won't be shown again"}),
    ))
}

pub async fn gen_service_token(
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if service_auth_enabled() {
        return Err(patchhive_product_core::auth::service_auth_already_configured_error());
    }
    if !service_token_generation_allowed(&headers) {
        return Err(patchhive_product_core::auth::service_token_generation_forbidden_error());
    }
    let token = generate_and_save_service_token()
        .map_err(|err| patchhive_product_core::auth::service_token_generation_failed_error(&err))?;
    Ok(Json(json!({
        "service_token": token,
        "message": "Store this for HiveCore or other PatchHive service callers — it won't be shown again"
    })))
}

pub async fn health() -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();
    let counts = db::overview_counts();

    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "MergeKeeper by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "github_ready": github::github_token_configured(),
        "assessment_count": counts.runs,
        "repo_count": counts.repos,
        "ready_count": counts.ready_runs,
        "hold_count": counts.hold_runs,
        "blocked_count": counts.blocked_runs,
        "mode": "github-merge-readiness",
        "github": {
            "token_configured": github::github_token_configured(),
            "webhook_secret_configured": github::webhook_secret_configured(),
            "public_url_configured": github::public_url_configured(),
            "report_publish_ready": github::github_token_configured(),
        },
        "integrations": {
            "review_bee_configured": integrations::review_bee_configured(),
            "trust_gate_configured": integrations::trust_gate_configured(),
            "repo_memory_configured": integrations::repo_memory_configured(),
        }
    }))
}

pub async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

pub async fn overview() -> Json<OverviewPayload> {
    Json(db::overview())
}

pub async fn history() -> Json<Vec<HistoryItem>> {
    Json(db::history(30))
}

pub async fn history_detail(Path(id): Path<String>) -> JsonResult<MergeAssessment> {
    db::get_assessment(&id)
        .map(Json)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "MergeKeeper run not found"))
}

pub async fn assess_github_pr(
    State(state): State<AppState>,
    Json(request): Json<AssessmentRequest>,
) -> JsonResult<MergeAssessment> {
    let repo = request.repo.trim();
    if !valid_repo(repo) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Repository must be in owner/name format.",
        ));
    }
    if request.pr_number <= 0 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Pull request number must be greater than zero.",
        ));
    }

    let assessment = run_github_pr_assessment(
        &state,
        repo.to_string(),
        request.pr_number,
        request.publish_report,
        "manual_pr_lookup".into(),
        "pull_request".into(),
        "manual".into(),
    )
    .await?;

    Ok(Json(assessment))
}

fn api_error(status: StatusCode, error: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": error.into() })))
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

fn verify_webhook_signature(headers: &HeaderMap, body: &[u8]) -> Result<(), ApiError> {
    let Some(secret) = github::webhook_secret() else {
        return Err(api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Configure MERGE_KEEPER_GITHUB_WEBHOOK_SECRET before enabling the MergeKeeper GitHub webhook.",
        ));
    };

    verify_github_webhook_signature(headers, body, &secret).map_err(|err| {
        api_error(
            StatusCode::UNAUTHORIZED,
            format!("GitHub webhook signature verification failed: {err}"),
        )
    })
}

fn supported_webhook_action(event: &str, action: &str) -> bool {
    match event {
        "pull_request" => matches!(
            action,
            "opened" | "reopened" | "synchronize" | "ready_for_review" | "edited" | "closed"
        ),
        "pull_request_review" => matches!(action, "submitted" | "edited" | "dismissed"),
        "pull_request_review_comment" => matches!(action, "created" | "edited" | "deleted"),
        "pull_request_review_thread" => matches!(action, "resolved" | "unresolved"),
        "check_run" => matches!(action, "created" | "completed" | "rerequested"),
        "check_suite" => matches!(action, "completed" | "rerequested"),
        _ => false,
    }
}

fn extract_webhook_target(event: &str, payload: &Value) -> Option<(String, i64)> {
    let repo = payload["repository"]["full_name"].as_str()?.to_string();
    let pr_number = match event {
        "pull_request"
        | "pull_request_review"
        | "pull_request_review_comment"
        | "pull_request_review_thread" => payload["pull_request"]["number"].as_i64()?,
        "check_run" => payload["check_run"]["pull_requests"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item["number"].as_i64())?,
        "check_suite" => payload["check_suite"]["pull_requests"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item["number"].as_i64())?,
        _ => return None,
    };
    Some((repo, pr_number))
}

async fn run_github_pr_assessment(
    state: &AppState,
    repo: String,
    pr_number: i64,
    publish_report: bool,
    trigger: String,
    event: String,
    action: String,
) -> Result<MergeAssessment, ApiError> {
    let context = github::fetch_merge_context(&state.http, &repo, pr_number)
        .await
        .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;
    let mut assessment = build_assessment(state, &context).await;
    assessment.github = Some(GitHubAssessmentContext {
        repo: context.pr.repo.clone(),
        pr_number: context.pr.number,
        pr_title: context.pr.title.clone(),
        pr_url: context.pr.html_url.clone(),
        head_sha: context.pr.head_sha.clone(),
        head_repo: context.pr.head_repo.clone(),
        head_ref: context.pr.head_ref.clone(),
        base_ref: context.pr.base_ref.clone(),
        trigger,
        event,
        action,
    });
    assessment.github_report = Some(if publish_report {
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

pub async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> JsonResult<serde_json::Value> {
    verify_webhook_signature(&headers, &body)?;

    let event = headers
        .get("X-GitHub-Event")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let payload: Value = serde_json::from_slice(&body).map_err(|_| {
        api_error(
            StatusCode::BAD_REQUEST,
            "Could not decode GitHub webhook payload.",
        )
    })?;
    let action = payload["action"].as_str().unwrap_or("").to_string();

    if !supported_webhook_action(&event, &action) {
        return Ok(Json(json!({
            "triggered": false,
            "event": event,
            "action": action,
            "reason": "This GitHub event does not trigger an automatic MergeKeeper refresh.",
        })));
    }

    let Some((repo, pr_number)) = extract_webhook_target(&event, &payload) else {
        return Ok(Json(json!({
            "triggered": false,
            "event": event,
            "action": action,
            "reason": "This webhook did not include an associated pull request target MergeKeeper could refresh.",
        })));
    };

    let assessment = run_github_pr_assessment(
        &state,
        repo,
        pr_number,
        true,
        "github_webhook".into(),
        event.clone(),
        action.clone(),
    )
    .await?;

    Ok(Json(json!({
        "triggered": true,
        "event": event,
        "action": action,
        "readiness": assessment.readiness,
        "assessment": assessment,
    })))
}

fn actionable_text(text: &str) -> bool {
    let compact = collapse_whitespace(text);
    if compact.len() < 10 {
        return false;
    }
    let lower = compact.to_ascii_lowercase();
    let request_terms = [
        "please",
        "need",
        "needs",
        "should",
        "must",
        "can you",
        "could you",
        "would you",
        "consider",
        "instead",
        "avoid",
        "prefer",
        "use ",
        "remove",
        "rename",
        "handle",
        "update",
        "add ",
        "include",
        "cover",
        "fix",
        "missing",
        "nit:",
        "nit ",
    ];
    let praise_terms = [
        "lgtm",
        "looks good",
        "nice work",
        "great work",
        "thanks",
        "thank you",
    ];

    if contains_any(&lower, &request_terms) {
        return true;
    }

    if lower.contains('?')
        && contains_any(
            &lower,
            &[
                "can",
                "could",
                "would",
                "should",
                "why",
                "what about",
                "do we",
            ],
        )
    {
        return true;
    }

    !contains_any(&lower, &praise_terms) && lower.split_whitespace().count() >= 6
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn truncate(value: &str, limit: usize) -> String {
    let compact = collapse_whitespace(value);
    if compact.chars().count() <= limit {
        compact
    } else {
        compact
            .chars()
            .take(limit.saturating_sub(1))
            .collect::<String>()
            + "…"
    }
}

fn reviewer_states(context: &GitHubMergeContext) -> Vec<ReviewerState> {
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

fn current_review_counts(states: &[ReviewerState]) -> (u32, u32, u32, Vec<String>) {
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

fn review_thread_metrics(context: &GitHubMergeContext) -> (u32, u32, u32, Vec<String>) {
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

fn mergeable_value(value: Option<bool>) -> String {
    match value {
        Some(true) => "yes".into(),
        Some(false) => "no".into(),
        None => "unknown".into(),
    }
}

fn normalize_mergeable_state(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "unknown".into()
    } else {
        trimmed.into()
    }
}

fn make_signal(
    key: &str,
    severity: &str,
    label: &str,
    detail: impl Into<String>,
    evidence: Vec<String>,
) -> MergeSignal {
    MergeSignal {
        key: key.into(),
        severity: severity.into(),
        label: label.into(),
        detail: detail.into(),
        evidence,
    }
}

async fn build_assessment(state: &AppState, context: &GitHubMergeContext) -> MergeAssessment {
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
    if context.pr.mergeable == Some(false)
        || matches!(mergeable_state.as_str(), "dirty" | "blocked")
    {
        blockers.push(make_signal(
            "merge-conflict",
            "block",
            "GitHub says this PR is not mergeable",
            format!(
                "Mergeable state is `{mergeable_state}` and GitHub does not currently consider the PR safely mergeable."
            ),
            vec![format!("mergeable={mergeable}")],
        ));
    } else if matches!(
        mergeable_state.as_str(),
        "unknown" | "unstable" | "behind" | "has_hooks"
    ) {
        warnings.push(make_signal(
            "merge-state-uncertain",
            "warn",
            "Mergeability is not settled yet",
            format!(
                "GitHub reports mergeable state `{mergeable_state}`, which usually means a rebase, background check, or branch-protection condition still needs to settle."
            ),
            vec![format!("mergeable={mergeable}")],
        ));
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

    if approvals == 0 {
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

    let summary = build_summary(&readiness, &metrics, &blockers, &warnings);

    MergeAssessment {
        id: Uuid::new_v4().to_string(),
        created_at,
        repo: context.pr.repo.clone(),
        pr_number: context.pr.number,
        pr_title: context.pr.title.clone(),
        pr_url: context.pr.html_url.clone(),
        readiness,
        summary,
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

fn diff_changed_paths(diff: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in diff.lines() {
        if line.starts_with("+++ b/") {
            let path = line.trim_start_matches("+++ b/").trim();
            if !path.is_empty() && !paths.iter().any(|existing| existing == path) {
                paths.push(path.to_string());
            }
        }
    }
    paths
}

fn apply_review_bee_signals(
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

fn apply_trust_gate_signals(
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

fn apply_repo_memory_signals(
    warnings: &mut Vec<MergeSignal>,
    context: Option<&RepoMemoryContextPreview>,
) {
    let Some(context) = context else {
        return;
    };

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
) -> String {
    match readiness {
        "ready" => format!(
            "This PR looks merge-ready: approvals are in place, no active changes-requested state remains, review pressure is quiet, and the current check picture is green."
        ),
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

fn plural_suffix(count: u32) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actionable_text_ignores_pure_praise() {
        assert!(!actionable_text("LGTM, nice work."));
        assert!(actionable_text(
            "Could you add a test for the edge case here?"
        ));
    }

    #[test]
    fn current_review_counts_use_latest_state() {
        let states = vec![
            ReviewerState {
                login: "sam".into(),
                state: "APPROVED".into(),
                submitted_at: "1".into(),
            },
            ReviewerState {
                login: "alex".into(),
                state: "CHANGES_REQUESTED".into(),
                submitted_at: "1".into(),
            },
            ReviewerState {
                login: "lee".into(),
                state: "COMMENTED".into(),
                submitted_at: "1".into(),
            },
        ];

        let (approvals, changes_requested, comment_reviews, requesters) =
            current_review_counts(&states);
        assert_eq!(approvals, 1);
        assert_eq!(changes_requested, 1);
        assert_eq!(comment_reviews, 1);
        assert_eq!(requesters, vec!["alex".to_string()]);
    }

    #[test]
    fn diff_changed_paths_collects_unique_paths() {
        let diff = r#"
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@
+pub fn next() {}
diff --git a/tests/lib.test.rs b/tests/lib.test.rs
--- a/tests/lib.test.rs
+++ b/tests/lib.test.rs
@@
+it("works", () => {})
+++ b/src/lib.rs
"#;

        assert_eq!(
            diff_changed_paths(diff),
            vec!["src/lib.rs".to_string(), "tests/lib.test.rs".to_string()]
        );
    }

    #[test]
    fn trust_gate_block_turns_into_blocker() {
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();

        apply_trust_gate_signals(
            &mut blockers,
            &mut warnings,
            Some(&TrustGateContext {
                recommendation: "block".into(),
                summary: "High-risk change.".into(),
                risk_score: 87,
                blocked_findings: 2,
                warning_findings: 0,
                top_findings: vec!["workflow [block]: touches CI".into()],
            }),
        );

        assert_eq!(blockers.len(), 1);
        assert!(warnings.is_empty());
        assert_eq!(blockers[0].key, "trust-gate-block");
    }

    #[test]
    fn repo_memory_only_warns_on_stronger_expectations() {
        let mut warnings = Vec::new();
        apply_repo_memory_signals(
            &mut warnings,
            Some(&RepoMemoryContextPreview {
                summary: "One soft hint.".into(),
                policy_entries: 1,
                pinned_entries: 0,
                top_entries: vec!["tests: reviewers usually ask for coverage here".into()],
                ..RepoMemoryContextPreview::default()
            }),
        );
        assert!(warnings.is_empty());

        apply_repo_memory_signals(
            &mut warnings,
            Some(&RepoMemoryContextPreview {
                summary: "Durable repo expectations.".into(),
                policy_entries: 2,
                pinned_entries: 0,
                top_entries: vec!["auth: require regression coverage".into()],
                ..RepoMemoryContextPreview::default()
            }),
        );
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].key, "repo-memory-policy");
    }
}

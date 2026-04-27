// pipeline.rs - Main route handlers and shared helpers for RepoMemory
// Refactored from 2568 lines into modular structure

use std::collections::HashMap;

use anyhow::Result;
use axum::{
 extract::{Path, Query, State},
 http::{HeaderMap, StatusCode},
 Json,
};
use patchhive_product_core::contract;
use serde_json::json;
use uuid::Uuid;

use crate::{
 auth::{
  auth_enabled, generate_and_save_key, generate_and_save_service_token,
  rotate_and_save_service_token, service_auth_enabled, service_token_generation_allowed,
  service_token_rotation_allowed, verify_token,
 },
 db, github,
 models::{
  stable_memory_ref, ContextRequest, ContextResponse, GitHubPullFile, GitHubPullRequest, GitHubReview,
  GitHubReviewComment, HistoryItem, IngestParams, IngestRecord, IngestSummary, KnownRepo,
  MemoryCurationUpdate, MemoryEntry, MemoryEvidence, OverviewPayload,
  RunDiffResponse,
 },
 state::AppState,
 STARTUP_CHECKS,
};

// Type aliases used across modules
pub type JsonError = (StatusCode, Json<serde_json::Value>);
pub type JsonResult<T> = Result<Json<T>, JsonError>;

// Struct definitions used by submodules
#[derive(Clone)]
pub struct PullBundle {
    pub pr: GitHubPullRequest,
    pub reviews: Vec<GitHubReview>,
    pub comments: Vec<GitHubReviewComment>,
    pub files: Vec<GitHubPullFile>,
}

#[derive(Default)]
pub struct SignalBucket {
    pub frequency: u32,
    pub evidence: Vec<MemoryEvidence>,
}

#[derive(Default)]
pub struct ReviewerProfileBucket {
    pub total_feedback: u32,
    pub category_counts: HashMap<&'static str, u32>,
    pub path_counts: HashMap<String, u32>,
    pub evidence: Vec<MemoryEvidence>,
}

#[derive(Default)]
pub struct MaintainerProfileBucket {
    pub merged_prs: u32,
    pub source_prs: u32,
    pub source_with_tests: u32,
    pub path_counts: HashMap<String, u32>,
    pub evidence: Vec<MemoryEvidence>,
}

// Module declarations
mod failguard;
mod memory_run;
mod context;
mod diff;
mod utils;

// Re-export public functions from submodules for backward compatibility
// This allows main.rs to still use `pipeline::capture_failguard_lesson` etc.
pub use failguard::{
    capture_failguard_lesson, failguard_candidates, create_failguard_candidate,
    promote_failguard_candidate, dismiss_failguard_candidate,
    build_failguard_lesson_run, build_failguard_candidate, candidate_to_lesson_request,
};
pub use memory_run::{
    build_memory_run, truncate,
};
pub use context::{
    rank_context_entries, disposition_rank,
};
pub use diff::build_run_diff;
pub use utils::{
 normalize_consumer, normalize_disposition,
 path_bucket,
 internal_error, internal_from_anyhow,
 upstream_error, bad_request, not_found, valid_repo, STOPWORDS,
};

// Query structs
#[derive(serde::Deserialize)]
pub struct MemoryQuery {
    repo: Option<String>,
    kind: Option<String>,
    search: Option<String>,
    run_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct HistoryQuery {
    repo: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct LoginBody {
    api_key: String,
}

// Capabilities and health endpoints
pub async fn capabilities() -> Json<contract::ProductCapabilities> {
    Json(contract::capabilities(
        "repo-memory",
        "RepoMemory",
        vec![
            contract::action(
                "ingest",
                "Ingest repo history",
                "POST",
                "/ingest",
                "Build durable repo memory from GitHub history and review feedback.",
                true,
            ),
            contract::action(
                "context",
                "Fetch repo context",
                "POST",
                "/context",
                "Return reusable repo-specific context for another PatchHive product or agent.",
                false,
            ),
            contract::action(
                "capture_failguard_lesson",
                "Capture FailGuard lesson",
                "POST",
                "/failguard/lessons",
                "Turn a painful outcome into a curated failure-pattern policy memory.",
                true,
            ),
            contract::action(
                "suggest_failguard_candidate",
                "Suggest FailGuard candidate",
                "POST",
                "/failguard/candidates",
                "Queue a bad outcome for operator review before it becomes durable memory.",
                false,
            ),
        ],
        vec![
            contract::link("overview", "Overview", "/overview"),
            contract::link("history", "History", "/history"),
            contract::link("memories", "Memories", "/memories"),
            contract::link(
                "failguard-candidates",
                "FailGuard candidates",
                "/failguard/candidates",
            ),
        ],
    ))
}

pub async fn runs() -> Json<contract::ProductRunsResponse> {
    let items = db::list_history(None).unwrap_or_default();
    Json(contract::runs_from_history("repo-memory", items))
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

pub async fn rotate_service_token(
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if !service_auth_enabled() {
        return Err(patchhive_product_core::auth::service_auth_not_configured_error());
    }
    if !service_token_rotation_allowed(&headers) {
        return Err(patchhive_product_core::auth::service_token_rotation_forbidden_error());
    }
    let token = rotate_and_save_service_token()
        .map_err(|err| patchhive_product_core::auth::service_token_rotation_failed_error(&err))?;
    Ok(Json(json!({
        "service_token": token,
        "message": "Store this replacement service token for HiveCore or other PatchHive service callers — it won't be shown again"
    })))
}

pub async fn health(State(_state): State<AppState>) -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| patchhive_product_core::startup::count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();

    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "RepoMemory by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "counts": db::overview_counts(),
        "github_ready": std::env::var("BOT_GITHUB_TOKEN").is_ok() || std::env::var("GITHUB_TOKEN").is_ok(),
        "memory_loop": "merged-prs + review feedback + closed issues",
    }))
}

pub async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

pub async fn overview() -> JsonResult<OverviewPayload> {
    let payload = OverviewPayload {
        product: "RepoMemory by PatchHive".into(),
        tagline: "Turn merged history and review pain into durable repo memory.".into(),
        counts: db::overview_counts(),
        repos: db::list_known_repos().map_err(internal_error)?,
        featured_memories: db::featured_memories(8).map_err(internal_error)?,
    };

    Ok(Json(payload))
}

pub async fn known_repos() -> JsonResult<serde_json::Value> {
    let repos: Vec<KnownRepo> = db::list_known_repos().map_err(internal_error)?;
    Ok(Json(json!({ "repos": repos })))
}

pub async fn memories(Query(query): Query<MemoryQuery>) -> JsonResult<serde_json::Value> {
    let memories = db::list_memories(
        query.repo.as_deref(),
        query.kind.as_deref(),
        query.search.as_deref(),
        query.run_id.as_deref(),
    )
    .map_err(internal_error)?;

    Ok(Json(json!({ "memories": memories })))
}

pub async fn curate_memory(
    Json(mut update): Json<MemoryCurationUpdate>,
) -> JsonResult<serde_json::Value> {
    update.repo = update.repo.trim().to_string();
    update.memory_ref = update.memory_ref.trim().to_string();
    update.disposition = normalize_disposition(&update.disposition).to_string();

    if !valid_repo(&update.repo) {
        return Err(bad_request(
            "RepoMemory expects repos in owner/repo format.",
        ));
    }
    if update.memory_ref.is_empty() {
        return Err(bad_request(
            "RepoMemory needs a stable memory_ref to save curation.",
        ));
    }

    db::save_memory_curation(
        &update.repo,
        &update.memory_ref,
        &update.disposition,
        update.pinned,
    )
    .map_err(internal_error)?;

    Ok(Json(json!({
        "ok": true,
        "repo": update.repo,
        "memory_ref": update.memory_ref,
        "disposition": update.disposition,
        "pinned": update.pinned,
    })))
}

pub async fn context(Json(request): Json<ContextRequest>) -> JsonResult<ContextResponse> {
    let repo = request.repo.trim().to_string();
    if !valid_repo(&repo) {
        return Err(bad_request(
            "RepoMemory expects repos in owner/repo format.",
        ));
    }

    let latest = db::list_history(Some(&repo))
        .map_err(internal_error)?
        .into_iter()
        .next()
        .ok_or_else(|| not_found("RepoMemory has no ingested history for that repo yet."))?;

    let run = db::get_history(&latest.id)
        .map_err(internal_error)?
        .ok_or_else(|| not_found("RepoMemory run not found."))?;

    let consumer = normalize_consumer(&request.consumer);
    let entries = rank_context_entries(
        &run.entries,
        &consumer,
        &request.changed_paths,
        &request.task_summary,
        &request.diff_summary,
        request.limit.max(1) as usize,
    );

    let policy_count = entries
        .iter()
        .filter(|entry| entry.disposition == "policy")
        .count();
    let pinned_count = entries.iter().filter(|entry| entry.pinned).count();
    let summary = if entries.is_empty() {
        format!(
            "RepoMemory found no especially relevant memories in the latest run for {repo}, so consumers should fall back to the full prompt pack."
        )
    } else {
        format!(
            "RepoMemory selected {} relevant memories from the latest run for {repo}{}{}.",
            entries.len(),
            if policy_count > 0 {
                format!(", including {policy_count} policy memories")
            } else {
                String::new()
            },
            if pinned_count > 0 {
                format!(", with {pinned_count} pinned")
            } else {
                String::new()
            },
        )
    };

    Ok(Json(ContextResponse {
        repo,
        consumer,
        run_id: run.id,
        created_at: run.created_at,
        summary,
        prompt_lines: entries
            .iter()
            .map(|entry| entry.prompt_line.clone())
            .collect(),
        entries,
    }))
}

pub async fn history(Query(query): Query<HistoryQuery>) -> JsonResult<serde_json::Value> {
    let items: Vec<HistoryItem> =
        db::list_history(query.repo.as_deref()).map_err(internal_error)?;
    Ok(Json(json!({ "history": items })))
}

pub async fn history_detail(Path(id): Path<String>) -> JsonResult<IngestRecord> {
    match db::get_history(&id).map_err(internal_error)? {
        Some(run) => Ok(Json(run)),
        None => Err(not_found("RepoMemory run not found.")),
    }
}

pub async fn history_diff(Path(id): Path<String>) -> JsonResult<RunDiffResponse> {
    let current = db::get_history(&id)
        .map_err(internal_error)?
        .ok_or_else(|| not_found("RepoMemory run not found."))?;

    let previous = db::list_history(Some(&current.repo))
        .map_err(internal_error)?
        .into_iter()
        .skip_while(|item| item.id != current.id)
        .nth(1)
        .map(|item| item.id)
        .map(|previous_id| {
            db::get_history(&previous_id)
                .map_err(internal_error)?
                .ok_or_else(|| not_found("RepoMemory previous run not found."))
        })
        .transpose()?;

    Ok(Json(build_run_diff(current, previous)))
}

pub async fn prompt_pack(Path(id): Path<String>) -> JsonResult<serde_json::Value> {
    match db::get_history(&id).map_err(internal_error)? {
        Some(run) => Ok(Json(json!({
            "id": run.id,
            "repo": run.repo,
            "prompt_pack": run.prompt_pack,
        }))),
        None => Err(not_found("RepoMemory run not found.")),
    }
}

pub async fn ingest(
    State(state): State<AppState>,
    Json(params): Json<IngestParams>,
) -> JsonResult<IngestRecord> {
    let params = params.normalized();
    if !valid_repo(&params.repo) {
        return Err(bad_request(
            "RepoMemory expects repos in owner/repo format.",
        ));
    }

    let pulls = github::fetch_merged_pull_requests(
        &state.http,
        &params.repo,
        params.merged_pr_limit,
        params.since_days,
    )
    .await
    .map_err(upstream_error)?;

    let mut bundles = Vec::new();
    for pr in pulls {
        let reviews = github::fetch_pr_reviews(&state.http, &params.repo, pr.number)
            .await
            .unwrap_or_default();
        let comments = github::fetch_pr_review_comments(&state.http, &params.repo, pr.number)
            .await
            .unwrap_or_default();
        let files = github::fetch_pr_files(&state.http, &params.repo, pr.number)
            .await
            .unwrap_or_default();
        bundles.push(PullBundle {
            pr,
            reviews,
            comments,
            files,
        });
    }

    let issues = github::fetch_closed_issues(
        &state.http,
        &params.repo,
        params.issue_limit,
        params.since_days,
    )
    .await
    .map_err(upstream_error)?;

    let run = build_memory_run(params, bundles, issues).map_err(internal_from_anyhow)?;
    db::save_run(&run).map_err(internal_error)?;
    Ok(Json(run))
}

// Shared helper functions used by submodules

pub fn build_entry(
    run_id: &str,
    repo: &str,
    kind: &str,
    title: impl Into<String>,
    detail: impl Into<String>,
    prompt_line: impl Into<String>,
    frequency: u32,
    tags: Vec<&str>,
    evidence: Vec<MemoryEvidence>,
    created_at: &str,
) -> MemoryEntry {
    let title = title.into();
    let detail = detail.into();
    let prompt_line = prompt_line.into();
    MemoryEntry {
        id: Uuid::new_v4().to_string(),
        memory_ref: stable_memory_ref(repo, kind, &title),
        run_id: run_id.to_string(),
        repo: repo.to_string(),
        kind: kind.to_string(),
        title,
        detail,
        prompt_line,
        confidence: confidence_for(frequency, evidence.len()),
        frequency,
        disposition: "signal".into(),
        pinned: false,
        tags: tags.into_iter().map(str::to_string).collect(),
        evidence,
        created_at: created_at.to_string(),
    }
}

pub fn confidence_for(frequency: u32, evidence_count: usize) -> f64 {
    let base = 42.0 + (frequency as f64 * 9.5) + (evidence_count.min(4) as f64 * 4.0);
    base.min(96.0)
}

pub fn build_summary(
    entries: &[MemoryEntry],
    merged_prs_analyzed: u32,
    review_feedback_items: u32,
    closed_issues_analyzed: u32,
) -> IngestSummary {
    let conventions = entries
        .iter()
        .filter(|entry| entry.kind == "review_rule" || entry.kind == "testing_expectation")
        .count() as u32;
    let failures = entries
        .iter()
        .filter(|entry| entry.kind == "failure_pattern")
        .count() as u32;
    let hotspots = entries
        .iter()
        .filter(|entry| entry.kind == "hotspot")
        .count() as u32;

    IngestSummary {
        merged_prs_analyzed,
        review_feedback_items,
        closed_issues_analyzed,
        memories_created: entries.len() as u32,
        conventions,
        failures,
        hotspots,
        top_memory: entries
            .first()
            .map(|entry| entry.title.clone())
            .unwrap_or_else(|| "No strong memory signals yet.".into()),
    }
}

pub fn build_prompt_pack(repo: &str, summary: &IngestSummary, entries: &[MemoryEntry]) -> String {
    let mut sections = Vec::new();
    let convention_lines: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "review_rule" || entry.kind == "testing_expectation")
        .map(|entry| format!("- {}", entry.prompt_line))
        .collect();
    let failure_lines: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "failure_pattern")
        .map(|entry| format!("- {}", entry.prompt_line))
        .collect();
    let hotspot_lines: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "hotspot")
        .map(|entry| format!("- {}", entry.prompt_line))
        .collect();
    let reviewer_lines: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "reviewer_profile")
        .map(|entry| format!("- {}", entry.prompt_line))
        .collect();
    let maintainer_lines: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "maintainer_profile")
        .map(|entry| format!("- {}", entry.prompt_line))
        .collect();

    if !convention_lines.is_empty() {
        sections.push(format!(
            "## Conventions and review habits\n{}",
            convention_lines.join("\n")
        ));
    }
    if !failure_lines.is_empty() {
        sections.push(format!(
            "## Failure patterns to watch\n{}",
            failure_lines.join("\n")
        ));
    }
    if !hotspot_lines.is_empty() {
        sections.push(format!(
            "## Hotspots\n{}",
            hotspot_lines.join("\n")
        ));
    }
    if !reviewer_lines.is_empty() {
        sections.push(format!(
            "## Reviewer feedback signatures\n{}",
            reviewer_lines.join("\n")
        ));
    }
    if !maintainer_lines.is_empty() {
        sections.push(format!(
            "## Maintainer patterns\n{}",
            maintainer_lines.join("\n")
        ));
    }

    if sections.is_empty() {
        sections.push("## Early signal\n- RepoMemory has not seen enough repeated patterns yet. Read recent merged PRs and reviewer comments before trusting automation.".into());
    }

    format!(
        "# RepoMemory Prompt Pack\n\nRepo: **{repo}**\nGenerated from **{}** merged PRs, **{}** review feedback items, and **{}** closed issues.\n\n{}\n",
        summary.merged_prs_analyzed,
        summary.review_feedback_items,
        summary.closed_issues_analyzed,
        sections.join("\n\n")
    )
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FailGuardCandidatePromoteRequest, FailGuardCandidateRequest, FailGuardLessonRequest};

    fn sample_entry(kind: &str, title: &str, detail: &str, prompt_line: &str) -> MemoryEntry {
        MemoryEntry {
            id: format!("id-{kind}"),
            memory_ref: format!("ref-{kind}"),
            run_id: "run-1".into(),
            repo: "patchhive/example".into(),
            kind: kind.into(),
            title: title.into(),
            detail: detail.into(),
            prompt_line: prompt_line.into(),
            confidence: 72.0,
            frequency: 3,
            disposition: "signal".into(),
            pinned: false,
            tags: vec![kind.into()],
            evidence: Vec::new(),
            created_at: "2026-04-11T00:00:00Z".into(),
        }
    }

    #[test]
    fn repo_reaper_prefers_maintainer_profiles_when_paths_match() {
        let maintainer = sample_entry(
            "maintainer_profile",
            "Merged patterns from @alex",
            "Recent merged work from @alex clusters in src/reaper.",
            "When touching src/reaper, match the conventions that recently landed in merged work from @alex.",
        );
        let reviewer = sample_entry(
            "reviewer_profile",
            "Review patterns from @sam",
            "Past feedback from @sam repeatedly focused on tests especially around docs/.",
            "Pre-empt the kinds of feedback @sam often gives when touching docs/.",
        );

        let ranked = rank_context_entries(
            &[reviewer, maintainer],
            "repo-reaper",
            &[String::from("src/reaper/fix_worker.rs")],
            "",
            "",
            4,
        );

        assert_eq!(
            ranked.first().map(|entry| entry.kind.as_str()),
            Some("maintainer_profile")
        );
    }

    #[test]
    fn trust_gate_prefers_reviewer_profiles_when_paths_match() {
        let maintainer = sample_entry(
            "maintainer_profile",
            "Merged patterns from @alex",
            "Recent merged work from @alex clusters in src/reaper.",
            "When touching src/reaper, match the conventions that recently landed in merged work from @alex.",
        );
        let reviewer = sample_entry(
            "reviewer_profile",
            "Review patterns from @sam",
            "Past feedback from @sam repeatedly focused on validation especially around src/reaper.",
            "Pre-empt the kinds of feedback @sam often gives when touching src/reaper.",
        );

        let ranked = rank_context_entries(
            &[maintainer, reviewer],
            "trust-gate",
            &[String::from("src/reaper/fix_worker.rs")],
            "",
            "",
            4,
        );

        assert_eq!(
            ranked.first().map(|entry| entry.kind.as_str()),
            Some("reviewer_profile")
        );
    }

    #[test]
    fn pinned_policy_entries_survive_fallback_and_outrank_regular_entries() {
        let mut policy = sample_entry(
            "testing_expectation",
            "Tests are expected for auth changes",
            "Recent fixes around auth nearly always shipped with tests.",
            "Add or update tests when touching auth behavior.",
        );
        policy.disposition = "policy".into();
        policy.pinned = true;

        let regular = sample_entry(
            "review_rule",
            "Use helper builders",
            "The repo prefers shared helper builders for config wiring.",
            "Prefer shared builders over inline config duplication.",
        );

        let ranked = rank_context_entries(&[regular, policy], "trust-gate", &[], "", "", 4);

        assert_eq!(ranked.len(), 2);
        assert_eq!(
            ranked.first().map(|entry| entry.disposition.as_str()),
            Some("policy")
        );
        assert_eq!(ranked.first().map(|entry| entry.pinned), Some(true));
    }

    #[test]
    fn failguard_lesson_builds_policy_failure_memory() {
        let run = build_failguard_lesson_run(
            FailGuardLessonRequest {
                repo: "patchhive/example".into(),
                title: "Webhook secrets must fail closed".into(),
                outcome: "Unsigned webhook could trigger autonomous work.".into(),
                lesson: "Public webhook routes must not accept unsigned payloads.".into(),
                prevention: "Reject webhook delivery when the signing secret is missing.".into(),
                affected_paths: vec!["backend/src/routes/webhook.rs".into()],
                evidence: vec!["Hermes review C2".into()],
                disposition: "policy".into(),
                pinned: true,
            },
            Vec::new(),
        );

        assert_eq!(run.summary.failures, 1);
        assert_eq!(run.entries.len(), 1);
        let entry = &run.entries[0];
        assert_eq!(entry.kind, "failure_pattern");
        assert_eq!(entry.disposition, "policy");
        assert!(entry.pinned);
        assert!(entry.tags.iter().any(|tag| tag == "failguard"));
        assert!(entry
            .evidence
            .iter()
            .any(|item| item.path.as_deref() == Some("backend/src/routes/webhook.rs")));
    }

    #[test]
    fn failguard_lesson_carries_forward_existing_snapshot() {
        let existing = sample_entry(
            "testing_expectation",
            "Tests are expected for auth changes",
            "Recent fixes around auth nearly always shipped with tests.",
            "Add or update tests when touching auth behavior.",
        );

        let run = build_failguard_lesson_run(
            FailGuardLessonRequest {
                repo: "patchhive/example".into(),
                title: "Webhook secrets must fail closed".into(),
                outcome: "Unsigned webhook could trigger autonomous work.".into(),
                lesson: "Public webhook routes must not accept unsigned payloads.".into(),
                prevention: "Reject webhook delivery when the signing secret is missing.".into(),
                affected_paths: vec!["backend/src/routes/webhook.rs".into()],
                evidence: Vec::new(),
                disposition: "policy".into(),
                pinned: true,
            },
            vec![existing],
        );

        assert_eq!(run.entries.len(), 2);
        assert!(run
            .entries
            .iter()
            .any(|entry| entry.kind == "testing_expectation"));
        assert!(run
            .entries
            .iter()
            .any(|entry| entry.title == "FailGuard: Webhook secrets must fail closed"));
    }

    #[test]
    fn failguard_candidate_drafts_reviewable_lesson() {
        let candidate = build_failguard_candidate(FailGuardCandidateRequest {
            repo: "patchhive/example".into(),
            source_type: "TrustGate block".into(),
            source_ref: "review-42".into(),
            title: "Diff touched auth without tests".into(),
            outcome: "TrustGate blocked a generated patch because auth behavior changed without coverage.".into(),
            lesson: String::new(),
            prevention: String::new(),
            affected_paths: vec!["src/auth.rs".into()],
            evidence: vec!["TrustGate block #42".into()],
            confidence: None,
        });

        assert_eq!(candidate.status, "open");
        assert_eq!(candidate.source_type, "trustgate-block");
        assert_eq!(candidate.confidence, 86.0);
        assert!(candidate.lesson.contains("durable guardrail"));
        assert!(candidate.prevention.contains("src/auth.rs"));
        assert!(candidate.evidence.iter().any(|item| item == "review-42"));
    }

    #[test]
    fn failguard_candidate_promotion_allows_operator_edits() {
        let candidate = build_failguard_candidate(FailGuardCandidateRequest {
            repo: "patchhive/example".into(),
            source_type: "repo-reaper-rejection".into(),
            source_ref: "run-7".into(),
            title: "Generated patch skipped webhook signing".into(),
            outcome: "Smith rejected a patch because webhook verification failed open.".into(),
            lesson: "Webhook verification cannot be optional on public routes.".into(),
            prevention: "Reject public webhook requests when signing configuration is absent.".into(),
            affected_paths: vec!["backend/src/routes/webhook.rs".into()],
            evidence: vec!["Smith rejection run-7".into()],
            confidence: Some(81.0),
        });

        let lesson = candidate_to_lesson_request(
            &candidate,
            FailGuardCandidatePromoteRequest {
                title: Some("Webhook signing must fail closed".into()),
                prevention: Some("Return 403 when webhook signing is unavailable.".into()),
                disposition: "policy".into(),
                pinned: true,
                ..Default::default()
            },
        );

        assert_eq!(lesson.title, "Webhook signing must fail closed");
        assert_eq!(lesson.outcome, candidate.outcome);
        assert_eq!(lesson.lesson, candidate.lesson);
        assert_eq!(
            lesson.prevention,
            "Return 403 when webhook signing is unavailable."
        );
        assert_eq!(lesson.affected_paths, candidate.affected_paths);
        assert_eq!(lesson.disposition, "policy");
        assert!(lesson.pinned);
    }

    #[test]
    fn suppressed_entries_are_filtered_out_of_context_results() {
        let mut suppressed = sample_entry(
            "failure_pattern",
            "Old flaky pattern",
            "A noisy signal that operators intentionally suppressed.",
            "Ignore this pattern.",
        );
        suppressed.disposition = "suppressed".into();

        let ranked = rank_context_entries(
            &[suppressed],
            "repo-reaper",
            &["src/lib.rs".into()],
            "",
            "",
            4,
        );
        assert!(ranked.is_empty());
    }
}

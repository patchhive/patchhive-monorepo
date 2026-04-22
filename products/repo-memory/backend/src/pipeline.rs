use std::collections::{HashMap, HashSet};

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use patchhive_product_core::contract;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{auth_enabled, generate_and_save_key, verify_token},
    db, github,
    models::{
        stable_memory_ref, ContextEntry, ContextRequest, ContextResponse, FailGuardCandidate,
        FailGuardCandidateDismissRequest, FailGuardCandidateListResponse,
        FailGuardCandidatePromoteRequest, FailGuardCandidatePromoteResponse,
        FailGuardCandidateRequest, FailGuardCandidateResponse, FailGuardLessonRequest,
        FailGuardLessonResponse, GitHubIssue, GitHubPullFile, GitHubPullRequest, GitHubReview,
        GitHubReviewComment, HistoryItem, IngestParams, IngestRecord, IngestSummary, KnownRepo,
        MemoryCurationUpdate, MemoryEntry, MemoryEvidence, OverviewPayload, RunDiffItem,
        RunDiffResponse, RunDiffSummary,
    },
    state::AppState,
    STARTUP_CHECKS,
};

type JsonError = (StatusCode, Json<serde_json::Value>);
type JsonResult<T> = Result<Json<T>, JsonError>;

#[derive(serde::Deserialize)]
pub struct LoginBody {
    api_key: String,
}

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
pub struct FailGuardCandidateQuery {
    repo: Option<String>,
    status: Option<String>,
}

#[derive(Clone)]
struct PullBundle {
    pr: GitHubPullRequest,
    reviews: Vec<GitHubReview>,
    comments: Vec<GitHubReviewComment>,
    files: Vec<GitHubPullFile>,
}

#[derive(Default)]
struct SignalBucket {
    frequency: u32,
    evidence: Vec<MemoryEvidence>,
}

#[derive(Default)]
struct ReviewerProfileBucket {
    total_feedback: u32,
    category_counts: HashMap<&'static str, u32>,
    path_counts: HashMap<String, u32>,
    evidence: Vec<MemoryEvidence>,
}

#[derive(Default)]
struct MaintainerProfileBucket {
    merged_prs: u32,
    source_prs: u32,
    source_with_tests: u32,
    path_counts: HashMap<String, u32>,
    evidence: Vec<MemoryEvidence>,
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

pub async fn capture_failguard_lesson(
    Json(request): Json<FailGuardLessonRequest>,
) -> JsonResult<FailGuardLessonResponse> {
    Ok(Json(save_failguard_lesson(request)?))
}

pub async fn failguard_candidates(
    Query(query): Query<FailGuardCandidateQuery>,
) -> JsonResult<FailGuardCandidateListResponse> {
    let status = normalize_candidate_status(query.status.as_deref().unwrap_or("open")).to_string();
    let candidates = db::list_failguard_candidates(query.repo.as_deref(), Some(&status))
        .map_err(internal_error)?;
    Ok(Json(FailGuardCandidateListResponse { candidates }))
}

pub async fn create_failguard_candidate(
    Json(mut request): Json<FailGuardCandidateRequest>,
) -> JsonResult<FailGuardCandidateResponse> {
    request.repo = request.repo.trim().to_string();
    request.title = request.title.trim().to_string();
    request.outcome = request.outcome.trim().to_string();
    request.lesson = request.lesson.trim().to_string();
    request.prevention = request.prevention.trim().to_string();
    request.source_type = normalize_source_type(&request.source_type);
    request.source_ref = request.source_ref.trim().to_string();

    if !valid_repo(&request.repo) {
        return Err(bad_request(
            "RepoMemory expects repos in owner/repo format.",
        ));
    }
    if request.title.is_empty() {
        return Err(bad_request("FailGuard candidates need a short title."));
    }
    if request.outcome.is_empty() {
        return Err(bad_request(
            "FailGuard candidates need the bad outcome they came from.",
        ));
    }

    let candidate = build_failguard_candidate(request);
    db::save_failguard_candidate(&candidate).map_err(internal_error)?;

    Ok(Json(FailGuardCandidateResponse {
        ok: true,
        message: "FailGuard candidate queued for review.".into(),
        candidate,
    }))
}

pub async fn promote_failguard_candidate(
    Path(id): Path<String>,
    Json(request): Json<FailGuardCandidatePromoteRequest>,
) -> JsonResult<FailGuardCandidatePromoteResponse> {
    let candidate = db::get_failguard_candidate(id.trim())
        .map_err(internal_error)?
        .ok_or_else(|| not_found("FailGuard candidate not found."))?;

    if candidate.status != "open" {
        return Err(bad_request("FailGuard can only promote open candidates."));
    }

    let response = save_failguard_lesson(candidate_to_lesson_request(&candidate, request))?;
    let note = "Promoted to RepoMemory failure-pattern policy.";
    db::update_failguard_candidate_status(
        &candidate.id,
        "promoted",
        Some(&response.entry.memory_ref),
        note,
    )
    .map_err(internal_error)?;
    let updated = db::get_failguard_candidate(&candidate.id)
        .map_err(internal_error)?
        .unwrap_or(candidate);

    Ok(Json(FailGuardCandidatePromoteResponse {
        ok: true,
        message: "FailGuard candidate promoted into RepoMemory.".into(),
        candidate: updated,
        run: response.run,
        entry: response.entry,
    }))
}

pub async fn dismiss_failguard_candidate(
    Path(id): Path<String>,
    Json(request): Json<FailGuardCandidateDismissRequest>,
) -> JsonResult<FailGuardCandidateResponse> {
    let candidate = db::get_failguard_candidate(id.trim())
        .map_err(internal_error)?
        .ok_or_else(|| not_found("FailGuard candidate not found."))?;

    if candidate.status != "open" {
        return Err(bad_request("FailGuard can only dismiss open candidates."));
    }

    let reason = request.reason.trim();
    let note = if reason.is_empty() {
        "Dismissed by operator."
    } else {
        reason
    };
    db::update_failguard_candidate_status(&candidate.id, "dismissed", None, note)
        .map_err(internal_error)?;
    let updated = db::get_failguard_candidate(&candidate.id)
        .map_err(internal_error)?
        .unwrap_or(candidate);

    Ok(Json(FailGuardCandidateResponse {
        ok: true,
        message: "FailGuard candidate dismissed.".into(),
        candidate: updated,
    }))
}

fn save_failguard_lesson(
    mut request: FailGuardLessonRequest,
) -> std::result::Result<FailGuardLessonResponse, JsonError> {
    request.repo = request.repo.trim().to_string();
    request.title = request.title.trim().to_string();
    request.outcome = request.outcome.trim().to_string();
    request.lesson = request.lesson.trim().to_string();
    request.prevention = request.prevention.trim().to_string();
    request.disposition = normalize_disposition(&request.disposition).to_string();

    if !valid_repo(&request.repo) {
        return Err(bad_request(
            "RepoMemory expects repos in owner/repo format.",
        ));
    }
    if request.title.is_empty() {
        return Err(bad_request("FailGuard needs a short lesson title."));
    }
    if request.outcome.is_empty() {
        return Err(bad_request(
            "FailGuard needs the bad outcome this lesson came from.",
        ));
    }
    if request.lesson.is_empty() {
        return Err(bad_request(
            "FailGuard needs the durable lesson this repo should remember.",
        ));
    }
    if request.prevention.is_empty() {
        return Err(bad_request(
            "FailGuard needs the future prevention rule or guardrail.",
        ));
    }

    let carry_forward = latest_repo_entries(&request.repo)?;
    let captured_title = format!("FailGuard: {}", request.title);
    let run = build_failguard_lesson_run(request, carry_forward);
    let entry = run
        .entries
        .iter()
        .find(|entry| entry.title == captured_title)
        .cloned()
        .ok_or_else(|| internal_error(anyhow::anyhow!("FailGuard lesson did not create memory")))?;

    db::save_run(&run).map_err(internal_error)?;
    db::save_memory_curation(
        &entry.repo,
        &entry.memory_ref,
        &entry.disposition,
        entry.pinned,
    )
    .map_err(internal_error)?;

    Ok(FailGuardLessonResponse {
        ok: true,
        message: "FailGuard lesson captured as a RepoMemory failure-pattern policy.".into(),
        run,
        entry,
    })
}

fn latest_repo_entries(repo: &str) -> std::result::Result<Vec<MemoryEntry>, JsonError> {
    let latest = db::list_history(Some(repo))
        .map_err(internal_error)?
        .into_iter()
        .next();
    let Some(latest) = latest else {
        return Ok(Vec::new());
    };

    let run = db::get_history(&latest.id)
        .map_err(internal_error)?
        .ok_or_else(|| not_found("RepoMemory latest run not found."))?;
    Ok(run.entries)
}

fn build_failguard_candidate(request: FailGuardCandidateRequest) -> FailGuardCandidate {
    let now = Utc::now().to_rfc3339();
    let affected_paths = clean_failguard_items(request.affected_paths, 12);
    let mut evidence = clean_failguard_items(request.evidence, 10);
    if !request.source_ref.trim().is_empty()
        && !evidence
            .iter()
            .any(|item| item.trim() == request.source_ref.trim())
    {
        evidence.insert(0, request.source_ref.trim().to_string());
        evidence.truncate(10);
    }
    let lesson = if request.lesson.trim().is_empty() {
        draft_failguard_lesson(&request.title, &request.outcome)
    } else {
        truncate(request.lesson.trim(), 260)
    };
    let prevention = if request.prevention.trim().is_empty() {
        draft_failguard_prevention(&request.title, &request.outcome, &affected_paths)
    } else {
        truncate(request.prevention.trim(), 260)
    };

    FailGuardCandidate {
        id: Uuid::new_v4().to_string(),
        repo: request.repo.trim().to_string(),
        source_type: normalize_source_type(&request.source_type),
        source_ref: request.source_ref.trim().to_string(),
        title: truncate(request.title.trim(), 140),
        outcome: truncate(request.outcome.trim(), 320),
        lesson,
        prevention,
        affected_paths,
        evidence,
        confidence: normalized_candidate_confidence(request.confidence, &request.source_type),
        status: "open".into(),
        memory_ref: String::new(),
        resolution_note: String::new(),
        created_at: now.clone(),
        updated_at: now,
    }
}

fn candidate_to_lesson_request(
    candidate: &FailGuardCandidate,
    request: FailGuardCandidatePromoteRequest,
) -> FailGuardLessonRequest {
    let title = promote_text(request.title, &candidate.title);
    let outcome = promote_text(request.outcome, &candidate.outcome);
    let lesson = promote_text(request.lesson, &candidate.lesson);
    let prevention = promote_text(request.prevention, &candidate.prevention);
    let affected_paths = request
        .affected_paths
        .unwrap_or_else(|| candidate.affected_paths.clone());
    let evidence = request
        .evidence
        .unwrap_or_else(|| candidate.evidence.clone());

    FailGuardLessonRequest {
        repo: candidate.repo.clone(),
        title,
        outcome,
        lesson,
        prevention,
        affected_paths,
        evidence,
        disposition: normalize_disposition(&request.disposition).to_string(),
        pinned: request.pinned,
    }
}

fn promote_text(next: Option<String>, fallback: &str) -> String {
    next.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn draft_failguard_lesson(title: &str, outcome: &str) -> String {
    truncate(
        &format!(
            "A previous failure showed this repo must treat \"{}\" as a durable guardrail. Outcome to remember: {}",
            title.trim(),
            outcome.trim()
        ),
        260,
    )
}

fn draft_failguard_prevention(title: &str, outcome: &str, affected_paths: &[String]) -> String {
    let scope = affected_paths
        .first()
        .map(|path| format!(" around {path}"))
        .unwrap_or_default();
    truncate(
        &format!(
            "Before similar changes are accepted{scope}, verify the work prevents this failure mode: {}. Original signal: {}",
            title.trim(),
            outcome.trim()
        ),
        260,
    )
}

fn normalized_candidate_confidence(confidence: Option<f64>, source_type: &str) -> f64 {
    let default = match normalize_source_type(source_type).as_str() {
        "trustgate-block" | "trust-gate-block" => 86.0,
        "trustgate-warn" | "trust-gate-warn" => 78.0,
        "repo-reaper-rejection" | "repo_reaper_rejection" => 82.0,
        "reverted-pr" | "reverted_pr" => 88.0,
        "reviewbee-thread" | "reviewbee_thread" => 74.0,
        _ => 70.0,
    };
    let value = confidence.unwrap_or(default);
    if value.is_finite() {
        value.clamp(10.0, 96.0)
    } else {
        default
    }
}

fn normalize_source_type(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.trim().chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let normalized = out.trim_matches('-');
    if normalized.is_empty() {
        "operator".into()
    } else {
        normalized.to_string()
    }
}

fn normalize_candidate_status(value: &str) -> &str {
    match value.trim().to_ascii_lowercase().as_str() {
        "all" => "all",
        "promoted" => "promoted",
        "dismissed" => "dismissed",
        _ => "open",
    }
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

fn build_memory_run(
    params: IngestParams,
    bundles: Vec<PullBundle>,
    issues: Vec<GitHubIssue>,
) -> Result<IngestRecord> {
    let run_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let repo = params.repo.clone();

    let mut entries = Vec::new();
    let mut review_buckets: HashMap<&'static str, SignalBucket> = HashMap::new();
    let mut dir_counts: HashMap<String, u32> = HashMap::new();
    let mut file_review_counts: HashMap<String, SignalBucket> = HashMap::new();
    let mut bug_terms: HashMap<String, SignalBucket> = HashMap::new();
    let mut reviewer_profiles: HashMap<String, ReviewerProfileBucket> = HashMap::new();
    let mut maintainer_profiles: HashMap<String, MaintainerProfileBucket> = HashMap::new();
    let mut source_prs = 0u32;
    let mut source_with_tests = 0u32;
    let mut review_feedback_items = 0u32;

    for bundle in &bundles {
        let mut touched_source = false;
        let mut touched_tests = false;
        let author_login = bundle
            .pr
            .user
            .as_ref()
            .map(|user| user.login.trim().to_string())
            .filter(|login| !login.is_empty());

        for file in &bundle.files {
            if is_source_file(&file.filename) {
                touched_source = true;
            }
            if is_test_file(&file.filename) {
                touched_tests = true;
            }

            let bucket = path_bucket(&file.filename);
            *dir_counts.entry(bucket).or_insert(0) += 1;
            if let Some(author) = author_login.as_ref() {
                *maintainer_profiles
                    .entry(author.clone())
                    .or_default()
                    .path_counts
                    .entry(path_bucket(&file.filename))
                    .or_insert(0) += 1;
            }
        }

        if let Some(author) = author_login.as_ref() {
            let profile = maintainer_profiles.entry(author.clone()).or_default();
            profile.merged_prs += 1;
            if touched_source {
                profile.source_prs += 1;
                if touched_tests {
                    profile.source_with_tests += 1;
                }
            }
            push_evidence(
                &mut profile.evidence,
                MemoryEvidence {
                    source_type: "merged_pr".into(),
                    title: format!("#{} {}", bundle.pr.number, bundle.pr.title),
                    url: bundle.pr.html_url.clone(),
                    path: None,
                    excerpt: truncate(
                        &format!(
                            "{} merged with {} changed files.",
                            author,
                            bundle.pr.changed_files.unwrap_or(bundle.files.len() as u32)
                        ),
                        180,
                    ),
                },
            );
        }

        if touched_source {
            source_prs += 1;
            if touched_tests {
                source_with_tests += 1;
            }
        }

        for review in &bundle.reviews {
            if review.state.eq_ignore_ascii_case("commented")
                || review.state.eq_ignore_ascii_case("changes_requested")
                || review.state.eq_ignore_ascii_case("approved")
            {
                if let Some(body) = review.body.as_deref() {
                    review_feedback_items += collect_feedback(
                        &mut review_buckets,
                        &mut reviewer_profiles,
                        &bundle.pr,
                        None,
                        review.html_url.as_deref().unwrap_or(&bundle.pr.html_url),
                        body,
                        review.user.as_ref().map(|user| user.login.as_str()),
                    );
                }
            }
        }

        for comment in &bundle.comments {
            review_feedback_items += collect_feedback(
                &mut review_buckets,
                &mut reviewer_profiles,
                &bundle.pr,
                comment.path.as_deref(),
                &comment.html_url,
                &comment.body,
                comment.user.as_ref().map(|user| user.login.as_str()),
            );

            if let Some(path) = comment.path.as_deref() {
                let entry = file_review_counts.entry(path.to_string()).or_default();
                entry.frequency += 1;
                push_evidence(
                    &mut entry.evidence,
                    MemoryEvidence {
                        source_type: "review_comment".into(),
                        title: format!("#{} {}", bundle.pr.number, bundle.pr.title),
                        url: comment.html_url.clone(),
                        path: Some(path.to_string()),
                        excerpt: truncate(comment.body.trim(), 180),
                    },
                );
            }
        }
    }

    for issue in &issues {
        if !looks_bug_like(issue) {
            continue;
        }
        let title_tokens = tokenize(&format!(
            "{} {}",
            issue.title,
            issue.body.clone().unwrap_or_default()
        ));

        for token in title_tokens {
            if token.len() < 4 || STOPWORDS.contains(&token.as_str()) {
                continue;
            }
            let bucket = bug_terms.entry(token.clone()).or_default();
            bucket.frequency += 1;
            push_evidence(
                &mut bucket.evidence,
                MemoryEvidence {
                    source_type: "issue".into(),
                    title: format!("#{} {}", issue.number, issue.title),
                    url: issue.html_url.clone(),
                    path: None,
                    excerpt: truncate(issue.body.as_deref().unwrap_or(issue.title.as_str()), 180),
                },
            );
        }
    }

    for (key, label, prompt_line, detail_line, tags) in review_bucket_specs() {
        if let Some(bucket) = review_buckets.get(key) {
            if bucket.frequency < 2 {
                continue;
            }
            entries.push(build_entry(
                &run_id,
                &repo,
                "review_rule",
                label,
                format!("{detail_line} Reviewer feedback surfaced this pattern {} times across recent merged PRs.", bucket.frequency),
                prompt_line,
                bucket.frequency,
                tags,
                bucket.evidence.clone(),
                &created_at,
            ));
        }
    }

    if source_prs >= 3 && source_with_tests >= 2 {
        let ratio = source_with_tests as f64 / source_prs as f64;
        if ratio >= 0.5 {
            entries.push(build_entry(
                &run_id,
                &repo,
                "testing_expectation",
                "Behavior changes usually ship with tests",
                format!(
                    "{} of the last {} merged PRs that touched source files also updated tests. This repo tends to expect test coverage when behavior changes.",
                    source_with_tests, source_prs
                ),
                "When behavior changes or bugs are fixed, update or add tests in the same patch.",
                source_with_tests,
                vec!["tests", "merged-pr-pattern"],
                Vec::new(),
                &created_at,
            ));
        }
    }

    let mut hotspot_dirs: Vec<_> = dir_counts.into_iter().collect();
    hotspot_dirs.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for (dir, frequency) in hotspot_dirs
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .take(3)
    {
        entries.push(build_entry(
            &run_id,
            &repo,
            "hotspot",
            format!("Recent fixes cluster in {dir}"),
            format!(
                "Recent merged PRs repeatedly touched {dir}. Treat it as a high-context area and read nearby helpers, tests, and conventions before changing it."
            ),
            format!("Treat {dir} as a high-context area; read nearby code and tests before editing it."),
            frequency,
            vec!["hotspot", "paths"],
            Vec::new(),
            &created_at,
        ));
    }

    let mut review_paths: Vec<_> = file_review_counts.into_iter().collect();
    review_paths.sort_by(|left, right| {
        right
            .1
            .frequency
            .cmp(&left.1.frequency)
            .then_with(|| left.0.cmp(&right.0))
    });
    for (path, bucket) in review_paths
        .into_iter()
        .filter(|(_, bucket)| bucket.frequency >= 2)
        .take(3)
    {
        entries.push(build_entry(
            &run_id,
            &repo,
            "hotspot",
            format!("{path} attracts repeat review churn"),
            format!(
                "Reviewer comments keep landing on {path}. This file or area likely encodes conventions that agents should read before making edits."
            ),
            format!("Read {path} carefully before editing; this path attracts repeat review feedback."),
            bucket.frequency,
            vec!["review-churn", "paths"],
            bucket.evidence,
            &created_at,
        ));
    }

    let mut bug_terms: Vec<_> = bug_terms.into_iter().collect();
    bug_terms.sort_by(|left, right| {
        right
            .1
            .frequency
            .cmp(&left.1.frequency)
            .then_with(|| left.0.cmp(&right.0))
    });
    for (term, bucket) in bug_terms
        .into_iter()
        .filter(|(_, bucket)| bucket.frequency >= 2)
        .take(4)
    {
        entries.push(build_entry(
            &run_id,
            &repo,
            "failure_pattern",
            format!("Recurring failures mention '{term}'"),
            format!(
                "Closed bug reports repeatedly mention {term}. RepoMemory is treating that as a repeated failure pattern worth checking before new patches move forward."
            ),
            format!("Re-check {term}-adjacent behavior and edge cases before finalizing a patch."),
            bucket.frequency,
            vec!["bugs", "issues", "failure-pattern"],
            bucket.evidence,
            &created_at,
        ));
    }

    let mut reviewer_profiles: Vec<_> = reviewer_profiles.into_iter().collect();
    reviewer_profiles.sort_by(|left, right| {
        right
            .1
            .total_feedback
            .cmp(&left.1.total_feedback)
            .then_with(|| left.0.cmp(&right.0))
    });
    for (reviewer, profile) in reviewer_profiles
        .into_iter()
        .filter(|(_, profile)| profile.total_feedback >= 2)
        .take(4)
    {
        let focus = top_named_counts(&profile.category_counts, 2)
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        let paths = top_string_counts(&profile.path_counts, 2)
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>();
        let focus_text = if focus.is_empty() {
            "general review consistency".into()
        } else {
            focus.join(" and ")
        };
        let path_text = if paths.is_empty() {
            "across the repo".into()
        } else {
            format!("especially around {}", paths.join(" and "))
        };

        entries.push(build_entry(
            &run_id,
            &repo,
            "reviewer_profile",
            format!("Review patterns from @{reviewer}"),
            format!(
                "Past feedback from @{reviewer} repeatedly focused on {focus_text} {path_text}. RepoMemory is keeping that signature so future patches can pre-empt the same review friction."
            ),
            format!(
                "Pre-empt the kinds of feedback @{reviewer} often gives around {focus_text} {}.",
                if paths.is_empty() {
                    "before you ship changes".into()
                } else {
                    format!("when touching {}", paths.join(" and "))
                }
            ),
            profile.total_feedback,
            vec!["reviewer-profile", "feedback-signature"],
            profile.evidence,
            &created_at,
        ));
    }

    let mut maintainer_profiles: Vec<_> = maintainer_profiles.into_iter().collect();
    maintainer_profiles.sort_by(|left, right| {
        right
            .1
            .merged_prs
            .cmp(&left.1.merged_prs)
            .then_with(|| left.0.cmp(&right.0))
    });
    for (author, profile) in maintainer_profiles
        .into_iter()
        .filter(|(_, profile)| profile.merged_prs >= 2)
        .take(4)
    {
        let paths = top_string_counts(&profile.path_counts, 2)
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>();
        let test_ratio = if profile.source_prs > 0 {
            profile.source_with_tests as f64 / profile.source_prs as f64
        } else {
            0.0
        };
        let path_text = if paths.is_empty() {
            "across the repo".into()
        } else {
            paths.join(" and ")
        };
        let test_text = if profile.source_prs >= 2 && test_ratio >= 0.5 {
            "Recent merged work from this author usually pairs source changes with tests."
        } else {
            "Recent merged work from this author mostly signals where accepted patterns keep landing."
        };

        entries.push(build_entry(
            &run_id,
            &repo,
            "maintainer_profile",
            format!("Merged patterns from @{author}"),
            format!(
                "Recent merged work from @{author} clusters in {path_text}. {test_text}"
            ),
            format!(
                "When touching {path_text}, match the conventions that recently landed in merged work from @{author}."
            ),
            profile.merged_prs,
            vec!["maintainer-profile", "merged-history"],
            profile.evidence,
            &created_at,
        ));
    }

    entries.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.frequency.cmp(&left.frequency))
    });

    let summary = build_summary(
        &entries,
        bundles.len() as u32,
        review_feedback_items,
        issues.len() as u32,
    );
    let prompt_pack = build_prompt_pack(&repo, &summary, &entries);

    Ok(IngestRecord {
        id: run_id,
        repo,
        created_at,
        params,
        summary,
        prompt_pack,
        entries,
    })
}

fn build_failguard_lesson_run(
    request: FailGuardLessonRequest,
    carry_forward: Vec<MemoryEntry>,
) -> IngestRecord {
    let run_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let repo = request.repo.trim().to_string();
    let title = request.title.trim().to_string();
    let disposition = normalize_disposition(&request.disposition).to_string();
    let pinned = request.pinned || disposition == "policy";
    let affected_paths = clean_failguard_items(request.affected_paths, 12);
    let evidence_notes = clean_failguard_items(request.evidence, 8);
    let memory_title = format!("FailGuard: {title}");
    let outcome = truncate(request.outcome.trim(), 260);
    let lesson = truncate(request.lesson.trim(), 260);
    let prevention = truncate(request.prevention.trim(), 260);
    let detail = format!("Outcome: {outcome}\nLesson: {lesson}\nPrevention: {prevention}");
    let prompt_line = format!("FailGuard policy for {repo}: {prevention} Remember: {lesson}");

    let mut tags = vec![
        "failguard".to_string(),
        "manual".to_string(),
        "failure".to_string(),
        "policy".to_string(),
    ];
    for path in &affected_paths {
        let bucket = path_bucket(path);
        if !bucket.is_empty() {
            tags.push(bucket);
        }
    }
    tags.sort();
    tags.dedup();

    let mut evidence = Vec::new();
    for path in &affected_paths {
        evidence.push(MemoryEvidence {
            source_type: "failguard_path".into(),
            title: "Affected path".into(),
            url: String::new(),
            path: Some(path.clone()),
            excerpt: truncate(&format!("FailGuard lesson applies here: {prevention}"), 220),
        });
    }

    for note in &evidence_notes {
        let url = if note.starts_with("http://") || note.starts_with("https://") {
            note.clone()
        } else {
            String::new()
        };
        evidence.push(MemoryEvidence {
            source_type: "failguard_evidence".into(),
            title: if url.is_empty() {
                "Operator evidence".into()
            } else {
                "External evidence".into()
            },
            url,
            path: None,
            excerpt: truncate(note, 260),
        });
    }

    if evidence.is_empty() {
        evidence.push(MemoryEvidence {
            source_type: "failguard_lesson".into(),
            title: memory_title.clone(),
            url: String::new(),
            path: None,
            excerpt: truncate(&detail, 320),
        });
    }

    let new_memory_ref = stable_memory_ref(&repo, "failure_pattern", &memory_title);
    let entry = MemoryEntry {
        id: Uuid::new_v4().to_string(),
        memory_ref: new_memory_ref.clone(),
        run_id: run_id.clone(),
        repo: repo.clone(),
        kind: "failure_pattern".into(),
        title: memory_title.clone(),
        detail,
        prompt_line: prompt_line.clone(),
        confidence: if disposition == "policy" { 94.0 } else { 82.0 },
        frequency: 1,
        disposition,
        pinned,
        tags,
        evidence,
        created_at: created_at.clone(),
    };

    let mut entries = carry_forward
        .into_iter()
        .filter(|entry| entry.memory_ref != new_memory_ref)
        .map(|mut entry| {
            entry.id = Uuid::new_v4().to_string();
            entry.run_id = run_id.clone();
            entry.created_at = created_at.clone();
            entry
        })
        .collect::<Vec<_>>();
    entries.push(entry);
    entries.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then_with(|| {
                disposition_rank(&right.disposition).cmp(&disposition_rank(&left.disposition))
            })
            .then_with(|| {
                right
                    .confidence
                    .partial_cmp(&left.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.frequency.cmp(&left.frequency))
    });
    let summary = build_summary(&entries, 0, 0, 0);
    let prompt_pack = build_prompt_pack(&repo, &summary, &entries);

    IngestRecord {
        id: run_id,
        repo: repo.clone(),
        created_at,
        params: IngestParams {
            repo,
            merged_pr_limit: 0,
            issue_limit: 0,
            since_days: 0,
        },
        summary,
        prompt_pack,
        entries,
    }
}

fn clean_failguard_items(items: Vec<String>, limit: usize) -> Vec<String> {
    let mut clean = items
        .into_iter()
        .map(|item| item.trim().trim_start_matches("./").to_string())
        .filter(|item| !item.is_empty())
        .take(limit)
        .collect::<Vec<_>>();
    clean.sort();
    clean.dedup();
    clean
}

fn build_summary(
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

fn build_prompt_pack(repo: &str, summary: &IngestSummary, entries: &[MemoryEntry]) -> String {
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
        sections.push(format!("## Hotspots\n{}", hotspot_lines.join("\n")));
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

fn build_run_diff(current: IngestRecord, previous: Option<IngestRecord>) -> RunDiffResponse {
    let Some(previous) = previous else {
        return RunDiffResponse {
            repo: current.repo,
            run_id: current.id,
            previous_run_id: None,
            created_at: current.created_at,
            previous_created_at: None,
            summary: "This is the first recorded RepoMemory run for this repo, so there is no earlier memory snapshot to compare yet.".into(),
            counts: RunDiffSummary {
                new_entries: current.entries.len() as u32,
                strengthened_entries: 0,
                faded_entries: 0,
                retired_entries: 0,
            },
            new_entries: current
                .entries
                .iter()
                .map(|entry| RunDiffItem {
                    memory_ref: entry.memory_ref.clone(),
                    kind: entry.kind.clone(),
                    title: entry.title.clone(),
                    prompt_line: entry.prompt_line.clone(),
                    current_confidence: Some(entry.confidence),
                    previous_confidence: None,
                    current_frequency: Some(entry.frequency),
                    previous_frequency: None,
                    delta_confidence: entry.confidence,
                    delta_frequency: entry.frequency as i32,
                })
                .take(8)
                .collect(),
            strengthened_entries: Vec::new(),
            faded_entries: Vec::new(),
            retired_entries: Vec::new(),
        };
    };

    let current_repo = current.repo.clone();
    let current_id = current.id.clone();
    let current_created_at = current.created_at.clone();
    let previous_id = previous.id.clone();
    let previous_created_at = previous.created_at.clone();
    let current_entries = current.entries;
    let previous_entries = previous.entries;

    let current_map = current_entries
        .iter()
        .map(|entry| (entry.memory_ref.clone(), entry))
        .collect::<HashMap<_, _>>();
    let previous_map = previous_entries
        .iter()
        .map(|entry| (entry.memory_ref.clone(), entry))
        .collect::<HashMap<_, _>>();

    let mut new_entries = Vec::new();
    let mut strengthened_entries = Vec::new();
    let mut faded_entries = Vec::new();
    let mut retired_entries = Vec::new();

    for entry in &current_entries {
        match previous_map.get(&entry.memory_ref) {
            None => new_entries.push(RunDiffItem {
                memory_ref: entry.memory_ref.clone(),
                kind: entry.kind.clone(),
                title: entry.title.clone(),
                prompt_line: entry.prompt_line.clone(),
                current_confidence: Some(entry.confidence),
                previous_confidence: None,
                current_frequency: Some(entry.frequency),
                previous_frequency: None,
                delta_confidence: entry.confidence,
                delta_frequency: entry.frequency as i32,
            }),
            Some(previous_entry) => {
                let delta_confidence = entry.confidence - previous_entry.confidence;
                let delta_frequency = entry.frequency as i32 - previous_entry.frequency as i32;
                if delta_confidence >= 4.0 || delta_frequency >= 1 {
                    strengthened_entries.push(diff_item(entry, Some(previous_entry)));
                } else if delta_confidence <= -4.0 || delta_frequency <= -1 {
                    faded_entries.push(diff_item(entry, Some(previous_entry)));
                }
            }
        }
    }

    for entry in &previous_entries {
        if !current_map.contains_key(&entry.memory_ref) {
            retired_entries.push(RunDiffItem {
                memory_ref: entry.memory_ref.clone(),
                kind: entry.kind.clone(),
                title: entry.title.clone(),
                prompt_line: entry.prompt_line.clone(),
                current_confidence: None,
                previous_confidence: Some(entry.confidence),
                current_frequency: None,
                previous_frequency: Some(entry.frequency),
                delta_confidence: -entry.confidence,
                delta_frequency: -(entry.frequency as i32),
            });
        }
    }

    sort_diff_items(&mut new_entries);
    sort_diff_items(&mut strengthened_entries);
    sort_diff_items(&mut faded_entries);
    sort_diff_items(&mut retired_entries);

    let counts = RunDiffSummary {
        new_entries: new_entries.len() as u32,
        strengthened_entries: strengthened_entries.len() as u32,
        faded_entries: faded_entries.len() as u32,
        retired_entries: retired_entries.len() as u32,
    };

    let summary = if counts.new_entries == 0
        && counts.strengthened_entries == 0
        && counts.faded_entries == 0
        && counts.retired_entries == 0
    {
        format!(
            "RepoMemory did not detect any major changes between this run and the previous snapshot for {}.",
            current_repo
        )
    } else {
        format!(
            "Compared with the previous RepoMemory run, {} has {} new, {} strengthened, {} faded, and {} retired memories.",
            current_repo,
            counts.new_entries,
            counts.strengthened_entries,
            counts.faded_entries,
            counts.retired_entries,
        )
    };

    RunDiffResponse {
        repo: current_repo,
        run_id: current_id,
        previous_run_id: Some(previous_id),
        created_at: current_created_at,
        previous_created_at: Some(previous_created_at),
        summary,
        counts,
        new_entries: new_entries.into_iter().take(8).collect(),
        strengthened_entries: strengthened_entries.into_iter().take(8).collect(),
        faded_entries: faded_entries.into_iter().take(8).collect(),
        retired_entries: retired_entries.into_iter().take(8).collect(),
    }
}

fn diff_item(current: &MemoryEntry, previous: Option<&MemoryEntry>) -> RunDiffItem {
    let previous_confidence = previous.map(|entry| entry.confidence);
    let previous_frequency = previous.map(|entry| entry.frequency);
    RunDiffItem {
        memory_ref: current.memory_ref.clone(),
        kind: current.kind.clone(),
        title: current.title.clone(),
        prompt_line: current.prompt_line.clone(),
        current_confidence: Some(current.confidence),
        previous_confidence,
        current_frequency: Some(current.frequency),
        previous_frequency,
        delta_confidence: current.confidence - previous_confidence.unwrap_or(0.0),
        delta_frequency: current.frequency as i32 - previous_frequency.unwrap_or(0) as i32,
    }
}

fn sort_diff_items(items: &mut [RunDiffItem]) {
    items.sort_by(|left, right| {
        let right_magnitude =
            right.delta_confidence.abs() + (right.delta_frequency.abs() as f64 * 5.0);
        let left_magnitude =
            left.delta_confidence.abs() + (left.delta_frequency.abs() as f64 * 5.0);
        right_magnitude
            .partial_cmp(&left_magnitude)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.kind.cmp(&left.kind))
            .then_with(|| left.title.cmp(&right.title))
    });
}

fn rank_context_entries(
    entries: &[MemoryEntry],
    consumer: &str,
    changed_paths: &[String],
    task_summary: &str,
    diff_summary: &str,
    limit: usize,
) -> Vec<ContextEntry> {
    let clean_paths: Vec<String> = changed_paths
        .iter()
        .map(|path| path.trim().trim_start_matches("./").to_string())
        .filter(|path| !path.is_empty())
        .collect();
    let context_tokens = tokenize_context(&format!("{task_summary} {diff_summary}"));

    let mut ranked = entries
        .iter()
        .map(|entry| {
            let disposition = normalize_disposition(&entry.disposition).to_string();
            if disposition == "suppressed" {
                return ContextEntry {
                    id: entry.id.clone(),
                    memory_ref: entry.memory_ref.clone(),
                    kind: entry.kind.clone(),
                    title: entry.title.clone(),
                    detail: entry.detail.clone(),
                    prompt_line: entry.prompt_line.clone(),
                    confidence: entry.confidence,
                    frequency: entry.frequency,
                    retrieval_score: -10_000.0,
                    disposition,
                    pinned: entry.pinned,
                    matched_paths: Vec::new(),
                    matched_terms: Vec::new(),
                    tags: entry.tags.clone(),
                    evidence: entry.evidence.clone(),
                };
            }
            let matched_paths = matching_entry_paths(entry, &clean_paths);
            let matched_terms = matching_entry_terms(entry, &context_tokens);
            let retrieval_score = entry.confidence * 0.48
                + (entry.frequency as f64 * 6.0)
                + (matched_paths.len() as f64 * 18.0)
                + (matched_terms.len() as f64 * 7.0)
                + context_kind_bonus(entry, &clean_paths, consumer)
                + profile_path_bonus(entry, &clean_paths, &matched_paths, consumer)
                + curation_bonus(entry);

            ContextEntry {
                id: entry.id.clone(),
                memory_ref: entry.memory_ref.clone(),
                kind: entry.kind.clone(),
                title: entry.title.clone(),
                detail: entry.detail.clone(),
                prompt_line: entry.prompt_line.clone(),
                confidence: entry.confidence,
                frequency: entry.frequency,
                retrieval_score,
                disposition,
                pinned: entry.pinned,
                matched_paths,
                matched_terms,
                tags: entry.tags.clone(),
                evidence: entry.evidence.clone(),
            }
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then_with(|| {
                disposition_rank(&right.disposition).cmp(&disposition_rank(&left.disposition))
            })
            .then_with(|| {
                right
                    .retrieval_score
                    .partial_cmp(&left.retrieval_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.frequency.cmp(&left.frequency))
    });

    let fallback_mode = clean_paths.is_empty() && context_tokens.is_empty();
    ranked
        .into_iter()
        .filter(|entry| entry.disposition != "suppressed")
        .filter(|entry| {
            fallback_mode
                || entry.pinned
                || entry.disposition == "policy"
                || !entry.matched_paths.is_empty()
                || !entry.matched_terms.is_empty()
                || entry.retrieval_score >= 68.0
        })
        .take(limit)
        .collect()
}

fn matching_entry_paths(entry: &MemoryEntry, changed_paths: &[String]) -> Vec<String> {
    let mut matched = Vec::new();
    for path in changed_paths {
        let path_lower = path.to_ascii_lowercase();
        let path_bucket = path_bucket(path).to_ascii_lowercase();
        let text = format!(
            "{} {} {} {}",
            entry.title,
            entry.detail,
            entry.prompt_line,
            entry.tags.join(" ")
        )
        .to_ascii_lowercase();

        let direct_match = entry
            .evidence
            .iter()
            .filter_map(|evidence| evidence.path.as_ref())
            .any(|candidate| path_matches_candidate(&path_lower, &path_bucket, candidate));

        if direct_match || text.contains(&path_bucket) || text.contains(&path_lower) {
            matched.push(path.clone());
        }
    }
    matched.sort();
    matched.dedup();
    matched
}

fn matching_entry_terms(entry: &MemoryEntry, context_tokens: &HashSet<String>) -> Vec<String> {
    if context_tokens.is_empty() {
        return Vec::new();
    }

    let entry_tokens = tokenize_context(&format!(
        "{} {} {} {} {}",
        entry.title,
        entry.detail,
        entry.prompt_line,
        entry.tags.join(" "),
        entry
            .evidence
            .iter()
            .map(|evidence| evidence.excerpt.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    ));

    let mut matched = context_tokens
        .intersection(&entry_tokens)
        .cloned()
        .collect::<Vec<_>>();
    matched.sort();
    matched.truncate(4);
    matched
}

fn context_kind_bonus(entry: &MemoryEntry, changed_paths: &[String], consumer: &str) -> f64 {
    match consumer {
        "trust-gate" => match entry.kind.as_str() {
            "testing_expectation" => 11.0,
            "failure_pattern" => 9.0,
            "hotspot" if !changed_paths.is_empty() => 8.0,
            "reviewer_profile" => 6.0,
            "maintainer_profile" => 3.0,
            "review_rule" => 5.0,
            _ => 0.0,
        },
        "repo-reaper" => match entry.kind.as_str() {
            "hotspot" if !changed_paths.is_empty() => 11.0,
            "failure_pattern" => 8.0,
            "maintainer_profile" => 6.0,
            "reviewer_profile" => 5.0,
            "review_rule" => 7.0,
            "testing_expectation" => 6.0,
            _ => 0.0,
        },
        _ => match entry.kind.as_str() {
            "hotspot" if !changed_paths.is_empty() => 9.0,
            "failure_pattern" => 7.0,
            "testing_expectation" => 6.0,
            "reviewer_profile" => 4.0,
            "maintainer_profile" => 4.0,
            "review_rule" => 4.0,
            _ => 0.0,
        },
    }
}

fn profile_path_bonus(
    entry: &MemoryEntry,
    changed_paths: &[String],
    matched_paths: &[String],
    consumer: &str,
) -> f64 {
    if changed_paths.is_empty() || matched_paths.is_empty() {
        return 0.0;
    }

    let matched_count = matched_paths.len().min(3) as f64;
    let coverage = matched_paths.len() as f64 / changed_paths.len().max(1) as f64;
    let evidence_hits = changed_paths
        .iter()
        .filter(|path| entry_path_focuses_on(entry, path))
        .count()
        .min(3) as f64;

    match (consumer, entry.kind.as_str()) {
        ("trust-gate", "reviewer_profile") => {
            16.0 + (matched_count * 5.0) + (coverage * 10.0) + (evidence_hits * 4.0)
        }
        ("trust-gate", "maintainer_profile") => {
            8.0 + (matched_count * 4.0) + (coverage * 8.0) + (evidence_hits * 3.0)
        }
        ("repo-reaper", "maintainer_profile") => {
            16.0 + (matched_count * 5.0) + (coverage * 10.0) + (evidence_hits * 4.0)
        }
        ("repo-reaper", "reviewer_profile") => {
            8.0 + (matched_count * 4.0) + (coverage * 8.0) + (evidence_hits * 3.0)
        }
        (_, "reviewer_profile" | "maintainer_profile") => {
            10.0 + (matched_count * 4.0) + (coverage * 8.0) + (evidence_hits * 3.0)
        }
        _ => 0.0,
    }
}

fn entry_path_focuses_on(entry: &MemoryEntry, path: &str) -> bool {
    let path_lower = path.to_ascii_lowercase();
    let path_bucket = path_bucket(path).to_ascii_lowercase();

    entry
        .evidence
        .iter()
        .filter_map(|evidence| evidence.path.as_ref())
        .any(|candidate| path_matches_candidate(&path_lower, &path_bucket, candidate))
}

fn path_matches_candidate(path_lower: &str, path_bucket_lower: &str, candidate: &str) -> bool {
    let candidate = candidate.to_ascii_lowercase();
    path_lower == candidate
        || path_lower.starts_with(&(candidate.clone() + "/"))
        || candidate.starts_with(&(path_lower.to_string() + "/"))
        || path_bucket_lower == candidate
}

fn curation_bonus(entry: &MemoryEntry) -> f64 {
    let mut score = 0.0;
    if entry.pinned {
        score += 24.0;
    }
    if normalize_disposition(&entry.disposition) == "policy" {
        score += 18.0;
    }
    score
}

fn disposition_rank(disposition: &str) -> i32 {
    match normalize_disposition(disposition) {
        "policy" => 2,
        "signal" => 1,
        _ => 0,
    }
}

fn build_entry(
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

fn confidence_for(frequency: u32, evidence_count: usize) -> f64 {
    let base = 42.0 + (frequency as f64 * 9.5) + (evidence_count.min(4) as f64 * 4.0);
    base.min(96.0)
}

fn collect_feedback(
    buckets: &mut HashMap<&'static str, SignalBucket>,
    reviewer_profiles: &mut HashMap<String, ReviewerProfileBucket>,
    pr: &GitHubPullRequest,
    path: Option<&str>,
    url: &str,
    body: &str,
    author: Option<&str>,
) -> u32 {
    let mut matched = 0;
    for sentence in split_feedback_sentences(body) {
        let Some((bucket_key, _label)) = classify_feedback(&sentence) else {
            continue;
        };
        let bucket = buckets.entry(bucket_key).or_default();
        bucket.frequency += 1;
        push_evidence(
            &mut bucket.evidence,
            MemoryEvidence {
                source_type: "review_feedback".into(),
                title: format!("#{} {}", pr.number, pr.title),
                url: url.to_string(),
                path: path.map(str::to_string),
                excerpt: if let Some(author) = author {
                    format!("{author}: {}", truncate(&sentence, 180))
                } else {
                    truncate(&sentence, 180)
                },
            },
        );
        if let Some(author) = author.filter(|value| !value.trim().is_empty()) {
            let profile = reviewer_profiles.entry(author.to_string()).or_default();
            profile.total_feedback += 1;
            *profile.category_counts.entry(bucket_key).or_insert(0) += 1;
            if let Some(path) = path {
                *profile.path_counts.entry(path_bucket(path)).or_insert(0) += 1;
            }
            push_evidence(
                &mut profile.evidence,
                MemoryEvidence {
                    source_type: "review_feedback".into(),
                    title: format!("#{} {}", pr.number, pr.title),
                    url: url.to_string(),
                    path: path.map(str::to_string),
                    excerpt: truncate(&sentence, 180),
                },
            );
        }
        matched += 1;
    }
    matched
}

fn review_bucket_specs() -> Vec<(
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Vec<&'static str>,
)> {
    vec![
        (
            "tests",
            "Reviewers repeatedly ask for tests",
            "Add or update tests when behavior changes, bugs are fixed, or risky code is touched.",
            "Repo reviewers regularly ask for stronger test coverage before merge.",
            vec!["tests", "review-feedback"],
        ),
        (
            "helpers",
            "Reviewers prefer existing helpers over one-off logic",
            "Prefer existing helpers, shared utilities, and established abstractions before adding one-off logic.",
            "Review feedback keeps steering changes back toward shared helpers and existing abstractions.",
            vec!["helpers", "conventions"],
        ),
        (
            "validation",
            "Boundary checks and validation matter here",
            "Preserve guard rails, input validation, and edge-case handling around boundaries.",
            "Reviewers repeatedly call out missing validation, guards, or edge-case handling.",
            vec!["validation", "safety"],
        ),
        (
            "naming",
            "Consistency beats clever naming in this repo",
            "Match existing naming, file placement, and structural conventions before inventing a new pattern.",
            "Reviewer feedback keeps reinforcing local naming and structure conventions.",
            vec!["naming", "style"],
        ),
        (
            "docs",
            "Docs and supporting context are expected alongside behavior changes",
            "Keep docs, comments, or README context in sync when interfaces or behavior change.",
            "Recent reviews repeatedly ask for docs, comments, or supporting context updates.",
            vec!["docs", "maintenance"],
        ),
        (
            "errors",
            "Error handling needs context, not just a happy path",
            "Preserve repo error-handling patterns and include context-rich failures or logging where expected.",
            "Reviewer feedback regularly calls out missing context, logging, or error-handling consistency.",
            vec!["errors", "operability"],
        ),
    ]
}

fn classify_feedback(sentence: &str) -> Option<(&'static str, &'static str)> {
    let lower = sentence.to_ascii_lowercase();
    if contains_any(&lower, &["test", "coverage", "assert", "spec"]) {
        return Some(("tests", "tests"));
    }
    if contains_any(
        &lower,
        &["helper", "utility", "shared", "common", "existing", "reuse"],
    ) {
        return Some(("helpers", "helpers"));
    }
    if contains_any(
        &lower,
        &[
            "validate",
            "validation",
            "guard",
            "sanitize",
            "check for",
            "edge case",
        ],
    ) {
        return Some(("validation", "validation"));
    }
    if contains_any(
        &lower,
        &[
            "rename",
            "naming",
            "consistent",
            "convention",
            "style",
            "pattern",
        ],
    ) {
        return Some(("naming", "naming"));
    }
    if contains_any(
        &lower,
        &["readme", "docs", "document", "comment", "changelog"],
    ) {
        return Some(("docs", "docs"));
    }
    if contains_any(
        &lower,
        &["error", "logging", "log ", "context", "fallback", "retry"],
    ) {
        return Some(("errors", "errors"));
    }
    None
}

fn category_label(key: &str) -> String {
    match key {
        "tests" => "tests".into(),
        "helpers" => "shared helpers".into(),
        "validation" => "validation".into(),
        "naming" => "naming and structure".into(),
        "docs" => "docs and supporting context".into(),
        "errors" => "error handling".into(),
        _ => key.to_string(),
    }
}

fn top_named_counts(counts: &HashMap<&'static str, u32>, limit: usize) -> Vec<(String, u32)> {
    let mut items = counts
        .iter()
        .map(|(name, count)| (category_label(name), *count))
        .collect::<Vec<_>>();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    items.truncate(limit);
    items
}

fn top_string_counts(counts: &HashMap<String, u32>, limit: usize) -> Vec<(String, u32)> {
    let mut items = counts
        .iter()
        .map(|(name, count)| (name.clone(), *count))
        .collect::<Vec<_>>();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    items.truncate(limit);
    items
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
            prevention: "Reject public webhook requests when signing configuration is absent."
                .into(),
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

fn normalize_consumer(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_disposition(value: &str) -> &str {
    match value.trim().to_ascii_lowercase().as_str() {
        "policy" => "policy",
        "suppressed" => "suppressed",
        _ => "signal",
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn split_feedback_sentences(body: &str) -> Vec<String> {
    body.replace('\r', "\n")
        .split(['\n', '.', '!', '?'])
        .map(str::trim)
        .filter(|part| part.len() >= 18)
        .map(str::to_string)
        .collect()
}

fn push_evidence(target: &mut Vec<MemoryEvidence>, evidence: MemoryEvidence) {
    if target.len() < 4 {
        target.push(evidence);
    }
}

fn path_bucket(path: &str) -> String {
    let clean = path.trim_matches('/');
    let parts: Vec<_> = clean.split('/').take(2).collect();
    if parts.is_empty() {
        clean.to_string()
    } else {
        parts.join("/")
    }
}

fn is_source_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let source_like = [
        ".rs", ".js", ".jsx", ".ts", ".tsx", ".py", ".go", ".java", ".kt", ".rb", ".php", ".c",
        ".cc", ".cpp", ".h", ".hpp",
    ];
    source_like.iter().any(|ext| lower.ends_with(ext)) && !is_test_file(path)
}

fn is_test_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("/test")
        || lower.contains("/tests/")
        || lower.contains("__tests__")
        || lower.contains(".spec.")
        || lower.contains(".test.")
}

fn looks_bug_like(issue: &GitHubIssue) -> bool {
    let lower = format!(
        "{} {} {}",
        issue.title,
        issue.body.clone().unwrap_or_default(),
        issue
            .labels
            .iter()
            .map(|label| label.name.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    )
    .to_ascii_lowercase();

    contains_any(
        &lower,
        &[
            "bug",
            "regression",
            "panic",
            "crash",
            "timeout",
            "failure",
            "failing",
            "broken",
            "error",
            "race",
            "leak",
        ],
    )
}

fn tokenize(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| part.len() >= 4)
        .collect()
}

fn tokenize_context(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| part.len() >= 3)
        .filter(|part| !STOPWORDS.contains(&part.as_str()))
        .collect()
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let truncated: String = value.chars().take(limit.saturating_sub(1)).collect();
    format!("{truncated}…")
}

fn internal_error(err: impl std::fmt::Display) -> JsonError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": err.to_string() })),
    )
}

fn internal_from_anyhow(err: anyhow::Error) -> JsonError {
    internal_error(err)
}

fn upstream_error(err: impl std::fmt::Display) -> JsonError {
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({ "error": err.to_string() })),
    )
}

fn bad_request(message: &str) -> JsonError {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}

fn not_found(message: &str) -> JsonError {
    (StatusCode::NOT_FOUND, Json(json!({ "error": message })))
}

static STOPWORDS: &[&str] = &[
    "with", "that", "this", "from", "when", "into", "after", "before", "still", "only", "over",
    "have", "more", "than", "they", "them", "then", "their", "there", "should", "could", "would",
    "about", "around", "while", "where", "which", "issue", "issues", "repo", "pull", "request",
    "closed", "merge", "merged", "fails", "failing", "tests", "test", "code", "review",
];

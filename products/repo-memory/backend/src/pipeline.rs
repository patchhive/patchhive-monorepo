use std::collections::{HashMap, HashSet};

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{auth_enabled, generate_and_save_key, verify_token},
    db,
    github,
    models::{
        stable_memory_ref, ContextEntry, ContextRequest, ContextResponse, GitHubIssue,
        GitHubPullFile, GitHubPullRequest, GitHubReview, GitHubReviewComment, HistoryItem,
        IngestParams, IngestRecord, IngestSummary, KnownRepo, MemoryCurationUpdate, MemoryEntry,
        MemoryEvidence, OverviewPayload,
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

pub async fn auth_status() -> Json<serde_json::Value> {
    Json(json!({"auth_enabled": auth_enabled()}))
}

pub async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !verify_token(&body.api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(json!({"ok": true, "auth_enabled": true})))
}

pub async fn gen_key() -> Result<Json<serde_json::Value>, StatusCode> {
    if auth_enabled() {
        return Err(StatusCode::FORBIDDEN);
    }
    let key = generate_and_save_key();
    Ok(Json(json!({"api_key": key, "message": "Store this — it won't be shown again"})))
}

pub async fn health(State(_state): State<AppState>) -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| patchhive_product_core::startup::count_errors(checks))
        .unwrap_or(0);

    Json(json!({
        "status": if errors > 0 { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "RepoMemory by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
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
        return Err(bad_request("RepoMemory expects repos in owner/repo format."));
    }
    if update.memory_ref.is_empty() {
        return Err(bad_request("RepoMemory needs a stable memory_ref to save curation."));
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
        return Err(bad_request("RepoMemory expects repos in owner/repo format."));
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
        prompt_lines: entries.iter().map(|entry| entry.prompt_line.clone()).collect(),
        entries,
    }))
}

pub async fn history(Query(query): Query<HistoryQuery>) -> JsonResult<serde_json::Value> {
    let items: Vec<HistoryItem> = db::list_history(query.repo.as_deref()).map_err(internal_error)?;
    Ok(Json(json!({ "history": items })))
}

pub async fn history_detail(Path(id): Path<String>) -> JsonResult<IngestRecord> {
    match db::get_history(&id).map_err(internal_error)? {
        Some(run) => Ok(Json(run)),
        None => Err(not_found("RepoMemory run not found.")),
    }
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
        return Err(bad_request("RepoMemory expects repos in owner/repo format."));
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

    let issues = github::fetch_closed_issues(&state.http, &params.repo, params.issue_limit, params.since_days)
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
    let mut source_prs = 0u32;
    let mut source_with_tests = 0u32;
    let mut review_feedback_items = 0u32;

    for bundle in &bundles {
        let mut touched_source = false;
        let mut touched_tests = false;

        for file in &bundle.files {
            if is_source_file(&file.filename) {
                touched_source = true;
            }
            if is_test_file(&file.filename) {
                touched_tests = true;
            }

            let bucket = path_bucket(&file.filename);
            *dir_counts.entry(bucket).or_insert(0) += 1;
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
    for (dir, frequency) in hotspot_dirs.into_iter().filter(|(_, count)| *count >= 2).take(3) {
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
    review_paths.sort_by(|left, right| right.1.frequency.cmp(&left.1.frequency).then_with(|| left.0.cmp(&right.0)));
    for (path, bucket) in review_paths.into_iter().filter(|(_, bucket)| bucket.frequency >= 2).take(3) {
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
    bug_terms.sort_by(|left, right| right.1.frequency.cmp(&left.1.frequency).then_with(|| left.0.cmp(&right.0)));
    for (term, bucket) in bug_terms.into_iter().filter(|(_, bucket)| bucket.frequency >= 2).take(4) {
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

    entries.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.frequency.cmp(&left.frequency))
    });

    let summary = build_summary(&entries, bundles.len() as u32, review_feedback_items, issues.len() as u32);
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
            let retrieval_score =
                entry.confidence * 0.48
                + (entry.frequency as f64 * 6.0)
                + (matched_paths.len() as f64 * 18.0)
                + (matched_terms.len() as f64 * 7.0)
                + context_kind_bonus(entry, &clean_paths, consumer)
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
            .then_with(|| disposition_rank(&right.disposition).cmp(&disposition_rank(&left.disposition)))
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
            .any(|candidate| {
                let candidate = candidate.to_ascii_lowercase();
                path_lower == candidate
                    || path_lower.starts_with(&(candidate.clone() + "/"))
                    || candidate.starts_with(&(path_lower.clone() + "/"))
                    || path_bucket == candidate
            });

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
            "review_rule" => 5.0,
            _ => 0.0,
        },
        "repo-reaper" => match entry.kind.as_str() {
            "hotspot" if !changed_paths.is_empty() => 11.0,
            "failure_pattern" => 8.0,
            "review_rule" => 7.0,
            "testing_expectation" => 6.0,
            _ => 0.0,
        },
        _ => match entry.kind.as_str() {
            "hotspot" if !changed_paths.is_empty() => 9.0,
            "failure_pattern" => 7.0,
            "testing_expectation" => 6.0,
            "review_rule" => 4.0,
            _ => 0.0,
        },
    }
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
        matched += 1;
    }
    matched
}

fn review_bucket_specs() -> Vec<(&'static str, &'static str, &'static str, &'static str, Vec<&'static str>)> {
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
    if contains_any(&lower, &["helper", "utility", "shared", "common", "existing", "reuse"]) {
        return Some(("helpers", "helpers"));
    }
    if contains_any(&lower, &["validate", "validation", "guard", "sanitize", "check for", "edge case"]) {
        return Some(("validation", "validation"));
    }
    if contains_any(&lower, &["rename", "naming", "consistent", "convention", "style", "pattern"]) {
        return Some(("naming", "naming"));
    }
    if contains_any(&lower, &["readme", "docs", "document", "comment", "changelog"]) {
        return Some(("docs", "docs"));
    }
    if contains_any(&lower, &["error", "logging", "log ", "context", "fallback", "retry"]) {
        return Some(("errors", "errors"));
    }
    None
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
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

use std::collections::{BTreeSet, HashMap};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use patchhive_product_core::startup::count_errors;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{auth_enabled, generate_and_save_key, verify_token},
    db, github,
    github::GitHubReviewContext,
    models::{
        ChecklistEvidence, ChecklistItem, HistoryItem, OverviewPayload, ReviewMetrics,
        ReviewRequest, ReviewResult,
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

#[derive(Default)]
struct ChecklistCluster {
    category: String,
    title: String,
    open_threads: u32,
    resolved_threads: u32,
    outdated_threads: u32,
    comment_count: u32,
    path_hints: BTreeSet<String>,
    commenter_logins: BTreeSet<String>,
    evidence: Vec<ChecklistEvidence>,
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

pub async fn health() -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let counts = db::overview_counts();

    Json(json!({
        "status": if errors > 0 { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "ReviewBee by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_path": db::db_path(),
        "github_ready": github::github_token_configured(),
        "review_count": counts.reviews,
        "repo_count": counts.repos,
        "open_item_count": counts.open_items,
        "mode": "github-pr-review-checklists",
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

pub async fn history_detail(Path(id): Path<String>) -> JsonResult<ReviewResult> {
    db::get_review(&id)
        .map(Json)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "ReviewBee review not found"))
}

pub async fn review_github_pr(
    State(state): State<AppState>,
    Json(request): Json<ReviewRequest>,
) -> JsonResult<ReviewResult> {
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

    let context = github::fetch_review_context(&state.http, repo, request.pr_number)
        .await
        .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;
    let result = build_review_result(&context);
    db::save_review(&result)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(result))
}

fn api_error(status: StatusCode, error: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": error.into() })))
}

fn build_review_result(context: &GitHubReviewContext) -> ReviewResult {
    let created_at = Utc::now().to_rfc3339();
    let mut reviewer_logins = BTreeSet::new();
    let mut requested_changes_reviews = 0u32;
    let mut approval_reviews = 0u32;
    let mut comment_reviews = 0u32;
    let mut clusters: HashMap<String, ChecklistCluster> = HashMap::new();
    let mut actionable_threads = 0u32;

    for review in &context.reviews {
        if !review.author_login.trim().is_empty() {
            reviewer_logins.insert(review.author_login.trim().to_string());
        }

        match review.state.as_str() {
            "CHANGES_REQUESTED" => requested_changes_reviews += 1,
            "APPROVED" => approval_reviews += 1,
            "COMMENTED" => comment_reviews += 1,
            _ => {}
        }

        if review.state != "APPROVED" && actionable_text(&review.body) {
            let path_hint = "general".to_string();
            let (category, _) = classify_category(&review.body);
            let key = format!("{category}:{path_hint}");
            let cluster = clusters.entry(key).or_default();
            cluster.category = category.into();
            if cluster.title.is_empty() {
                cluster.title = checklist_title(category, &path_hint);
            }
            cluster.open_threads += 1;
            cluster.comment_count += 1;
            if !review.author_login.trim().is_empty() {
                cluster
                    .commenter_logins
                    .insert(review.author_login.trim().to_string());
            }
            push_evidence(
                &mut cluster.evidence,
                ChecklistEvidence {
                    author_login: review.author_login.clone(),
                    url: review.html_url.clone(),
                    path: String::new(),
                    excerpt: truncate(&collapse_whitespace(&review.body), 220),
                    resolved: false,
                    outdated: false,
                },
            );
        }
    }

    for thread in &context.threads {
        let actionable_comments = thread
            .comments
            .iter()
            .filter(|comment| actionable_text(&comment.body))
            .collect::<Vec<_>>();

        if actionable_comments.is_empty() {
            continue;
        }

        actionable_threads += 1;
        let combined = actionable_comments
            .iter()
            .map(|comment| comment.body.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let (category, _) = classify_category(&combined);
        let path_hint = if thread.path.trim().is_empty() {
            "general".into()
        } else {
            path_bucket(&thread.path)
        };
        let key = format!("{category}:{path_hint}");
        let cluster = clusters.entry(key).or_default();
        cluster.category = category.into();
        if cluster.title.is_empty() {
            cluster.title = checklist_title(category, &path_hint);
        }
        if thread.is_resolved {
            cluster.resolved_threads += 1;
        } else {
            cluster.open_threads += 1;
        }
        if thread.is_outdated {
            cluster.outdated_threads += 1;
        }
        if path_hint != "general" {
            cluster.path_hints.insert(path_hint.clone());
        }

        for comment in actionable_comments {
            cluster.comment_count += 1;
            if !comment.author_login.trim().is_empty() {
                reviewer_logins.insert(comment.author_login.trim().to_string());
                cluster
                    .commenter_logins
                    .insert(comment.author_login.trim().to_string());
            }
            push_evidence(
                &mut cluster.evidence,
                ChecklistEvidence {
                    author_login: comment.author_login.clone(),
                    url: comment.url.clone(),
                    path: thread.path.clone(),
                    excerpt: truncate(&collapse_whitespace(&comment.body), 220),
                    resolved: thread.is_resolved,
                    outdated: thread.is_outdated,
                },
            );
        }
    }

    let mut checklist = clusters
        .into_iter()
        .map(|(_, cluster)| {
            let category = cluster.category.clone();
            into_checklist_item(category, cluster)
        })
        .collect::<Vec<_>>();
    checklist.sort_by(|left, right| {
        status_rank(&right.status)
            .cmp(&status_rank(&left.status))
            .then_with(|| right.open_threads.cmp(&left.open_threads))
            .then_with(|| right.comment_count.cmp(&left.comment_count))
            .then_with(|| left.title.cmp(&right.title))
    });

    let open_items = checklist
        .iter()
        .filter(|item| item.status == "open" || item.status == "mixed")
        .count() as u32;
    let resolved_items = checklist
        .iter()
        .filter(|item| item.status == "resolved")
        .count() as u32;
    let metrics = ReviewMetrics {
        review_count: context.reviews.len() as u32,
        requested_changes_reviews,
        approval_reviews,
        comment_reviews,
        thread_count: context.threads.len() as u32,
        actionable_threads,
        open_items,
        resolved_items,
        reviewer_count: reviewer_logins.len() as u32,
    };
    let status = overall_status(&metrics, requested_changes_reviews);
    let summary = build_summary(&metrics, &checklist, &status);
    let prompt_suggestions = checklist
        .iter()
        .filter(|item| item.status != "resolved")
        .map(|item| item.prompt_hint.clone())
        .take(4)
        .collect::<Vec<_>>();

    ReviewResult {
        id: Uuid::new_v4().to_string(),
        repo: context.pr.repo.clone(),
        pr_number: context.pr.number,
        pr_title: context.pr.title.clone(),
        pr_url: context.pr.html_url.clone(),
        status,
        summary,
        created_at,
        metrics,
        reviewers: reviewer_logins.into_iter().collect(),
        prompt_suggestions,
        checklist,
    }
}

fn into_checklist_item(category: String, cluster: ChecklistCluster) -> ChecklistItem {
    let mut path_hints = cluster.path_hints.into_iter().collect::<Vec<_>>();
    path_hints.sort();
    let mut commenter_logins = cluster.commenter_logins.into_iter().collect::<Vec<_>>();
    commenter_logins.sort();
    let status = if cluster.open_threads == 0 {
        "resolved"
    } else if cluster.resolved_threads > 0 || cluster.outdated_threads > 0 {
        "mixed"
    } else {
        "open"
    };
    let prompt_hint = prompt_hint(&category, &path_hints);
    let summary = checklist_summary(
        &cluster.title,
        status,
        cluster.open_threads,
        cluster.resolved_threads,
        cluster.outdated_threads,
        cluster.comment_count,
        &path_hints,
    );

    ChecklistItem {
        key: format!("{}:{}", category, path_hints.join("+")),
        title: cluster.title,
        category,
        status: status.into(),
        summary,
        prompt_hint,
        path_hints,
        commenter_logins,
        open_threads: cluster.open_threads,
        resolved_threads: cluster.resolved_threads,
        outdated_threads: cluster.outdated_threads,
        comment_count: cluster.comment_count,
        evidence: cluster.evidence,
    }
}

fn checklist_summary(
    title: &str,
    status: &str,
    open_threads: u32,
    resolved_threads: u32,
    outdated_threads: u32,
    comment_count: u32,
    path_hints: &[String],
) -> String {
    let mut parts = Vec::new();
    parts.push(match status {
        "resolved" => format!("{title} looks resolved right now."),
        "mixed" => format!("{title} still has some active follow-up."),
        _ => format!("{title} is still blocking clean review closure."),
    });
    parts.push(format!(
        "{} actionable comment{} across {} open thread{}, {} resolved thread{}",
        comment_count,
        plural_suffix(comment_count),
        open_threads,
        plural_suffix(open_threads),
        resolved_threads,
        plural_suffix(resolved_threads),
    ));
    if outdated_threads > 0 {
        parts.push(format!(
            "{} outdated thread{} still shape the context.",
            outdated_threads,
            plural_suffix(outdated_threads)
        ));
    }
    if !path_hints.is_empty() {
        parts.push(format!("Focus areas: {}.", path_hints.join(", ")));
    }
    parts.join(" ")
}

fn overall_status(metrics: &ReviewMetrics, requested_changes_reviews: u32) -> String {
    if metrics.open_items == 0 && metrics.actionable_threads == 0 {
        return "clear".into();
    }
    if metrics.open_items == 0 {
        return "resolved".into();
    }
    if requested_changes_reviews > 0 || metrics.open_items >= 3 {
        return "attention".into();
    }
    "follow-up".into()
}

fn build_summary(metrics: &ReviewMetrics, checklist: &[ChecklistItem], status: &str) -> String {
    if checklist.is_empty() {
        return "ReviewBee did not find actionable review feedback in the current PR threads. This PR looks close to merge from a comment-clustering perspective.".into();
    }

    let mut category_counts = HashMap::new();
    for item in checklist.iter().filter(|item| item.status != "resolved") {
        *category_counts.entry(item.category.as_str()).or_insert(0u32) += 1;
    }
    let mut top_categories = category_counts.into_iter().collect::<Vec<_>>();
    top_categories.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    let top_categories = top_categories
        .into_iter()
        .take(3)
        .map(|(category, _)| category_label(category))
        .collect::<Vec<_>>();

    let status_lead = match status {
        "clear" => "The active review queue looks clear.",
        "resolved" => "Most actionable feedback appears resolved already.",
        "attention" => "This PR still needs focused follow-up before it feels merge-ready.",
        _ => "This PR still has a manageable but real review follow-up list.",
    };

    format!(
        "{} {} open checklist item{} across {} actionable thread{} from {} reviewer{}. Biggest themes: {}.",
        status_lead,
        metrics.open_items,
        plural_suffix(metrics.open_items),
        metrics.actionable_threads,
        plural_suffix(metrics.actionable_threads),
        metrics.reviewer_count,
        plural_suffix(metrics.reviewer_count),
        if top_categories.is_empty() {
            "general review follow-up".into()
        } else {
            top_categories.join(", ")
        }
    )
}

fn checklist_title(category: &str, path_hint: &str) -> String {
    let area = if path_hint == "general" {
        String::new()
    } else {
        format!(" in {path_hint}")
    };

    match category {
        "tests" => format!("Add or adjust tests{area}"),
        "validation" => format!("Strengthen validation and guards{area}"),
        "naming" => format!("Align naming or structure{area}"),
        "docs" => format!("Update docs or supporting context{area}"),
        "cleanup" => format!("Simplify or clean up the implementation{area}"),
        "errors" => format!("Tighten error handling{area}"),
        "api" => format!("Clarify API or behavior contracts{area}"),
        "performance" => format!("Address performance or query concerns{area}"),
        "style" => format!("Smooth out style or consistency concerns{area}"),
        _ => format!("Address remaining review feedback{area}"),
    }
}

fn prompt_hint(category: &str, path_hints: &[String]) -> String {
    let area = if path_hints.is_empty() {
        "the touched code".into()
    } else {
        path_hints.join(", ")
    };

    match category {
        "tests" => format!("Resolve review feedback by adding or tightening tests around {area}."),
        "validation" => format!("Resolve review feedback by restoring validation and guard-rail coverage around {area}."),
        "naming" => format!("Resolve review feedback by matching local naming and structure conventions in {area}."),
        "docs" => format!("Resolve review feedback by syncing docs, comments, or explanatory context in {area}."),
        "cleanup" => format!("Resolve review feedback by simplifying the implementation in {area} and removing one-off logic."),
        "errors" => format!("Resolve review feedback by improving error handling and context in {area}."),
        "api" => format!("Resolve review feedback by clarifying behavior contracts and edge cases in {area}."),
        "performance" => format!("Resolve review feedback by checking the performance impact of changes in {area}."),
        "style" => format!("Resolve review feedback by matching the repo’s style expectations in {area}."),
        _ => format!("Resolve the remaining review feedback in {area} before merge."),
    }
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
        "great catch",
        "thanks",
        "thank you",
        "awesome",
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

fn classify_category(text: &str) -> (&'static str, &'static str) {
    let lower = text.to_ascii_lowercase();

    if contains_any(&lower, &["test", "coverage", "assert", "spec", "fixture"]) {
        ("tests", "Tests")
    } else if contains_any(&lower, &["validate", "validation", "guard", "sanitize", "edge case", "null", "nil"]) {
        ("validation", "Validation")
    } else if contains_any(&lower, &["rename", "naming", "consistent", "convention", "structure", "pattern"]) {
        ("naming", "Naming")
    } else if contains_any(&lower, &["readme", "docs", "document", "comment", "explain"]) {
        ("docs", "Docs")
    } else if contains_any(&lower, &["refactor", "simplify", "clean up", "cleanup", "helper", "reuse"]) {
        ("cleanup", "Cleanup")
    } else if contains_any(&lower, &["error", "logging", "log ", "fallback", "retry", "panic"]) {
        ("errors", "Error handling")
    } else if contains_any(&lower, &["api", "contract", "behavior", "return", "response", "interface"]) {
        ("api", "API behavior")
    } else if contains_any(&lower, &["slow", "performance", "query", "n+1", "efficient"]) {
        ("performance", "Performance")
    } else if contains_any(&lower, &["style", "format", "lint", "indent", "spacing"]) {
        ("style", "Style")
    } else {
        ("general", "General")
    }
}

fn category_label(category: &str) -> &'static str {
    match category {
        "tests" => "tests",
        "validation" => "validation",
        "naming" => "naming / structure",
        "docs" => "docs",
        "cleanup" => "cleanup",
        "errors" => "error handling",
        "api" => "API behavior",
        "performance" => "performance",
        "style" => "style",
        _ => "general follow-up",
    }
}

fn path_bucket(path: &str) -> String {
    let trimmed = path.trim().trim_start_matches("./");
    if trimmed.is_empty() {
        return "general".into();
    }

    let parts = trimmed.split('/').collect::<Vec<_>>();
    if parts.len() >= 2
        && matches!(
            parts[0],
            "src" | "app" | "lib" | "server" | "backend" | "frontend" | "tests" | "packages"
        )
    {
        return format!("{}/{}", parts[0], parts[1]);
    }

    parts.first().copied().unwrap_or(trimmed).to_string()
}

fn status_rank(status: &str) -> i32 {
    match status {
        "open" => 3,
        "mixed" => 2,
        "resolved" => 1,
        _ => 0,
    }
}

fn plural_suffix(count: u32) -> &'static str {
    if count == 1 { "" } else { "s" }
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
        compact.chars().take(limit.saturating_sub(1)).collect::<String>() + "…"
    }
}

fn push_evidence(items: &mut Vec<ChecklistEvidence>, evidence: ChecklistEvidence) {
    if items
        .iter()
        .any(|existing| existing.url == evidence.url && existing.excerpt == evidence.excerpt)
    {
        return;
    }
    if items.len() < 5 {
        items.push(evidence);
    }
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

    #[test]
    fn actionability_filters_praise_but_keeps_requests() {
        assert!(!actionable_text("LGTM, nice work."));
        assert!(actionable_text("Could you add a regression test for this path?"));
    }

    #[test]
    fn path_bucket_keeps_useful_area_context() {
        assert_eq!(path_bucket("src/reaper/fix_worker.rs"), "src/reaper");
        assert_eq!(path_bucket("docs/guide.md"), "docs");
    }
}

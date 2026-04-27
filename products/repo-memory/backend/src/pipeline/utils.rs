// utils.rs - Shared utility functions for the pipeline module

use serde_json::json;

pub fn normalize_consumer(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub fn normalize_disposition(value: &str) -> &str {
    match value.trim().to_ascii_lowercase().as_str() {
        "policy" => "policy",
        "suppressed" => "suppressed",
        _ => "signal",
    }
}

pub fn normalize_candidate_status(value: &str) -> &str {
    match value.trim().to_ascii_lowercase().as_str() {
        "dismissed" => "dismissed",
        "promoted" => "promoted",
        _ => "open",
    }
}

pub fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

pub fn split_feedback_sentences(body: &str) -> Vec<String> {
    body.replace('\r', "\n")
        .split(['\n', '.', '!', '?'])
        .map(str::trim)
        .filter(|part| part.len() >= 18)
        .map(str::to_string)
        .collect()
}

pub fn push_evidence(target: &mut Vec<crate::models::MemoryEvidence>, evidence: crate::models::MemoryEvidence) {
    if target.len() < 4 {
        target.push(evidence);
    }
}

pub fn path_bucket(path: &str) -> String {
    let clean = path.trim_matches('/');
    let parts: Vec<_> = clean.split('/').take(2).collect();
    if parts.is_empty() {
        clean.to_string()
    } else {
        parts.join("/")
    }
}

pub fn is_source_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let source_like = [
        ".rs", ".js", ".jsx", ".ts", ".tsx", ".py", ".go", ".java", ".kt", ".rb", ".php", ".c",
        ".cc", ".cpp", ".h", ".hpp",
    ];
    source_like.iter().any(|ext| lower.ends_with(ext)) && !is_test_file(path)
}

pub fn is_test_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("/test")
        || lower.contains("/tests/")
        || lower.contains("__tests__")
        || lower.contains(".spec.")
        || lower.contains(".test.")
}

pub fn looks_bug_like(issue: &crate::models::GitHubIssue) -> bool {
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
            "bug", "regression", "panic", "crash", "timeout", "failure", "failing",
            "broken", "error", "race", "leak",
        ],
    )
}

pub fn tokenize(text: &str) -> std::collections::HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| part.len() >= 4)
        .collect()
}

pub fn tokenize_context(text: &str) -> std::collections::HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| part.len() >= 3)
        .filter(|part| !STOPWORDS.contains(&part.as_str()))
        .collect()
}

pub fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let truncated: String = value.chars().take(limit.saturating_sub(1)).collect();
    format!("{truncated}…")
}

pub fn internal_error(err: impl std::fmt::Display) -> super::JsonError {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        axum::Json(json!({ "error": err.to_string() })),
    )
}

pub fn internal_from_anyhow(err: anyhow::Error) -> super::JsonError {
    internal_error(err)
}

pub fn upstream_error(err: impl std::fmt::Display) -> super::JsonError {
    (
        axum::http::StatusCode::BAD_GATEWAY,
        axum::Json(json!({ "error": err.to_string() })),
    )
}

pub fn bad_request(message: &str) -> super::JsonError {
    (
        axum::http::StatusCode::BAD_REQUEST,
        axum::Json(json!({ "error": message })),
    )
}

pub fn not_found(message: &str) -> super::JsonError {
    (
        axum::http::StatusCode::NOT_FOUND,
        axum::Json(json!({ "error": message })),
    )
}

pub fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

pub const STOPWORDS: &[&str] = &[
    "with", "that", "this", "from", "when", "into", "after", "before", "still", "only", "over",
    "have", "more", "than", "they", "them", "then", "their", "there", "should", "could",
    "would", "about", "around", "while", "where", "which", "issue", "issues", "repo", "pull",
    "request", "closed", "merge", "merged", "fails", "failing", "tests", "test", "code",
    "review",
];

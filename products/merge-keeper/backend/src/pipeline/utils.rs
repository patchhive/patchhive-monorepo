use axum::{
    http::StatusCode,
    Json,
};
use serde_json::json;

use crate::models::MergeSignal;

pub type ApiError = (StatusCode, Json<serde_json::Value>);

pub fn api_error(status: StatusCode, error: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": error.into() })))
}

pub fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

pub fn make_signal(
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

pub fn mergeable_value(value: Option<bool>) -> String {
    match value {
        Some(true) => "yes".into(),
        Some(false) => "no".into(),
        None => "unknown".into(),
    }
}

pub fn normalize_mergeable_state(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "unknown".into()
    } else {
        trimmed.into()
    }
}

pub fn actionable_text(text: &str) -> bool {
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

pub fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

pub fn truncate(value: &str, limit: usize) -> String {
    let compact = collapse_whitespace(value);
    if compact.chars().count() <= limit {
        compact
    } else {
        compact
            .chars()
            .take(limit.saturating_sub(1))
            .collect::<String>()
            + "\u{2026}"
    }
}

pub fn plural_suffix(count: u32) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

pub fn diff_changed_paths(diff: &str) -> Vec<String> {
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

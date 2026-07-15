// utils.rs — Small shared helpers

use axum::{http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde_json::json;

use crate::models::{GitHubIssue, IssueSample, ScanParams};

pub fn bad_request(message: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": message.into() })),
    )
}

pub fn internal_error(err: anyhow::Error) -> (StatusCode, Json<serde_json::Value>) {
    tracing::error!("signal-hive error: {err:?}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": err.to_string() })),
    )
}

pub fn clamp_params(mut params: ScanParams) -> ScanParams {
    if params.max_repos == 0 {
        params.max_repos = 8;
    }
    if params.issues_per_repo == 0 {
        params.issues_per_repo = 30;
    }
    if params.stale_days == 0 {
        params.stale_days = 45;
    }
    if params.min_stars == 0 {
        params.min_stars = 25;
    }
    params.max_repos = params.max_repos.min(25);
    params.issues_per_repo = params.issues_per_repo.clamp(5, 100);
    params.stale_days = params.stale_days.min(730);
    params.min_stars = params.min_stars.min(1_000_000);
    params.topics = params
        .topics
        .into_iter()
        .map(|topic| topic.trim().to_string())
        .filter(|topic| !topic.is_empty())
        .collect();
    params.languages = params
        .languages
        .into_iter()
        .map(|language| language.trim().to_string())
        .filter(|language| !language.is_empty())
        .collect();
    params.search_query = params.search_query.trim().to_string();
    params
}

pub fn validate_scan_scope(
    params: &ScanParams,
    allowlist_configured: bool,
) -> Result<(), &'static str> {
    if params.search_query.starts_with("repo:") && params.direct_repository().is_none() {
        return Err("SignalHive direct targets must use owner/repository format.");
    }
    if params.search_query.is_empty()
        && params.topics.is_empty()
        && params.languages.is_empty()
        && !allowlist_configured
    {
        return Err(
            "Provide a target repository, search query, topic, or language, or configure an allowlist.",
        );
    }
    Ok(())
}

pub fn issue_age_days(updated_at: &str) -> i64 {
    DateTime::parse_from_rfc3339(updated_at)
        .ok()
        .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_days())
        .unwrap_or_default()
}

pub fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

pub fn format_scope_text(params: &ScanParams) -> String {
    if let Some(repository) = params.direct_repository() {
        return format!("direct repository `{repository}`");
    }

    let query = if params.search_query.is_empty() {
        "no search query filter".to_string()
    } else {
        format!("search query `{}`", params.search_query)
    };
    let topics = if params.topics.is_empty() {
        "no topic filter".to_string()
    } else {
        format!("topics `{}`", params.topics.join(", "))
    };
    let languages = if params.languages.is_empty() {
        "all languages".to_string()
    } else {
        format!("languages `{}`", params.languages.join(", "))
    };

    format!("{query} · {topics} · {languages}")
}

pub fn marker_total(
    todo_count: u32,
    fixme_count: u32,
    todo_available: bool,
    fixme_available: bool,
) -> u32 {
    let mut total = 0;
    if todo_available {
        total += todo_count;
    }
    if fixme_available {
        total += fixme_count;
    }
    total
}

pub fn marker_scan_repo_limit() -> usize {
    std::env::var("SIGNAL_MARKER_REPO_LIMIT")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(4)
}

pub fn title_tokens(title: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "and", "or", "for", "with", "from", "into", "after", "before", "that",
        "this", "when", "then", "have", "has", "had", "not", "are", "can", "its", "was", "were",
        "issue", "issues", "help", "support", "question",
    ];

    let mut seen = std::collections::HashSet::new();
    let mut tokens = Vec::new();
    for part in title.split(|ch: char| !ch.is_alphanumeric()) {
        let token = part.trim().to_lowercase();
        if token.len() <= 2 || STOP.contains(&token.as_str()) || !seen.insert(token.clone()) {
            continue;
        }
        tokens.push(token);
    }
    tokens
}

pub fn tokenize_title(title: &str) -> std::collections::HashSet<String> {
    title_tokens(title).into_iter().collect()
}

pub fn recurring_bug_tokens(title: &str) -> Vec<String> {
    const IGNORE: &[&str] = &[
        "bug",
        "bugs",
        "error",
        "errors",
        "panic",
        "panics",
        "crash",
        "crashes",
        "broken",
        "failure",
        "failures",
        "failing",
        "fails",
        "fail",
        "regression",
        "regressions",
        "unexpected",
        "incorrect",
        "wrong",
    ];

    title_tokens(title)
        .into_iter()
        .filter(|token| !IGNORE.contains(&token.as_str()))
        .collect()
}

pub fn to_issue_sample(issue: &GitHubIssue) -> IssueSample {
    IssueSample {
        number: issue.number,
        title: issue.title.clone(),
        url: issue.html_url.clone(),
        updated_at: issue.updated_at.clone(),
        age_days: issue_age_days(&issue.updated_at),
        comments: issue.comments,
    }
}

pub fn label_names(issue: &GitHubIssue) -> Vec<String> {
    issue
        .labels
        .iter()
        .map(|label| label.name.trim().to_lowercase())
        .filter(|label| !label.is_empty())
        .collect()
}

pub fn title_has_bug_hint(title: &str) -> bool {
    let lower = title.to_lowercase();
    [
        "bug",
        "regression",
        "panic",
        "crash",
        "error",
        "broken",
        "fails",
        "failing",
    ]
    .iter()
    .any(|hint| lower.contains(hint))
}

pub fn is_bug_issue(issue: &GitHubIssue) -> bool {
    label_names(issue).iter().any(|label| {
        label.contains("bug")
            || label.contains("regression")
            || label.contains("defect")
            || label.contains("failure")
            || label.contains("crash")
            || label.contains("panic")
            || label.contains("error")
    }) || title_has_bug_hint(&issue.title)
}

pub fn recurring_issue_count(clusters: &[crate::models::RecurringBugCluster]) -> i32 {
    clusters
        .iter()
        .map(|cluster| cluster.issue_count as i32)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{clamp_params, format_scope_text, validate_scan_scope};
    use crate::models::ScanParams;

    #[test]
    fn server_bounds_scan_parameters_even_outside_the_frontend() {
        let params = clamp_params(ScanParams {
            min_stars: u32::MAX,
            max_repos: u32::MAX,
            issues_per_repo: u32::MAX,
            stale_days: u32::MAX,
            ..ScanParams::default()
        });

        assert_eq!(params.min_stars, 1_000_000);
        assert_eq!(params.max_repos, 25);
        assert_eq!(params.issues_per_repo, 100);
        assert_eq!(params.stale_days, 730);
    }

    #[test]
    fn direct_scopes_are_validated_and_described_explicitly() {
        let direct = ScanParams {
            search_query: "repo:PatchHive/patchhive".into(),
            ..ScanParams::default()
        };
        assert!(validate_scan_scope(&direct, false).is_ok());
        assert_eq!(
            format_scope_text(&direct),
            "direct repository `patchhive/patchhive`"
        );

        let malformed = ScanParams {
            search_query: "repo:not-a-repository".into(),
            ..ScanParams::default()
        };
        assert!(validate_scan_scope(&malformed, false).is_err());
    }
}

use std::collections::HashSet;

use anyhow::Result;
use axum::{extract::{Path, State}, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde_json::json;

use crate::{
    db,
    github,
    models::{DuplicateCandidate, GitHubIssue, IssueSample, RepoSignal, ScanParams},
    state::AppState,
};

fn bad_request(message: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message.into() })))
}

fn internal_error(err: anyhow::Error) -> (StatusCode, Json<serde_json::Value>) {
    tracing::error!("signal-hive error: {err:?}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": err.to_string() })),
    )
}

fn clamp_params(mut params: ScanParams) -> ScanParams {
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

fn issue_age_days(updated_at: &str) -> i64 {
    DateTime::parse_from_rfc3339(updated_at)
        .ok()
        .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_days())
        .unwrap_or_default()
}

fn tokenize_title(title: &str) -> HashSet<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "and", "or", "for", "with", "from", "into", "after", "before", "that",
        "this", "when", "then", "have", "has", "had", "not", "are", "can", "its", "was", "were",
    ];

    title
        .split(|ch: char| !ch.is_alphanumeric())
        .map(|part| part.trim().to_lowercase())
        .filter(|part| part.len() > 2 && !STOP.contains(&part.as_str()))
        .collect()
}

fn duplicate_candidates(issues: &[GitHubIssue]) -> Vec<DuplicateCandidate> {
    let mut pairs = Vec::new();

    for left_index in 0..issues.len() {
        let left = &issues[left_index];
        let left_tokens = tokenize_title(&left.title);
        if left_tokens.is_empty() {
            continue;
        }

        for right in issues.iter().skip(left_index + 1) {
            let right_tokens = tokenize_title(&right.title);
            if right_tokens.is_empty() {
                continue;
            }

            let union = left_tokens.union(&right_tokens).count() as f64;
            let shared = left_tokens.intersection(&right_tokens).count() as f64;
            if union == 0.0 || shared < 2.0 {
                continue;
            }

            let similarity = shared / union;
            if similarity >= 0.55 {
                pairs.push(DuplicateCandidate {
                    left_number: left.number,
                    right_number: right.number,
                    left_title: left.title.clone(),
                    right_title: right.title.clone(),
                    similarity: (similarity * 100.0).round() / 100.0,
                });
            }
        }
    }

    pairs.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
    pairs.truncate(3);
    pairs
}

fn stale_issue_examples(issues: &[GitHubIssue], stale_days: u32) -> Vec<IssueSample> {
    let mut examples = issues
        .iter()
        .filter_map(|issue| {
            let age_days = issue_age_days(&issue.updated_at);
            if age_days < stale_days as i64 {
                return None;
            }

            Some(IssueSample {
                number: issue.number,
                title: issue.title.clone(),
                url: issue.html_url.clone(),
                updated_at: issue.updated_at.clone(),
                age_days,
                comments: issue.comments,
            })
        })
        .collect::<Vec<_>>();

    examples.sort_by(|a, b| b.age_days.cmp(&a.age_days));
    examples.truncate(3);
    examples
}

fn issue_signals(issues: &[GitHubIssue], stale_days: u32) -> (u32, Vec<IssueSample>, Vec<DuplicateCandidate>) {
    let stale_issue_count = issues
        .iter()
        .filter(|issue| issue_age_days(&issue.updated_at) >= stale_days as i64)
        .count() as u32;
    let samples = stale_issue_examples(issues, stale_days);
    let duplicates = duplicate_candidates(issues);
    (stale_issue_count, samples, duplicates)
}

fn priority_score(
    open_issues: u32,
    stale_issues: u32,
    duplicate_count: usize,
    todo_count: u32,
    fixme_count: u32,
) -> f64 {
    let mut score = 0.0;
    score += stale_issues as f64 * 7.5;
    score += duplicate_count as f64 * 18.0;
    score += todo_count.min(25) as f64 * 1.2;
    score += fixme_count.min(25) as f64 * 1.6;
    score += open_issues.min(50) as f64 * 0.35;
    if stale_issues >= 3 {
        score += 10.0;
    }
    if duplicate_count >= 2 {
        score += 8.0;
    }
    score.min(100.0)
}

fn summary_from_signals(
    stale_issues: u32,
    duplicate_count: usize,
    todo_count: u32,
    fixme_count: u32,
) -> (String, Vec<String>) {
    let mut signals = Vec::new();

    if stale_issues > 0 {
        signals.push(format!("{stale_issues} stale issues need triage"));
    }
    if duplicate_count > 0 {
        signals.push(format!("{duplicate_count} likely duplicate issue pairs"));
    }
    if todo_count > 0 {
        signals.push(format!("{todo_count} TODO markers found"));
    }
    if fixme_count > 0 {
        signals.push(format!("{fixme_count} FIXME markers found"));
    }
    if signals.is_empty() {
        signals.push("No major maintenance signals found in the current sample".into());
    }

    let summary = signals.iter().take(2).cloned().collect::<Vec<_>>().join(" · ");
    (summary, signals)
}

async fn analyze_repo(
    client: &reqwest::Client,
    repo: &crate::models::SearchRepo,
    params: &ScanParams,
) -> Result<RepoSignal> {
    let issues = github::fetch_open_issues(client, &repo.owner.login, &repo.name, params.issues_per_repo).await?;
    let (stale_issues, issue_examples, duplicate_candidates) = issue_signals(&issues, params.stale_days);
    let todo_count = github::search_code_marker(client, &repo.full_name, "TODO").await;
    let fixme_count = github::search_code_marker(client, &repo.full_name, "FIXME").await;
    let priority_score = priority_score(
        repo.open_issues_count,
        stale_issues,
        duplicate_candidates.len(),
        todo_count,
        fixme_count,
    );
    let (summary, signals) = summary_from_signals(
        stale_issues,
        duplicate_candidates.len(),
        todo_count,
        fixme_count,
    );

    Ok(RepoSignal {
        full_name: repo.full_name.clone(),
        repo_url: repo.html_url.clone(),
        description: repo.description.clone().unwrap_or_default(),
        language: repo.language.clone().unwrap_or_else(|| "unknown".into()),
        stars: repo.stargazers_count,
        open_issues: repo.open_issues_count,
        stale_issues,
        duplicate_candidates,
        todo_count,
        fixme_count,
        priority_score: (priority_score * 10.0).round() / 10.0,
        summary,
        signals,
        issue_examples,
    })
}

pub async fn scan(
    State(state): State<AppState>,
    Json(params): Json<ScanParams>,
) -> Result<Json<crate::models::ScanRecord>, (StatusCode, Json<serde_json::Value>)> {
    let params = clamp_params(params);
    if params.search_query.is_empty() && params.topics.is_empty() && params.languages.is_empty() {
        return Err(bad_request("Provide at least a search query, topic, or language."));
    }

    let repos = github::discover_repositories(&state.http, &params)
        .await
        .map_err(internal_error)?;

    let mut signals = Vec::new();
    for repo in repos {
        match analyze_repo(&state.http, &repo, &params).await {
            Ok(signal) => signals.push(signal),
            Err(err) => tracing::warn!("failed to analyze {}: {err}", repo.full_name),
        }
    }

    signals.sort_by(|a, b| {
        b.priority_score
            .partial_cmp(&a.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.stars.cmp(&a.stars))
    });

    let record = db::save_scan(&params, &signals).map_err(internal_error)?;
    Ok(Json(record))
}

pub async fn history() -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let scans = db::list_scans().map_err(internal_error)?;
    Ok(Json(json!({ "scans": scans })))
}

pub async fn history_detail(
    Path(id): Path<String>,
) -> Result<Json<crate::models::ScanRecord>, (StatusCode, Json<serde_json::Value>)> {
    match db::get_scan(&id).map_err(internal_error)? {
        Some(scan) => Ok(Json(scan)),
        None => Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Scan not found" })))),
    }
}

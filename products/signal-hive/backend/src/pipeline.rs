use std::collections::HashSet;

use anyhow::Result;
use axum::{extract::{Path, State}, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde_json::json;

use crate::{
    db,
    github,
    models::{
        DuplicateCandidate, GitHubIssue, IssueSample, RecurringBugCluster, RepoSignal, ScanParams,
        ScoreFactor,
    },
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

struct IssueAnalysis {
    sampled_issues: u32,
    stale_issues: u32,
    unlabeled_issues: u32,
    stale_bug_issues: u32,
    stale_high_comment_issues: u32,
    issue_examples: Vec<IssueSample>,
    duplicate_candidates: Vec<DuplicateCandidate>,
    recurring_bug_clusters: Vec<RecurringBugCluster>,
}

fn issue_age_days(updated_at: &str) -> i64 {
    DateTime::parse_from_rfc3339(updated_at)
        .ok()
        .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_days())
        .unwrap_or_default()
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn title_tokens(title: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "and", "or", "for", "with", "from", "into", "after", "before", "that",
        "this", "when", "then", "have", "has", "had", "not", "are", "can", "its", "was", "were",
        "issue", "issues", "help", "support", "question",
    ];

    let mut seen = HashSet::new();
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

fn tokenize_title(title: &str) -> HashSet<String> {
    title_tokens(title).into_iter().collect()
}

fn recurring_bug_tokens(title: &str) -> Vec<String> {
    const IGNORE: &[&str] = &[
        "bug", "bugs", "error", "errors", "panic", "panics", "crash", "crashes", "broken",
        "failure", "failures", "failing", "fails", "fail", "regression", "regressions",
        "unexpected", "incorrect", "wrong",
    ];

    title_tokens(title)
        .into_iter()
        .filter(|token| !IGNORE.contains(&token.as_str()))
        .collect()
}

fn to_issue_sample(issue: &GitHubIssue) -> IssueSample {
    IssueSample {
        number: issue.number,
        title: issue.title.clone(),
        url: issue.html_url.clone(),
        updated_at: issue.updated_at.clone(),
        age_days: issue_age_days(&issue.updated_at),
        comments: issue.comments,
    }
}

fn label_names(issue: &GitHubIssue) -> Vec<String> {
    issue
        .labels
        .iter()
        .map(|label| label.name.trim().to_lowercase())
        .filter(|label| !label.is_empty())
        .collect()
}

fn title_has_bug_hint(title: &str) -> bool {
    let lower = title.to_lowercase();
    ["bug", "regression", "panic", "crash", "error", "broken", "fails", "failing"]
        .iter()
        .any(|hint| lower.contains(hint))
}

fn is_bug_issue(issue: &GitHubIssue) -> bool {
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

fn duplicate_candidates(issues: &[GitHubIssue]) -> Vec<DuplicateCandidate> {
    let mut pairs = Vec::new();

    for left_index in 0..issues.len() {
        let left = &issues[left_index];
        let left_tokens = tokenize_title(&left.title);
        let left_phrase = title_tokens(&left.title).join(" ");
        if left_tokens.is_empty() {
            continue;
        }

        for right in issues.iter().skip(left_index + 1) {
            let right_tokens = tokenize_title(&right.title);
            let right_phrase = title_tokens(&right.title).join(" ");
            if right_tokens.is_empty() {
                continue;
            }

            let union = left_tokens.union(&right_tokens).count() as f64;
            let shared = left_tokens.intersection(&right_tokens).count() as f64;
            if union == 0.0 || shared < 2.0 {
                continue;
            }

            let contains_match = left_phrase.len() > 10
                && right_phrase.len() > 10
                && (left_phrase.contains(&right_phrase) || right_phrase.contains(&left_phrase));

            let mut similarity = shared / union;
            if contains_match {
                similarity = similarity.max(0.78);
            }

            if similarity >= 0.58 {
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

fn recurring_bug_clusters(issues: &[GitHubIssue]) -> Vec<RecurringBugCluster> {
    let bug_issues = issues
        .iter()
        .filter(|issue| is_bug_issue(issue))
        .collect::<Vec<_>>();

    if bug_issues.len() < 2 {
        return Vec::new();
    }

    let token_sets = bug_issues
        .iter()
        .map(|issue| recurring_bug_tokens(&issue.title).into_iter().collect::<HashSet<_>>())
        .collect::<Vec<_>>();

    let mut adjacency = vec![Vec::<usize>::new(); bug_issues.len()];
    for left_index in 0..bug_issues.len() {
        if token_sets[left_index].is_empty() {
            continue;
        }

        for right_index in (left_index + 1)..bug_issues.len() {
            if token_sets[right_index].is_empty() {
                continue;
            }

            let union = token_sets[left_index].union(&token_sets[right_index]).count() as f64;
            let shared = token_sets[left_index]
                .intersection(&token_sets[right_index])
                .count() as f64;

            if union == 0.0 {
                continue;
            }

            let similarity = shared / union;
            if shared >= 2.0 && similarity >= 0.34 {
                adjacency[left_index].push(right_index);
                adjacency[right_index].push(left_index);
            }
        }
    }

    let mut visited = vec![false; bug_issues.len()];
    let mut clusters = Vec::new();

    for start in 0..bug_issues.len() {
        if visited[start] {
            continue;
        }

        let mut stack = vec![start];
        let mut component = Vec::new();
        while let Some(index) = stack.pop() {
            if visited[index] {
                continue;
            }
            visited[index] = true;
            component.push(index);
            for neighbor in &adjacency[index] {
                if !visited[*neighbor] {
                    stack.push(*neighbor);
                }
            }
        }

        if component.len() < 2 {
            continue;
        }

        let mut term_counts = std::collections::HashMap::<String, u32>::new();
        let mut samples = component
            .iter()
            .map(|index| {
                for token in &token_sets[*index] {
                    *term_counts.entry(token.clone()).or_insert(0) += 1;
                }
                to_issue_sample(bug_issues[*index])
            })
            .collect::<Vec<_>>();

        let mut shared_terms = term_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .collect::<Vec<_>>();
        shared_terms.sort_by(|(left_term, left_count), (right_term, right_count)| {
            right_count.cmp(left_count).then_with(|| left_term.cmp(right_term))
        });

        let shared_terms = shared_terms
            .into_iter()
            .map(|(term, _)| term)
            .take(3)
            .collect::<Vec<_>>();

        samples.sort_by(|left, right| {
            right
                .comments
                .cmp(&left.comments)
                .then_with(|| right.age_days.cmp(&left.age_days))
        });

        let label = if shared_terms.is_empty() {
            "Repeated bug pattern".to_string()
        } else {
            shared_terms.join(" / ")
        };

        clusters.push(RecurringBugCluster {
            label,
            issue_count: component.len() as u32,
            shared_terms,
            examples: samples.into_iter().take(3).collect(),
        });
    }

    clusters.sort_by(|left, right| {
        right
            .issue_count
            .cmp(&left.issue_count)
            .then_with(|| right.examples.first().map(|example| example.comments).unwrap_or(0).cmp(
                &left.examples.first().map(|example| example.comments).unwrap_or(0),
            ))
    });
    clusters.truncate(3);
    clusters
}

fn stale_issue_examples(issues: &[GitHubIssue], stale_days: u32) -> Vec<IssueSample> {
    let mut examples = issues
        .iter()
        .filter_map(|issue| {
            let age_days = issue_age_days(&issue.updated_at);
            if age_days < stale_days as i64 {
                return None;
            }

            Some(IssueSample { age_days, ..to_issue_sample(issue) })
        })
        .collect::<Vec<_>>();

    examples.sort_by(|a, b| b.age_days.cmp(&a.age_days).then_with(|| b.comments.cmp(&a.comments)));
    examples.truncate(3);
    examples
}

fn issue_signals(issues: &[GitHubIssue], stale_days: u32) -> IssueAnalysis {
    let sampled_issues = issues.len() as u32;
    let unlabeled_issues = issues.iter().filter(|issue| issue.labels.is_empty()).count() as u32;
    let stale_issues = issues
        .iter()
        .filter(|issue| issue_age_days(&issue.updated_at) >= stale_days as i64)
        .count() as u32;
    let stale_bug_issues = issues
        .iter()
        .filter(|issue| issue_age_days(&issue.updated_at) >= stale_days as i64 && is_bug_issue(issue))
        .count() as u32;
    let stale_high_comment_issues = issues
        .iter()
        .filter(|issue| issue_age_days(&issue.updated_at) >= stale_days as i64 && issue.comments >= 3)
        .count() as u32;

    IssueAnalysis {
        sampled_issues,
        stale_issues,
        unlabeled_issues,
        stale_bug_issues,
        stale_high_comment_issues,
        issue_examples: stale_issue_examples(issues, stale_days),
        duplicate_candidates: duplicate_candidates(issues),
        recurring_bug_clusters: recurring_bug_clusters(issues),
    }
}

fn priority_score(
    stars: u32,
    open_issues: u32,
    issue_analysis: &IssueAnalysis,
    todo_count: u32,
    fixme_count: u32,
) -> (f64, Vec<ScoreFactor>) {
    let sampled = issue_analysis.sampled_issues.max(1) as f64;
    let stale_ratio = issue_analysis.stale_issues as f64 / sampled;
    let unlabeled_ratio = issue_analysis.unlabeled_issues as f64 / sampled;
    let issue_density = (open_issues as f64 / stars.max(25) as f64) * 100.0;
    let duplicate_pressure = issue_analysis
        .duplicate_candidates
        .iter()
        .map(|pair| pair.similarity)
        .sum::<f64>();
    let recurring_bug_issue_count = issue_analysis
        .recurring_bug_clusters
        .iter()
        .map(|cluster| cluster.issue_count)
        .sum::<u32>();

    let mut breakdown = Vec::new();

    let stale_backlog_impact =
        (stale_ratio * 34.0).min(24.0) + (issue_analysis.stale_issues.min(6) as f64 * 2.2).min(12.0);
    if issue_analysis.stale_issues > 0 {
        breakdown.push(ScoreFactor {
            key: "stale_backlog".into(),
            label: "Stale backlog".into(),
            impact: round1(stale_backlog_impact),
            detail: format!(
                "{} of {} sampled issues are stale",
                issue_analysis.stale_issues, issue_analysis.sampled_issues
            ),
        });
    }

    let stale_bug_impact = (issue_analysis.stale_bug_issues.min(3) as f64 * 7.5).min(18.0);
    if issue_analysis.stale_bug_issues > 0 {
        breakdown.push(ScoreFactor {
            key: "stale_bug".into(),
            label: "Stale bug pressure".into(),
            impact: round1(stale_bug_impact),
            detail: format!(
                "{} stale bug-like issues are still open",
                issue_analysis.stale_bug_issues
            ),
        });
    }

    let stalled_discussion_impact = (issue_analysis.stale_high_comment_issues.min(3) as f64 * 4.8).min(14.4);
    if issue_analysis.stale_high_comment_issues > 0 {
        breakdown.push(ScoreFactor {
            key: "stalled_discussion".into(),
            label: "Stalled discussions".into(),
            impact: round1(stalled_discussion_impact),
            detail: format!(
                "{} stale issues still have active discussion",
                issue_analysis.stale_high_comment_issues
            ),
        });
    }

    let recurring_bug_impact = ((recurring_bug_issue_count.min(6) as f64) * 2.8
        + issue_analysis.recurring_bug_clusters.len() as f64 * 3.5)
        .min(18.0);
    if !issue_analysis.recurring_bug_clusters.is_empty() {
        let strongest = issue_analysis
            .recurring_bug_clusters
            .first()
            .map(|cluster| format!("top pattern '{}' appears in {} issues", cluster.label, cluster.issue_count))
            .unwrap_or_else(|| "bug reports cluster around repeated symptoms".into());
        breakdown.push(ScoreFactor {
            key: "recurring_bugs".into(),
            label: "Recurring bug pattern".into(),
            impact: round1(recurring_bug_impact),
            detail: format!(
                "{} recurring bug clusters across {} issues; {}",
                issue_analysis.recurring_bug_clusters.len(),
                recurring_bug_issue_count,
                strongest
            ),
        });
    }

    let duplicate_impact = (duplicate_pressure * 10.0).min(14.0)
        + if issue_analysis.duplicate_candidates.len() >= 2 { 3.0 } else { 0.0 };
    if !issue_analysis.duplicate_candidates.is_empty() {
        let strongest = issue_analysis
            .duplicate_candidates
            .first()
            .map(|pair| format!("strongest pair looks {}% alike", (pair.similarity * 100.0).round() as i64))
            .unwrap_or_else(|| "title overlap suggests duplicate work".into());
        breakdown.push(ScoreFactor {
            key: "duplicates".into(),
            label: "Duplicate issue pressure".into(),
            impact: round1(duplicate_impact),
            detail: format!(
                "{} likely duplicate pairs; {}",
                issue_analysis.duplicate_candidates.len(),
                strongest
            ),
        });
    }

    let unlabeled_impact =
        ((unlabeled_ratio * 18.0) + (issue_analysis.unlabeled_issues.min(4) as f64 * 1.4)).min(12.0);
    if issue_analysis.unlabeled_issues > 0 {
        breakdown.push(ScoreFactor {
            key: "triage_gap".into(),
            label: "Triage gap".into(),
            impact: round1(unlabeled_impact),
            detail: format!(
                "{} sampled issues have no labels",
                issue_analysis.unlabeled_issues
            ),
        });
    }

    let marker_impact = (todo_count.min(20) as f64 * 0.45 + fixme_count.min(15) as f64 * 0.8).min(12.0);
    if todo_count > 0 || fixme_count > 0 {
        breakdown.push(ScoreFactor {
            key: "markers".into(),
            label: "Code markers".into(),
            impact: round1(marker_impact),
            detail: format!("{todo_count} TODO and {fixme_count} FIXME markers found"),
        });
    }

    let density_impact = ((issue_density - 10.0).max(0.0) * 0.35).min(10.0);
    if density_impact > 0.0 {
        breakdown.push(ScoreFactor {
            key: "issue_density".into(),
            label: "Open issue density".into(),
            impact: round1(density_impact),
            detail: format!(
                "{open_issues} open issues across {stars} stars ({:.1} per 100 stars)",
                issue_density
            ),
        });
    }

    breakdown.sort_by(|a, b| {
        b.impact
            .partial_cmp(&a.impact)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total = breakdown.iter().map(|factor| factor.impact).sum::<f64>().min(100.0);
    (round1(total), breakdown)
}

fn summary_from_signals(
    stars: u32,
    open_issues: u32,
    issue_analysis: &IssueAnalysis,
    todo_count: u32,
    fixme_count: u32,
) -> (String, Vec<String>) {
    let mut signals = Vec::new();

    if let Some(cluster) = issue_analysis.recurring_bug_clusters.first() {
        signals.push(format!(
            "Recurring bug pattern '{}' appears across {} sampled issues",
            cluster.label, cluster.issue_count
        ));
    }
    if issue_analysis.stale_bug_issues > 0 {
        signals.push(format!(
            "{} stale bug-like issues are still open",
            issue_analysis.stale_bug_issues
        ));
    }
    if issue_analysis.stale_issues > 0 {
        signals.push(format!(
            "{} of {} sampled issues look stale",
            issue_analysis.stale_issues, issue_analysis.sampled_issues
        ));
    }
    if issue_analysis.stale_high_comment_issues > 0 {
        signals.push(format!(
            "{} stale issues still have active comment history",
            issue_analysis.stale_high_comment_issues
        ));
    }
    if !issue_analysis.duplicate_candidates.is_empty() {
        signals.push(format!(
            "{} likely duplicate issue pairs were found",
            issue_analysis.duplicate_candidates.len()
        ));
    }
    if issue_analysis.unlabeled_issues >= 2 {
        signals.push(format!(
            "{} sampled issues are unlabeled, which points to triage drift",
            issue_analysis.unlabeled_issues
        ));
    }
    if todo_count > 0 || fixme_count > 0 {
        signals.push(format!(
            "{todo_count} TODO and {fixme_count} FIXME markers were found in code search"
        ));
    }

    let issue_density = (open_issues as f64 / stars.max(25) as f64) * 100.0;
    if issue_density >= 18.0 {
        signals.push(format!(
            "Open issue density is high for repo size ({:.1} per 100 stars)",
            issue_density
        ));
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
    let issue_analysis = issue_signals(&issues, params.stale_days);
    let todo_count = github::search_code_marker(client, &repo.full_name, "TODO").await;
    let fixme_count = github::search_code_marker(client, &repo.full_name, "FIXME").await;
    let (priority_score, score_breakdown) = priority_score(
        repo.stargazers_count,
        repo.open_issues_count,
        &issue_analysis,
        todo_count,
        fixme_count,
    );
    let (summary, signals) = summary_from_signals(
        repo.stargazers_count,
        repo.open_issues_count,
        &issue_analysis,
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
        sampled_issues: issue_analysis.sampled_issues,
        stale_issues: issue_analysis.stale_issues,
        unlabeled_issues: issue_analysis.unlabeled_issues,
        stale_bug_issues: issue_analysis.stale_bug_issues,
        stale_high_comment_issues: issue_analysis.stale_high_comment_issues,
        duplicate_candidates: issue_analysis.duplicate_candidates,
        recurring_bug_clusters: issue_analysis.recurring_bug_clusters,
        todo_count,
        fixme_count,
        priority_score,
        score_breakdown,
        summary,
        signals,
        issue_examples: issue_analysis.issue_examples,
    })
}

pub async fn scan(
    State(state): State<AppState>,
    Json(params): Json<ScanParams>,
) -> Result<Json<crate::models::ScanRecord>, (StatusCode, Json<serde_json::Value>)> {
    let params = clamp_params(params);
    let (allowlist, denylist, opt_out) = db::repo_list_sets().map_err(internal_error)?;
    if params.search_query.is_empty()
        && params.topics.is_empty()
        && params.languages.is_empty()
        && allowlist.is_empty()
    {
        return Err(bad_request(
            "Provide at least a search query, topic, or language, or configure an allowlist.",
        ));
    }

    let repos = github::discover_repositories(&state.http, &params, &allowlist, &denylist, &opt_out)
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

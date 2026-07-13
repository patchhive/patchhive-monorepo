use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use patchhive_github_data::{
    fetch_pull_files as fetch_shared_pull_files,
    fetch_pull_review_comments as fetch_shared_pull_review_comments,
    fetch_pull_reviews as fetch_shared_pull_reviews,
    search_closed_issues as search_shared_closed_issues,
    search_merged_pull_requests as search_shared_merged_pull_requests,
};
use reqwest::Client;

use crate::models::{
    GitHubIssue, GitHubPullFile, GitHubPullRequest, GitHubReview, GitHubReviewComment,
};

fn cutoff(since_days: u32) -> DateTime<Utc> {
    Utc::now() - Duration::days(since_days as i64)
}

fn is_recent(date: &str, since_days: u32) -> bool {
    DateTime::parse_from_rfc3339(date)
        .map(|value| value.with_timezone(&Utc) >= cutoff(since_days))
        .unwrap_or(false)
}

fn recent_merged_pulls(
    pulls: Vec<GitHubPullRequest>,
    limit: u32,
    since_days: u32,
) -> Vec<GitHubPullRequest> {
    pulls
        .into_iter()
        .filter(|pr| {
            pr.merged_at
                .as_deref()
                .is_some_and(|merged| is_recent(merged, since_days))
        })
        .take(limit as usize)
        .collect()
}

fn recent_closed_issues(issues: Vec<GitHubIssue>, limit: u32, since_days: u32) -> Vec<GitHubIssue> {
    issues
        .into_iter()
        .filter(|issue| issue.pull_request.is_none())
        .filter(|issue| {
            issue
                .closed_at
                .as_deref()
                .is_some_and(|closed_at| is_recent(closed_at, since_days))
        })
        .take(limit as usize)
        .collect()
}

pub async fn fetch_merged_pull_requests(
    client: &Client,
    repo: &str,
    limit: u32,
    since_days: u32,
) -> Result<Vec<GitHubPullRequest>> {
    let merged_since = cutoff(since_days).format("%Y-%m-%d").to_string();
    let pulls = search_shared_merged_pull_requests(client, repo, &merged_since, limit).await?;

    Ok(recent_merged_pulls(pulls, limit, since_days))
}

pub async fn fetch_pr_reviews(
    client: &Client,
    repo: &str,
    number: u32,
) -> Result<Vec<GitHubReview>> {
    fetch_shared_pull_reviews(client, repo, number).await
}

pub async fn fetch_pr_review_comments(
    client: &Client,
    repo: &str,
    number: u32,
) -> Result<Vec<GitHubReviewComment>> {
    fetch_shared_pull_review_comments(client, repo, number).await
}

pub async fn fetch_pr_files(
    client: &Client,
    repo: &str,
    number: u32,
) -> Result<Vec<GitHubPullFile>> {
    fetch_shared_pull_files(client, repo, number).await
}

pub async fn fetch_closed_issues(
    client: &Client,
    repo: &str,
    limit: u32,
    since_days: u32,
) -> Result<Vec<GitHubIssue>> {
    let closed_since = cutoff(since_days).format("%Y-%m-%d").to_string();
    let issues = search_shared_closed_issues(client, repo, &closed_since, limit).await?;

    Ok(recent_closed_issues(issues, limit, since_days))
}

#[cfg(test)]
mod tests {
    use chrono::SecondsFormat;

    use super::*;

    fn recent_timestamp() -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    #[test]
    fn merged_pull_limit_is_applied_after_filtering() {
        let closed_unmerged = GitHubPullRequest::default();
        let merged = GitHubPullRequest {
            number: 42,
            merged_at: Some(recent_timestamp()),
            ..GitHubPullRequest::default()
        };

        let filtered = recent_merged_pulls(vec![closed_unmerged, merged], 1, 180);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].number, 42);
    }

    #[test]
    fn issue_limit_is_applied_after_pull_requests_are_removed() {
        let pull_request = GitHubIssue {
            pull_request: Some(serde_json::json!({})),
            closed_at: Some(recent_timestamp()),
            ..GitHubIssue::default()
        };
        let issue = GitHubIssue {
            number: 24,
            closed_at: Some(recent_timestamp()),
            ..GitHubIssue::default()
        };

        let filtered = recent_closed_issues(vec![pull_request, issue], 1, 180);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].number, 24);
    }
}

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use patchhive_github_data::{
    fetch_issues, fetch_pull_files as fetch_shared_pull_files,
    fetch_pull_requests as fetch_shared_pull_requests,
    fetch_pull_review_comments as fetch_shared_pull_review_comments,
    fetch_pull_reviews as fetch_shared_pull_reviews,
    validate_token as validate_shared_token,
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

pub async fn validate_token(client: &Client) -> Result<()> {
    validate_shared_token(client).await
}

pub async fn fetch_merged_pull_requests(
    client: &Client,
    repo: &str,
    limit: u32,
    since_days: u32,
) -> Result<Vec<GitHubPullRequest>> {
    let pulls = fetch_shared_pull_requests(
        client,
        repo,
        "closed",
        "updated",
        "desc",
        limit.min(50),
    )
    .await?;

    Ok(pulls
        .into_iter()
        .filter(|pr| pr.merged_at.as_deref().is_some_and(|merged| is_recent(merged, since_days)))
        .collect())
}

pub async fn fetch_pr_reviews(client: &Client, repo: &str, number: u32) -> Result<Vec<GitHubReview>> {
    fetch_shared_pull_reviews(client, repo, number).await
}

pub async fn fetch_pr_review_comments(
    client: &Client,
    repo: &str,
    number: u32,
) -> Result<Vec<GitHubReviewComment>> {
    fetch_shared_pull_review_comments(client, repo, number).await
}

pub async fn fetch_pr_files(client: &Client, repo: &str, number: u32) -> Result<Vec<GitHubPullFile>> {
    fetch_shared_pull_files(client, repo, number).await
}

pub async fn fetch_closed_issues(
    client: &Client,
    repo: &str,
    limit: u32,
    since_days: u32,
) -> Result<Vec<GitHubIssue>> {
    let issues = fetch_issues(client, repo, "closed", "updated", "desc", limit.min(50)).await?;

    Ok(issues
        .into_iter()
        .filter(|issue| issue.pull_request.is_none())
        .filter(|issue| issue.closed_at.as_deref().is_some_and(|closed_at| is_recent(closed_at, since_days)))
        .collect())
}

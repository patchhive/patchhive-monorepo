use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::models::{
    GitHubIssue, GitHubPullFile, GitHubPullRequest, GitHubReview, GitHubReviewComment,
};

fn github_token() -> Result<String> {
    std::env::var("BOT_GITHUB_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .map_err(|_| anyhow!("BOT_GITHUB_TOKEN is not set"))
}

async fn github_get<T: DeserializeOwned>(
    client: &Client,
    path: &str,
    params: &[(&str, String)],
) -> Result<T> {
    let token = github_token()?;
    let url = format!("https://api.github.com{path}");
    let response = client
        .get(url)
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .query(params)
        .send()
        .await
        .with_context(|| format!("GitHub request failed for {path}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("GitHub request failed for {path}: {status} {body}"));
    }

    response
        .json::<T>()
        .await
        .with_context(|| format!("Failed to decode GitHub response for {path}"))
}

fn cutoff(since_days: u32) -> DateTime<Utc> {
    Utc::now() - Duration::days(since_days as i64)
}

fn is_recent(date: &str, since_days: u32) -> bool {
    DateTime::parse_from_rfc3339(date)
        .map(|value| value.with_timezone(&Utc) >= cutoff(since_days))
        .unwrap_or(false)
}

pub async fn validate_token(client: &Client) -> Result<()> {
    let _: serde_json::Value = github_get(client, "/rate_limit", &[]).await?;
    Ok(())
}

pub async fn fetch_merged_pull_requests(
    client: &Client,
    repo: &str,
    limit: u32,
    since_days: u32,
) -> Result<Vec<GitHubPullRequest>> {
    let path = format!("/repos/{repo}/pulls");
    let pulls: Vec<GitHubPullRequest> = github_get(
        client,
        &path,
        &[
            ("state", "closed".into()),
            ("sort", "updated".into()),
            ("direction", "desc".into()),
            ("per_page", limit.min(50).to_string()),
        ],
    )
    .await?;

    Ok(pulls
        .into_iter()
        .filter(|pr| pr.merged_at.as_deref().is_some_and(|merged| is_recent(merged, since_days)))
        .collect())
}

pub async fn fetch_pr_reviews(client: &Client, repo: &str, number: u32) -> Result<Vec<GitHubReview>> {
    let path = format!("/repos/{repo}/pulls/{number}/reviews");
    github_get(client, &path, &[("per_page", "100".into())]).await
}

pub async fn fetch_pr_review_comments(
    client: &Client,
    repo: &str,
    number: u32,
) -> Result<Vec<GitHubReviewComment>> {
    let path = format!("/repos/{repo}/pulls/{number}/comments");
    github_get(client, &path, &[("per_page", "100".into())]).await
}

pub async fn fetch_pr_files(client: &Client, repo: &str, number: u32) -> Result<Vec<GitHubPullFile>> {
    let path = format!("/repos/{repo}/pulls/{number}/files");
    github_get(client, &path, &[("per_page", "100".into())]).await
}

pub async fn fetch_closed_issues(
    client: &Client,
    repo: &str,
    limit: u32,
    since_days: u32,
) -> Result<Vec<GitHubIssue>> {
    let path = format!("/repos/{repo}/issues");
    let issues: Vec<GitHubIssue> = github_get(
        client,
        &path,
        &[
            ("state", "closed".into()),
            ("sort", "updated".into()),
            ("direction", "desc".into()),
            ("per_page", limit.min(50).to_string()),
        ],
    )
    .await?;

    Ok(issues
        .into_iter()
        .filter(|issue| issue.pull_request.is_none())
        .filter(|issue| issue.closed_at.as_deref().is_some_and(|closed_at| is_recent(closed_at, since_days)))
        .collect())
}

use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
    Client,
};
use serde::de::DeserializeOwned;

use crate::models::{
    GitHubActionsWorkflowJob, GitHubActionsWorkflowJobsResponse, GitHubActionsWorkflowRun,
    GitHubActionsWorkflowRunsResponse, GitHubCodeSearchResponse, GitHubIssue, GitHubPullFile,
    GitHubPullRequest, GitHubRepository, GitHubReview, GitHubReviewComment,
    GitHubSearchRepositoriesResponse,
};

const GH_API: &str = "https://api.github.com";

pub fn github_token() -> Option<String> {
    std::env::var("BOT_GITHUB_TOKEN")
        .ok()
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn github_token_required() -> Result<String> {
    github_token().ok_or_else(|| anyhow!("BOT_GITHUB_TOKEN is not set"))
}

pub fn github_token_configured() -> bool {
    github_token().is_some()
}

fn request_headers(token: Option<&str>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("patchhive-github-data/0.1"),
    );
    headers.insert("X-GitHub-Api-Version", HeaderValue::from_static("2022-11-28"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
    if let Some(token) = token {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))?,
        );
    }
    Ok(headers)
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

async fn get_json<T: DeserializeOwned>(
    client: &Client,
    path: &str,
    query: &[(&str, String)],
    token: Option<&str>,
) -> Result<T> {
    let response = client
        .get(format!("{GH_API}{path}"))
        .headers(request_headers(token)?)
        .query(query)
        .send()
        .await
        .with_context(|| format!("GitHub request failed for {path}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("GitHub GET {path} -> {status}: {body}"));
    }

    response
        .json::<T>()
        .await
        .with_context(|| format!("Could not decode GitHub JSON for {path}"))
}

async fn get_public_json<T: DeserializeOwned>(
    client: &Client,
    path: &str,
    query: &[(&str, String)],
) -> Result<T> {
    let token = github_token();
    get_json(client, path, query, token.as_deref()).await
}

async fn get_authenticated_json<T: DeserializeOwned>(
    client: &Client,
    path: &str,
    query: &[(&str, String)],
) -> Result<T> {
    let token = github_token_required()?;
    get_json(client, path, query, Some(token.as_str())).await
}

pub async fn validate_token(client: &Client) -> Result<()> {
    let _: serde_json::Value = get_authenticated_json(client, "/rate_limit", &[]).await?;
    Ok(())
}

pub async fn fetch_repository(client: &Client, full_name: &str) -> Result<GitHubRepository> {
    if !valid_repo(full_name) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    get_public_json(client, &format!("/repos/{full_name}"), &[]).await
}

pub async fn search_repositories(
    client: &Client,
    query: &str,
    per_page: u32,
    sort: &str,
    order: &str,
) -> Result<GitHubSearchRepositoriesResponse> {
    get_public_json(
        client,
        "/search/repositories",
        &[
            ("q", query.trim().to_string()),
            ("sort", sort.trim().to_string()),
            ("order", order.trim().to_string()),
            ("per_page", per_page.min(100).max(1).to_string()),
        ],
    )
    .await
}

pub async fn fetch_issues(
    client: &Client,
    repo: &str,
    state: &str,
    sort: &str,
    direction: &str,
    per_page: u32,
) -> Result<Vec<GitHubIssue>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    get_public_json(
        client,
        &format!("/repos/{repo}/issues"),
        &[
            ("state", state.trim().to_string()),
            ("sort", sort.trim().to_string()),
            ("direction", direction.trim().to_string()),
            ("per_page", per_page.min(100).max(1).to_string()),
        ],
    )
    .await
}

pub async fn fetch_pull_requests(
    client: &Client,
    repo: &str,
    state: &str,
    sort: &str,
    direction: &str,
    per_page: u32,
) -> Result<Vec<GitHubPullRequest>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    get_public_json(
        client,
        &format!("/repos/{repo}/pulls"),
        &[
            ("state", state.trim().to_string()),
            ("sort", sort.trim().to_string()),
            ("direction", direction.trim().to_string()),
            ("per_page", per_page.min(100).max(1).to_string()),
        ],
    )
    .await
}

pub async fn fetch_pull_reviews(
    client: &Client,
    repo: &str,
    number: u32,
) -> Result<Vec<GitHubReview>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    get_public_json(
        client,
        &format!("/repos/{repo}/pulls/{number}/reviews"),
        &[("per_page", "100".into())],
    )
    .await
}

pub async fn fetch_pull_review_comments(
    client: &Client,
    repo: &str,
    number: u32,
) -> Result<Vec<GitHubReviewComment>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    get_public_json(
        client,
        &format!("/repos/{repo}/pulls/{number}/comments"),
        &[("per_page", "100".into())],
    )
    .await
}

pub async fn fetch_pull_files(
    client: &Client,
    repo: &str,
    number: u32,
) -> Result<Vec<GitHubPullFile>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    get_public_json(
        client,
        &format!("/repos/{repo}/pulls/{number}/files"),
        &[("per_page", "100".into())],
    )
    .await
}

pub async fn code_search_count(client: &Client, query: &str) -> Result<u32> {
    let response: GitHubCodeSearchResponse = get_public_json(
        client,
        "/search/code",
        &[
            ("q", query.trim().to_string()),
            ("per_page", "1".into()),
        ],
    )
    .await?;
    Ok(response.total_count)
}

pub async fn fetch_workflow_runs(
    client: &Client,
    repo: &str,
    branch: Option<&str>,
    limit: u32,
) -> Result<Vec<GitHubActionsWorkflowRun>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    let mut query = vec![
        ("per_page", limit.min(100).max(1).to_string()),
        ("exclude_pull_requests", "false".into()),
    ];

    if let Some(branch) = branch.filter(|value| !value.trim().is_empty()) {
        query.push(("branch", branch.trim().to_string()));
    }

    let response: GitHubActionsWorkflowRunsResponse =
        get_public_json(client, &format!("/repos/{repo}/actions/runs"), &query).await?;
    Ok(response.workflow_runs)
}

pub async fn fetch_workflow_jobs(
    client: &Client,
    repo: &str,
    run_id: i64,
) -> Result<Vec<GitHubActionsWorkflowJob>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    let response: GitHubActionsWorkflowJobsResponse = get_public_json(
        client,
        &format!("/repos/{repo}/actions/runs/{run_id}/jobs"),
        &[("per_page", "100".into())],
    )
    .await?;
    Ok(response.jobs)
}

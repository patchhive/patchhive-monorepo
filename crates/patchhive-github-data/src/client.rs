use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
    Client,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::models::{
    GitHubActionsWorkflowJob, GitHubActionsWorkflowRun, GitHubCodeSearchResponse, GitHubIssue,
    GitHubPullFile, GitHubPullRequest, GitHubRepository, GitHubReview, GitHubReviewComment,
    GitHubSearchRepositoriesResponse,
};

pub const GH_API: &str = "https://api.github.com";

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

pub fn request_headers(user_agent: &str, token: Option<&str>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_str(user_agent)?);
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

pub fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

pub async fn get_json<T: DeserializeOwned>(
    client: &Client,
    user_agent: &str,
    path: &str,
    query: &[(&str, String)],
    token: Option<&str>,
) -> Result<T> {
    let response = client
        .get(format!("{GH_API}{path}"))
        .headers(request_headers(user_agent, token)?)
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

pub async fn get_paginated_json<T: DeserializeOwned>(
    client: &Client,
    user_agent: &str,
    path: &str,
    query: &[(&str, String)],
    token: Option<&str>,
    max_items: usize,
) -> Result<Vec<T>> {
    if max_items == 0 {
        return Ok(Vec::new());
    }

    let mut page = 1usize;
    let mut items = Vec::new();

    loop {
        let remaining = max_items.saturating_sub(items.len());
        if remaining == 0 {
            break;
        }

        let mut page_query = query.to_vec();
        page_query.push(("per_page", remaining.min(100).to_string()));
        page_query.push(("page", page.to_string()));

        let mut page_items: Vec<T> = get_json(client, user_agent, path, &page_query, token).await?;
        let page_len = page_items.len();
        items.append(&mut page_items);

        if page_len < remaining.min(100) {
            break;
        }
        page += 1;
    }

    Ok(items)
}

pub async fn get_paginated_field_json<T: DeserializeOwned>(
    client: &Client,
    user_agent: &str,
    path: &str,
    query: &[(&str, String)],
    token: Option<&str>,
    array_key: &str,
    max_items: usize,
) -> Result<Vec<T>> {
    if max_items == 0 {
        return Ok(Vec::new());
    }

    let mut page = 1usize;
    let mut items = Vec::new();

    loop {
        let remaining = max_items.saturating_sub(items.len());
        if remaining == 0 {
            break;
        }

        let mut page_query = query.to_vec();
        let page_size = remaining.min(100);
        page_query.push(("per_page", page_size.to_string()));
        page_query.push(("page", page.to_string()));

        let value: Value = get_json(client, user_agent, path, &page_query, token).await?;
        let page_items = value[array_key]
            .as_array()
            .ok_or_else(|| anyhow!("GitHub response field `{array_key}` was not an array"))?;

        let page_len = page_items.len();
        for item in page_items {
            items.push(
                serde_json::from_value::<T>(item.clone())
                    .with_context(|| format!("Could not decode GitHub JSON field `{array_key}` for {path}"))?,
            );
        }

        if page_len < page_size {
            break;
        }
        page += 1;
    }

    Ok(items)
}

async fn get_public_json<T: DeserializeOwned>(
    client: &Client,
    path: &str,
    query: &[(&str, String)],
) -> Result<T> {
    let token = github_token();
    get_json(client, "patchhive-github-data/0.1", path, query, token.as_deref()).await
}

async fn get_authenticated_json<T: DeserializeOwned>(
    client: &Client,
    path: &str,
    query: &[(&str, String)],
) -> Result<T> {
    let token = github_token_required()?;
    get_json(client, "patchhive-github-data/0.1", path, query, Some(token.as_str())).await
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
    let limit = per_page.clamp(1, 1000) as usize;
    let token = github_token();
    let items = get_paginated_field_json(
        client,
        "patchhive-github-data/0.1",
        "/search/repositories",
        &[
            ("q", query.trim().to_string()),
            ("sort", sort.trim().to_string()),
            ("order", order.trim().to_string()),
        ],
        token.as_deref(),
        "items",
        limit,
    )
    .await?;
    Ok(GitHubSearchRepositoriesResponse { items })
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

    get_paginated_json(
        client,
        "patchhive-github-data/0.1",
        &format!("/repos/{repo}/issues"),
        &[
            ("state", state.trim().to_string()),
            ("sort", sort.trim().to_string()),
            ("direction", direction.trim().to_string()),
        ],
        github_token().as_deref(),
        per_page.clamp(1, 1000) as usize,
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

    get_paginated_json(
        client,
        "patchhive-github-data/0.1",
        &format!("/repos/{repo}/pulls"),
        &[
            ("state", state.trim().to_string()),
            ("sort", sort.trim().to_string()),
            ("direction", direction.trim().to_string()),
        ],
        github_token().as_deref(),
        per_page.clamp(1, 1000) as usize,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_repo_accepts_owner_name_format() {
        assert!(valid_repo("patchhive/repo.reaper_1"));
    }

    #[test]
    fn valid_repo_rejects_extra_or_missing_segments() {
        assert!(!valid_repo("patchhive/repo/extra"));
        assert!(!valid_repo("patchhive/"));
    }

    #[test]
    fn request_headers_adds_bearer_token_when_present() {
        let headers = request_headers("patchhive-test", Some("secret-token"))
            .expect("headers should build");

        assert_eq!(
            headers
                .get(AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer secret-token")
        );
        assert_eq!(
            headers
                .get(USER_AGENT)
                .and_then(|value| value.to_str().ok()),
            Some("patchhive-test")
        );
    }
}

pub async fn fetch_pull_reviews(
    client: &Client,
    repo: &str,
    number: u32,
) -> Result<Vec<GitHubReview>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    get_paginated_json(
        client,
        "patchhive-github-data/0.1",
        &format!("/repos/{repo}/pulls/{number}/reviews"),
        &[],
        github_token().as_deref(),
        1000,
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

    get_paginated_json(
        client,
        "patchhive-github-data/0.1",
        &format!("/repos/{repo}/pulls/{number}/comments"),
        &[],
        github_token().as_deref(),
        1000,
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

    get_paginated_json(
        client,
        "patchhive-github-data/0.1",
        &format!("/repos/{repo}/pulls/{number}/files"),
        &[],
        github_token().as_deref(),
        1000,
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

    let mut query = vec![("exclude_pull_requests", "false".into())];

    if let Some(branch) = branch.filter(|value| !value.trim().is_empty()) {
        query.push(("branch", branch.trim().to_string()));
    }

    get_paginated_field_json(
        client,
        "patchhive-github-data/0.1",
        &format!("/repos/{repo}/actions/runs"),
        &query,
        github_token().as_deref(),
        "workflow_runs",
        limit.clamp(1, 1000) as usize,
    )
    .await
}

pub async fn fetch_workflow_jobs(
    client: &Client,
    repo: &str,
    run_id: i64,
) -> Result<Vec<GitHubActionsWorkflowJob>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    get_paginated_field_json(
        client,
        "patchhive-github-data/0.1",
        &format!("/repos/{repo}/actions/runs/{run_id}/jobs"),
        &[],
        github_token().as_deref(),
        "jobs",
        1000,
    )
    .await
}

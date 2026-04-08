use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use tracing::warn;

use crate::models::{CodeSearchResponse, GitHubIssue, ScanParams, SearchRepo, SearchRepositoriesResponse};

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

pub async fn validate_token(client: &Client) -> Result<()> {
    let _: serde_json::Value = github_get(client, "/rate_limit", &[]).await?;
    Ok(())
}

pub async fn discover_repositories(client: &Client, params: &ScanParams) -> Result<Vec<SearchRepo>> {
    let languages = if params.languages.is_empty() {
        vec![String::new()]
    } else {
        params.languages.clone()
    };

    let mut seen = std::collections::HashSet::new();
    let mut repos = Vec::new();

    for language in languages {
        if repos.len() >= params.max_repos as usize {
            break;
        }

        let mut query_parts = vec![
            "archived:false".to_string(),
            "is:public".to_string(),
            format!("stars:>={}", params.min_stars.max(1)),
        ];

        if !params.search_query.trim().is_empty() {
            query_parts.push(params.search_query.trim().to_string());
        }

        for topic in &params.topics {
            let topic = topic.trim();
            if !topic.is_empty() {
                query_parts.push(topic.to_string());
            }
        }

        if !language.trim().is_empty() {
            query_parts.push(format!("language:{language}"));
        }

        let response: SearchRepositoriesResponse = github_get(
            client,
            "/search/repositories",
            &[
                ("q", query_parts.join(" ")),
                ("sort", "updated".to_string()),
                ("order", "desc".to_string()),
                ("per_page", params.max_repos.min(25).to_string()),
            ],
        )
        .await?;

        for repo in response.items {
            if seen.insert(repo.full_name.clone()) {
                repos.push(repo);
            }
            if repos.len() >= params.max_repos as usize {
                break;
            }
        }
    }

    Ok(repos)
}

pub async fn fetch_open_issues(
    client: &Client,
    owner: &str,
    repo: &str,
    per_page: u32,
) -> Result<Vec<GitHubIssue>> {
    let path = format!("/repos/{owner}/{repo}/issues");
    let mut issues: Vec<GitHubIssue> = github_get(
        client,
        &path,
        &[
            ("state", "open".to_string()),
            ("sort", "updated".to_string()),
            ("direction", "desc".to_string()),
            ("per_page", per_page.min(100).to_string()),
        ],
    )
    .await?;

    issues.retain(|issue| issue.pull_request.is_none());
    Ok(issues)
}

pub async fn search_code_marker(client: &Client, full_name: &str, marker: &str) -> u32 {
    let result: Result<CodeSearchResponse> = github_get(
        client,
        "/search/code",
        &[
            ("q", format!("{marker} repo:{full_name}")),
            ("per_page", "1".to_string()),
        ],
    )
    .await;

    match result {
        Ok(response) => response.total_count,
        Err(err) => {
            warn!("code search failed for {full_name} marker {marker}: {err}");
            0
        }
    }
}

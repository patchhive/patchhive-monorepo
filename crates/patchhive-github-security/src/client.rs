use anyhow::{anyhow, Context, Result};
pub use patchhive_github_data::{
    github_token, github_token_configured, github_token_required, validate_token,
};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
    Client,
};
use serde::de::DeserializeOwned;

use crate::models::{GitHubCodeScanningAlert, GitHubDependabotAlert};

const GH_API: &str = "https://api.github.com";

fn request_headers(token: Option<&str>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("patchhive-github-security/0.1"),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );
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

pub async fn fetch_code_scanning_alerts(
    client: &Client,
    repo: &str,
    limit: u32,
) -> Result<Vec<GitHubCodeScanningAlert>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    let token = github_token();
    get_json(
        client,
        &format!("/repos/{repo}/code-scanning/alerts"),
        &[
            ("state", "open".into()),
            ("sort", "created".into()),
            ("direction", "desc".into()),
            ("per_page", limit.clamp(1, 100).to_string()),
        ],
        token.as_deref(),
    )
    .await
}

pub async fn fetch_dependabot_alerts(
    client: &Client,
    repo: &str,
    limit: u32,
) -> Result<Vec<GitHubDependabotAlert>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    let token = github_token_required()?;
    get_json(
        client,
        &format!("/repos/{repo}/dependabot/alerts"),
        &[
            ("state", "open".into()),
            ("per_page", limit.clamp(1, 100).to_string()),
        ],
        Some(token.as_str()),
    )
    .await
}

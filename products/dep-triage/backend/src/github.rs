use anyhow::{anyhow, Context, Result};
use patchhive_github_data::{
    fetch_pull_files as fetch_shared_pull_files, fetch_pull_requests as fetch_shared_pull_requests,
    github_token,
};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
    Client,
};
use serde::Deserialize;

pub use patchhive_github_data::models::{GitHubPullFile, GitHubPullRequest};

const GH_API: &str = "https://api.github.com";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubDependabotAlert {
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub security_advisory: GitHubSecurityAdvisory,
    #[serde(default)]
    pub security_vulnerability: GitHubSecurityVulnerability,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubSecurityAdvisory {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub severity: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubSecurityVulnerability {
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub vulnerable_version_range: String,
    #[serde(default)]
    pub package: GitHubPackageRef,
    pub first_patched_version: Option<GitHubPatchedVersion>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubPackageRef {
    #[serde(default)]
    pub ecosystem: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitHubPatchedVersion {
    #[serde(default)]
    pub identifier: String,
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

fn request_headers() -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("deptriage-patchhive/0.1"),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));

    let token = github_token().ok_or_else(|| {
        anyhow!("BOT_GITHUB_TOKEN or GITHUB_TOKEN is required for Dependabot alert reads")
    })?;
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}"))?,
    );
    Ok(headers)
}

pub async fn fetch_pull_requests(
    client: &Client,
    repo: &str,
    limit: u32,
) -> Result<Vec<GitHubPullRequest>> {
    fetch_shared_pull_requests(client, repo, "open", "updated", "desc", limit).await
}

pub async fn fetch_pull_files(
    client: &Client,
    repo: &str,
    number: u32,
) -> Result<Vec<GitHubPullFile>> {
    fetch_shared_pull_files(client, repo, number).await
}

pub async fn fetch_dependabot_alerts(
    client: &Client,
    repo: &str,
    limit: u32,
) -> Result<Vec<GitHubDependabotAlert>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    let response = client
        .get(format!("{GH_API}/repos/{repo}/dependabot/alerts"))
        .headers(request_headers()?)
        .query(&[
            ("state", "open".to_string()),
            ("per_page", limit.clamp(1, 100).to_string()),
        ])
        .send()
        .await
        .with_context(|| format!("GitHub request failed for dependabot alerts in {repo}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "GitHub GET /repos/{repo}/dependabot/alerts -> {status}: {body}"
        ));
    }

    response
        .json::<Vec<GitHubDependabotAlert>>()
        .await
        .context("Could not decode Dependabot alerts")
}

pub use patchhive_github_data::{github_token_configured, validate_token};

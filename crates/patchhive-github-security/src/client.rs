use anyhow::{anyhow, Result};
pub use patchhive_github_data::{
    get_paginated_json, github_token, github_token_configured, github_token_required, valid_repo,
    validate_token,
};
use reqwest::Client;

use crate::models::{GitHubCodeScanningAlert, GitHubDependabotAlert};

pub async fn fetch_code_scanning_alerts(
    client: &Client,
    repo: &str,
    limit: u32,
) -> Result<Vec<GitHubCodeScanningAlert>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    let token = github_token();
    get_paginated_json(
        client,
        "patchhive-github-security/0.1",
        &format!("/repos/{repo}/code-scanning/alerts"),
        &[
            ("state", "open".into()),
            ("sort", "created".into()),
            ("direction", "desc".into()),
        ],
        token.as_deref(),
        limit.clamp(1, 1000) as usize,
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
    get_paginated_json(
        client,
        "patchhive-github-security/0.1",
        &format!("/repos/{repo}/dependabot/alerts"),
        &[("state", "open".into())],
        Some(token.as_str()),
        limit.clamp(1, 1000) as usize,
    )
    .await
}

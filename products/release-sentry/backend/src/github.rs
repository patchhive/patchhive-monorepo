use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use patchhive_github_data::{
    fetch_issues as fetch_shared_issues, fetch_workflow_runs as fetch_shared_workflow_runs,
    get_json,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub use patchhive_github_data::models::{GitHubActionsWorkflowRun, GitHubIssue};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubRepositoryDetail {
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub default_branch: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub open_issues_count: u32,
    pub pushed_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubRelease {
    #[serde(default)]
    pub tag_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    pub published_at: Option<String>,
    #[serde(default)]
    pub target_commitish: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubTagCommit {
    #[serde(default)]
    pub sha: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubTag {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub zipball_url: String,
    #[serde(default)]
    pub tarball_url: String,
    #[serde(default)]
    pub commit: GitHubTagCommit,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubContentFile {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub encoding: String,
    #[serde(default)]
    pub content: String,
}

pub async fn fetch_repository(client: &Client, repo: &str) -> Result<GitHubRepositoryDetail> {
    get_json(
        client,
        "release-sentry/0.1",
        &format!("/repos/{repo}"),
        &[],
        patchhive_github_data::github_token().as_deref(),
    )
    .await
}

pub async fn fetch_releases(client: &Client, repo: &str, limit: u32) -> Result<Vec<GitHubRelease>> {
    get_json(
        client,
        "release-sentry/0.1",
        &format!("/repos/{repo}/releases"),
        &[("per_page", limit.clamp(1, 100).to_string())],
        patchhive_github_data::github_token().as_deref(),
    )
    .await
}

pub async fn fetch_tags(client: &Client, repo: &str, limit: u32) -> Result<Vec<GitHubTag>> {
    get_json(
        client,
        "release-sentry/0.1",
        &format!("/repos/{repo}/tags"),
        &[("per_page", limit.clamp(1, 100).to_string())],
        patchhive_github_data::github_token().as_deref(),
    )
    .await
}

pub async fn fetch_workflow_runs(
    client: &Client,
    repo: &str,
    branch: Option<&str>,
    limit: u32,
) -> Result<Vec<GitHubActionsWorkflowRun>> {
    fetch_shared_workflow_runs(client, repo, branch, limit).await
}

pub async fn fetch_open_issues(
    client: &Client,
    repo: &str,
    limit: u32,
) -> Result<Vec<GitHubIssue>> {
    fetch_shared_issues(client, repo, "open", "updated", "desc", limit).await
}

pub async fn fetch_content_text(
    client: &Client,
    repo: &str,
    path: &str,
    branch: &str,
) -> Result<GitHubContentFile> {
    get_json(
        client,
        "release-sentry/0.1",
        &format!("/repos/{repo}/contents/{}", path.trim_start_matches('/')),
        &[("ref", branch.to_string())],
        patchhive_github_data::github_token().as_deref(),
    )
    .await
}

pub fn decode_content(file: &GitHubContentFile) -> Option<String> {
    if file.encoding != "base64" {
        return None;
    }
    let encoded = file.content.replace(['\n', '\r'], "");
    let bytes = general_purpose::STANDARD.decode(encoded).ok()?;
    String::from_utf8(bytes).ok()
}

pub use patchhive_github_data::{github_token_configured, validate_token};

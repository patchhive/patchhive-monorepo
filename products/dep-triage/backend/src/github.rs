use anyhow::Result;
use patchhive_github_data::{
    fetch_pull_files as fetch_shared_pull_files, fetch_pull_requests as fetch_shared_pull_requests,
};
use patchhive_github_security::fetch_dependabot_alerts as fetch_shared_dependabot_alerts;
use reqwest::Client;

pub use patchhive_github_data::models::{GitHubPullFile, GitHubPullRequest};
pub use patchhive_github_security::models::GitHubDependabotAlert;

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
    fetch_shared_dependabot_alerts(client, repo, limit).await
}

pub use patchhive_github_data::github_token_configured;
pub use patchhive_github_security::validate_token;

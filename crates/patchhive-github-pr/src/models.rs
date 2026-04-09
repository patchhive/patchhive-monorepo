use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    #[serde(default)]
    pub repo: String,
    pub number: i64,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub merged: bool,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub head_repo: String,
    #[serde(default)]
    pub head_sha: String,
    #[serde(default)]
    pub head_ref: String,
    #[serde(default)]
    pub base_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCheckRunRequest {
    pub name: String,
    pub head_sha: String,
    pub conclusion: String,
    #[serde(default)]
    pub external_id: String,
    #[serde(default)]
    pub details_url: Option<String>,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubCheckRunResult {
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub api_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCommitStatusRequest {
    pub sha: String,
    pub state: String,
    pub context: String,
    pub description: String,
    #[serde(default)]
    pub target_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubCommitStatusResult {
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubManagedCommentResult {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub html_url: String,
}

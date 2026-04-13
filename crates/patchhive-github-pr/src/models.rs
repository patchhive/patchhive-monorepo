use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubPullRequestDetail {
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
    pub mergeable: Option<bool>,
    #[serde(default)]
    pub mergeable_state: String,
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
    #[serde(default)]
    pub additions: u32,
    #[serde(default)]
    pub deletions: u32,
    #[serde(default)]
    pub changed_files: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubPullReview {
    pub id: i64,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub submitted_at: String,
    #[serde(default)]
    pub author_login: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubPullReviewThreadComment {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub author_login: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubPullReviewThread {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub is_resolved: bool,
    #[serde(default)]
    pub is_outdated: bool,
    #[serde(default)]
    pub comments: Vec<GitHubPullReviewThreadComment>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubStatusContext {
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub target_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubCheckRunSummary {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: String,
    #[serde(default)]
    pub html_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubCommitHealth {
    #[serde(default)]
    pub combined_state: String,
    #[serde(default)]
    pub successful_contexts: u32,
    #[serde(default)]
    pub pending_contexts: u32,
    #[serde(default)]
    pub failing_contexts: u32,
    #[serde(default)]
    pub neutral_contexts: u32,
    #[serde(default)]
    pub successful_checks: u32,
    #[serde(default)]
    pub pending_checks: u32,
    #[serde(default)]
    pub failing_checks: u32,
    #[serde(default)]
    pub neutral_checks: u32,
    #[serde(default)]
    pub statuses: Vec<GitHubStatusContext>,
    #[serde(default)]
    pub check_runs: Vec<GitHubCheckRunSummary>,
}

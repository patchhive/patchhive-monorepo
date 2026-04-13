use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubUser {
    #[serde(default)]
    pub login: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubLabel {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubRepositoryOwner {
    #[serde(default)]
    pub login: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubRepository {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub html_url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    #[serde(default)]
    pub stargazers_count: u32,
    #[serde(default)]
    pub open_issues_count: u32,
    #[serde(default)]
    pub owner: GitHubRepositoryOwner,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubSearchRepositoriesResponse {
    #[serde(default)]
    pub items: Vec<GitHubRepository>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubIssue {
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub html_url: String,
    pub body: Option<String>,
    pub closed_at: Option<String>,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub comments: u32,
    #[serde(default)]
    pub labels: Vec<GitHubLabel>,
    pub pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub html_url: String,
    pub body: Option<String>,
    pub merged_at: Option<String>,
    #[serde(default)]
    pub updated_at: String,
    pub additions: Option<u32>,
    pub deletions: Option<u32>,
    pub changed_files: Option<u32>,
    pub user: Option<GitHubUser>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubReview {
    pub body: Option<String>,
    pub html_url: Option<String>,
    pub submitted_at: Option<String>,
    #[serde(default)]
    pub state: String,
    pub user: Option<GitHubUser>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubReviewComment {
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub html_url: String,
    pub path: Option<String>,
    #[serde(default)]
    pub created_at: String,
    pub user: Option<GitHubUser>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubPullFile {
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub additions: u32,
    #[serde(default)]
    pub deletions: u32,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubCodeSearchResponse {
    #[serde(default)]
    pub total_count: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubActionsWorkflowRun {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub head_branch: String,
    #[serde(default)]
    pub conclusion: String,
    #[serde(default = "default_run_attempt")]
    pub run_attempt: u32,
    #[serde(default)]
    pub run_number: u32,
}

fn default_run_attempt() -> u32 {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubActionsWorkflowStep {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub conclusion: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubActionsWorkflowJob {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub conclusion: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub runner_name: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub steps: Vec<GitHubActionsWorkflowStep>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubActionsWorkflowRunsResponse {
    #[serde(default)]
    pub workflow_runs: Vec<GitHubActionsWorkflowRun>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubActionsWorkflowJobsResponse {
    #[serde(default)]
    pub jobs: Vec<GitHubActionsWorkflowJob>,
}

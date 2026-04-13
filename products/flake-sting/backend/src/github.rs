use anyhow::Result;
use patchhive_github_data::{
    fetch_workflow_jobs as fetch_shared_workflow_jobs,
    fetch_workflow_runs as fetch_shared_workflow_runs,
};
use reqwest::Client;

pub use patchhive_github_data::github_token_configured;
pub use patchhive_github_data::models::{
    GitHubActionsWorkflowJob as GitHubWorkflowJob,
    GitHubActionsWorkflowRun as GitHubWorkflowRun,
};

pub async fn fetch_workflow_runs(
    client: &Client,
    repo: &str,
    branch: Option<&str>,
    limit: u32,
) -> Result<Vec<GitHubWorkflowRun>> {
    fetch_shared_workflow_runs(client, repo, branch, limit).await
}

pub async fn fetch_workflow_jobs(
    client: &Client,
    repo: &str,
    run_id: i64,
) -> Result<Vec<GitHubWorkflowJob>> {
    fetch_shared_workflow_jobs(client, repo, run_id).await
}

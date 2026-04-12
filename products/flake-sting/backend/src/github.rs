use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
    Client,
};
use serde_json::Value;

const GH_API: &str = "https://api.github.com";

#[derive(Debug, Clone, Default)]
pub struct GitHubWorkflowRun {
    pub id: i64,
    pub name: String,
    pub html_url: String,
    pub head_branch: String,
    pub conclusion: String,
    pub run_attempt: u32,
    pub run_number: u32,
}

#[derive(Debug, Clone, Default)]
pub struct GitHubWorkflowStep {
    pub name: String,
    pub conclusion: String,
}

#[derive(Debug, Clone, Default)]
pub struct GitHubWorkflowJob {
    pub name: String,
    pub conclusion: String,
    pub html_url: String,
    pub runner_name: String,
    pub labels: Vec<String>,
    pub steps: Vec<GitHubWorkflowStep>,
}

fn github_token() -> Option<String> {
    std::env::var("BOT_GITHUB_TOKEN")
        .ok()
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn github_token_configured() -> bool {
    github_token().is_some()
}

fn headers() -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("flake-sting/0.1"));
    headers.insert("X-GitHub-Api-Version", HeaderValue::from_static("2022-11-28"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
    if let Some(token) = github_token() {
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

async fn get_json(
    client: &Client,
    path: &str,
    query: &[(&str, String)],
) -> Result<Value> {
    let response = client
        .get(format!("{GH_API}{path}"))
        .headers(headers()?)
        .query(query)
        .send()
        .await
        .with_context(|| format!("GitHub request failed for {path}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("GitHub GET {path} -> {status}: {body}"));
    }

    response
        .json::<Value>()
        .await
        .with_context(|| format!("Could not decode GitHub JSON for {path}"))
}

pub async fn fetch_workflow_runs(
    client: &Client,
    repo: &str,
    branch: Option<&str>,
    limit: u32,
) -> Result<Vec<GitHubWorkflowRun>> {
    if !valid_repo(repo) {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    let mut query = vec![
        ("per_page", limit.min(100).max(1).to_string()),
        ("exclude_pull_requests", "false".into()),
    ];
    if let Some(branch) = branch.filter(|value| !value.trim().is_empty()) {
        query.push(("branch", branch.trim().to_string()));
    }

    let value = get_json(client, &format!("/repos/{repo}/actions/runs"), &query).await?;
    let items = value["workflow_runs"]
        .as_array()
        .ok_or_else(|| anyhow!("GitHub workflow runs response was not an array"))?;

    Ok(items
        .iter()
        .map(|item| GitHubWorkflowRun {
            id: item["id"].as_i64().unwrap_or(0),
            name: item["name"].as_str().unwrap_or("").to_string(),
            html_url: item["html_url"].as_str().unwrap_or("").to_string(),
            head_branch: item["head_branch"].as_str().unwrap_or("").to_string(),
            conclusion: item["conclusion"].as_str().unwrap_or("").to_string(),
            run_attempt: item["run_attempt"].as_u64().unwrap_or(1) as u32,
            run_number: item["run_number"].as_u64().unwrap_or(0) as u32,
        })
        .collect())
}

pub async fn fetch_workflow_jobs(
    client: &Client,
    repo: &str,
    run_id: i64,
) -> Result<Vec<GitHubWorkflowJob>> {
    let value = get_json(
        client,
        &format!("/repos/{repo}/actions/runs/{run_id}/jobs"),
        &[("per_page", "100".into())],
    )
    .await?;
    let items = value["jobs"]
        .as_array()
        .ok_or_else(|| anyhow!("GitHub workflow jobs response was not an array"))?;

    Ok(items
        .iter()
        .map(|item| GitHubWorkflowJob {
            name: item["name"].as_str().unwrap_or("").to_string(),
            conclusion: item["conclusion"].as_str().unwrap_or("").to_string(),
            html_url: item["html_url"].as_str().unwrap_or("").to_string(),
            runner_name: item["runner_name"].as_str().unwrap_or("").to_string(),
            labels: item["labels"]
                .as_array()
                .map(|labels| {
                    labels
                        .iter()
                        .filter_map(|label| label.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            steps: item["steps"]
                .as_array()
                .map(|steps| {
                    steps
                        .iter()
                        .map(|step| GitHubWorkflowStep {
                            name: step["name"].as_str().unwrap_or("").to_string(),
                            conclusion: step["conclusion"].as_str().unwrap_or("").to_string(),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        })
        .collect())
}

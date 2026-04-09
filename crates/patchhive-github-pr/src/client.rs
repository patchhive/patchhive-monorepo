use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
    Client,
};
use serde_json::{json, Value};

use crate::{
    models::{
        GitHubCheckRunRequest, GitHubCheckRunResult, GitHubCommitStatusRequest,
        GitHubCommitStatusResult, GitHubManagedCommentResult, GitHubPullRequest,
    },
    webhook::github_token_from_env,
};

const GH_API: &str = "https://api.github.com";

#[derive(Clone)]
pub struct GitHubPrClient {
    client: Client,
    token: Option<String>,
    user_agent: String,
}

impl GitHubPrClient {
    pub fn new(client: Client, token: Option<String>, user_agent: impl Into<String>) -> Self {
        Self {
            client,
            token: token
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            user_agent: user_agent.into(),
        }
    }

    pub fn with_env_token(client: Client, user_agent: impl Into<String>) -> Self {
        Self::new(client, github_token_from_env(), user_agent)
    }

    pub fn token_configured(&self) -> bool {
        self.token.is_some()
    }

    fn token_required(&self) -> Result<&str> {
        self.token
            .as_deref()
            .ok_or_else(|| anyhow!("GitHub token is required for this operation"))
    }

    fn headers(&self, accept: &str, require_token: bool) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_str(&self.user_agent)?);
        headers.insert("X-GitHub-Api-Version", HeaderValue::from_static("2022-11-28"));
        headers.insert(ACCEPT, HeaderValue::from_str(accept)?);

        if require_token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", self.token_required()?))?,
            );
        } else if let Some(token) = self.token.as_deref() {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}"))?,
            );
        }

        Ok(headers)
    }

    async fn get_json(&self, path: &str) -> Result<Value> {
        let response = self
            .client
            .get(format!("{GH_API}{path}"))
            .headers(self.headers("application/vnd.github+json", false)?)
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
            .with_context(|| format!("Failed to decode GitHub JSON for {path}"))
    }

    async fn get_text(&self, path: &str, accept: &str) -> Result<String> {
        let response = self
            .client
            .get(format!("{GH_API}{path}"))
            .headers(self.headers(accept, false)?)
            .send()
            .await
            .with_context(|| format!("GitHub request failed for {path}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("GitHub GET {path} -> {status}: {body}"));
        }

        response
            .text()
            .await
            .with_context(|| format!("Failed to decode GitHub text for {path}"))
    }

    async fn post_json(&self, path: &str, body: &Value) -> Result<Value> {
        let response = self
            .client
            .post(format!("{GH_API}{path}"))
            .headers(self.headers("application/vnd.github+json", true)?)
            .json(body)
            .send()
            .await
            .with_context(|| format!("GitHub POST failed for {path}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("GitHub POST {path} -> {status}: {text}"));
        }

        if response.status() == reqwest::StatusCode::NO_CONTENT {
            Ok(json!({}))
        } else {
            response
                .json::<Value>()
                .await
                .with_context(|| format!("Failed to decode GitHub JSON for {path}"))
        }
    }

    async fn patch_json(&self, path: &str, body: &Value) -> Result<Value> {
        let response = self
            .client
            .patch(format!("{GH_API}{path}"))
            .headers(self.headers("application/vnd.github+json", true)?)
            .json(body)
            .send()
            .await
            .with_context(|| format!("GitHub PATCH failed for {path}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("GitHub PATCH {path} -> {status}: {text}"));
        }

        response
            .json::<Value>()
            .await
            .with_context(|| format!("Failed to decode GitHub JSON for {path}"))
    }

    pub async fn fetch_pull_request(&self, repo: &str, pr_number: i64) -> Result<GitHubPullRequest> {
        let value = self
            .get_json(&format!("/repos/{repo}/pulls/{pr_number}"))
            .await?;

        Ok(GitHubPullRequest {
            repo: repo.to_string(),
            number: pr_number,
            title: value["title"].as_str().unwrap_or("").to_string(),
            html_url: value["html_url"].as_str().unwrap_or("").to_string(),
            head_repo: value["head"]["repo"]["full_name"]
                .as_str()
                .unwrap_or(repo)
                .to_string(),
            head_sha: value["head"]["sha"].as_str().unwrap_or("").to_string(),
            head_ref: value["head"]["ref"].as_str().unwrap_or("").to_string(),
            base_ref: value["base"]["ref"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn fetch_pull_request_diff(&self, repo: &str, pr_number: i64) -> Result<String> {
        self.get_text(
            &format!("/repos/{repo}/pulls/{pr_number}"),
            "application/vnd.github.v3.diff",
        )
        .await
    }

    pub async fn create_check_run(
        &self,
        repo: &str,
        request: GitHubCheckRunRequest,
    ) -> Result<GitHubCheckRunResult> {
        let body = json!({
            "name": request.name,
            "head_sha": request.head_sha,
            "status": "completed",
            "conclusion": request.conclusion,
            "external_id": if request.external_id.trim().is_empty() { Value::Null } else { json!(request.external_id) },
            "details_url": request.details_url,
            "output": {
                "title": request.title,
                "summary": request.summary,
                "text": request.text,
            }
        });

        let value = self
            .post_json(&format!("/repos/{repo}/check-runs"), &body)
            .await?;
        Ok(GitHubCheckRunResult {
            html_url: value["html_url"].as_str().unwrap_or("").to_string(),
            api_url: value["url"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn create_commit_status(
        &self,
        repo: &str,
        request: GitHubCommitStatusRequest,
    ) -> Result<GitHubCommitStatusResult> {
        let body = json!({
            "state": request.state,
            "context": request.context,
            "description": request.description,
            "target_url": request.target_url,
        });

        let value = self
            .post_json(&format!("/repos/{repo}/statuses/{}", request.sha), &body)
            .await?;
        Ok(GitHubCommitStatusResult {
            url: value["url"].as_str().unwrap_or("").to_string(),
        })
    }

    pub async fn upsert_issue_comment(
        &self,
        repo: &str,
        issue_number: i64,
        marker: &str,
        body: &str,
    ) -> Result<GitHubManagedCommentResult> {
        let comments = self
            .get_json(&format!("/repos/{repo}/issues/{issue_number}/comments?per_page=100"))
            .await?;

        if let Some(existing) = comments.as_array().and_then(|items| {
            items.iter().find(|item| {
                item["body"]
                    .as_str()
                    .map(|text| text.contains(marker))
                    .unwrap_or(false)
            })
        }) {
            let id = existing["id"]
                .as_i64()
                .ok_or_else(|| anyhow!("Existing managed comment was missing an id"))?;
            let updated = self
                .patch_json(
                    &format!("/repos/{repo}/issues/comments/{id}"),
                    &json!({ "body": body }),
                )
                .await?;
            return Ok(GitHubManagedCommentResult {
                mode: "updated".into(),
                html_url: updated["html_url"].as_str().unwrap_or("").to_string(),
            });
        }

        let created = self
            .post_json(
                &format!("/repos/{repo}/issues/{issue_number}/comments"),
                &json!({ "body": body }),
            )
            .await?;
        Ok(GitHubManagedCommentResult {
            mode: "created".into(),
            html_url: created["html_url"].as_str().unwrap_or("").to_string(),
        })
    }
}

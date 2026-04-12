use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
    Client,
};
use serde_json::{json, Value};

use crate::{
    models::{
        GitHubCheckRunRequest, GitHubCheckRunResult, GitHubCommitStatusRequest,
        GitHubCommitHealth, GitHubCommitStatusResult, GitHubManagedCommentResult,
        GitHubPullRequest, GitHubPullReview, GitHubPullReviewThread,
        GitHubPullReviewThreadComment, GitHubCheckRunSummary, GitHubStatusContext,
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

    async fn post_json_with_headers(
        &self,
        path: &str,
        body: &Value,
        accept: &str,
    ) -> Result<Value> {
        let response = self
            .client
            .post(format!("{GH_API}{path}"))
            .headers(self.headers(accept, true)?)
            .json(body)
            .send()
            .await
            .with_context(|| format!("GitHub POST failed for {path}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("GitHub POST {path} -> {status}: {text}"));
        }

        response
            .json::<Value>()
            .await
            .with_context(|| format!("Failed to decode GitHub JSON for {path}"))
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
            state: value["state"].as_str().unwrap_or("").to_string(),
            merged: value["merged"].as_bool().unwrap_or(false),
            draft: value["draft"].as_bool().unwrap_or(false),
            mergeable: value["mergeable"].as_bool(),
            mergeable_state: value["mergeable_state"].as_str().unwrap_or("").to_string(),
            title: value["title"].as_str().unwrap_or("").to_string(),
            html_url: value["html_url"].as_str().unwrap_or("").to_string(),
            head_repo: value["head"]["repo"]["full_name"]
                .as_str()
                .unwrap_or(repo)
                .to_string(),
            head_sha: value["head"]["sha"].as_str().unwrap_or("").to_string(),
            head_ref: value["head"]["ref"].as_str().unwrap_or("").to_string(),
            base_ref: value["base"]["ref"].as_str().unwrap_or("").to_string(),
            additions: value["additions"].as_u64().unwrap_or(0) as u32,
            deletions: value["deletions"].as_u64().unwrap_or(0) as u32,
            changed_files: value["changed_files"].as_u64().unwrap_or(0) as u32,
        })
    }

    pub async fn fetch_commit_health(&self, repo: &str, sha: &str) -> Result<GitHubCommitHealth> {
        let sha = sha.trim();
        if sha.is_empty() {
            return Err(anyhow!("Commit SHA is required for commit health lookup"));
        }

        let statuses_value = self
            .get_json(&format!("/repos/{repo}/commits/{sha}/status"))
            .await?;
        let check_runs_value = self
            .get_json(&format!("/repos/{repo}/commits/{sha}/check-runs?per_page=100"))
            .await?;

        let status_items = statuses_value["statuses"]
            .as_array()
            .ok_or_else(|| anyhow!("GitHub combined status response was not an array"))?;
        let check_items = check_runs_value["check_runs"]
            .as_array()
            .ok_or_else(|| anyhow!("GitHub check-runs response was not an array"))?;

        let statuses = status_items
            .iter()
            .map(|item| GitHubStatusContext {
                context: item["context"].as_str().unwrap_or("").to_string(),
                state: item["state"].as_str().unwrap_or("").to_string(),
                description: item["description"].as_str().unwrap_or("").to_string(),
                target_url: item["target_url"].as_str().unwrap_or("").to_string(),
            })
            .collect::<Vec<_>>();
        let check_runs = check_items
            .iter()
            .map(|item| GitHubCheckRunSummary {
                name: item["name"].as_str().unwrap_or("").to_string(),
                status: item["status"].as_str().unwrap_or("").to_string(),
                conclusion: item["conclusion"].as_str().unwrap_or("").to_string(),
                html_url: item["html_url"].as_str().unwrap_or("").to_string(),
            })
            .collect::<Vec<_>>();

        let mut health = GitHubCommitHealth {
            combined_state: statuses_value["state"].as_str().unwrap_or("").to_string(),
            statuses,
            check_runs,
            ..GitHubCommitHealth::default()
        };

        for status in &health.statuses {
            match status.state.as_str() {
                "success" => health.successful_contexts += 1,
                "pending" => health.pending_contexts += 1,
                "failure" | "error" => health.failing_contexts += 1,
                _ => health.neutral_contexts += 1,
            }
        }

        for run in &health.check_runs {
            match (run.status.as_str(), run.conclusion.as_str()) {
                ("completed", "success") => health.successful_checks += 1,
                ("completed", "neutral" | "skipped") => health.neutral_checks += 1,
                ("completed", "failure" | "timed_out" | "cancelled" | "action_required" | "startup_failure" | "stale") => {
                    health.failing_checks += 1
                }
                ("completed", _) => health.neutral_checks += 1,
                _ => health.pending_checks += 1,
            }
        }

        Ok(health)
    }

    pub async fn fetch_pull_request_diff(&self, repo: &str, pr_number: i64) -> Result<String> {
        self.get_text(
            &format!("/repos/{repo}/pulls/{pr_number}"),
            "application/vnd.github.v3.diff",
        )
        .await
    }

    pub async fn fetch_pull_request_reviews(
        &self,
        repo: &str,
        pr_number: i64,
    ) -> Result<Vec<GitHubPullReview>> {
        let value = self
            .get_json(&format!("/repos/{repo}/pulls/{pr_number}/reviews?per_page=100"))
            .await?;

        let items = value
            .as_array()
            .ok_or_else(|| anyhow!("GitHub reviews response was not an array"))?;

        Ok(items
            .iter()
            .map(|item| GitHubPullReview {
                id: item["id"].as_i64().unwrap_or(0),
                state: item["state"].as_str().unwrap_or("").to_string(),
                body: item["body"].as_str().unwrap_or("").to_string(),
                html_url: item["html_url"].as_str().unwrap_or("").to_string(),
                submitted_at: item["submitted_at"].as_str().unwrap_or("").to_string(),
                author_login: item["user"]["login"].as_str().unwrap_or("").to_string(),
            })
            .collect())
    }

    pub async fn fetch_pull_request_review_threads(
        &self,
        repo: &str,
        pr_number: i64,
    ) -> Result<Vec<GitHubPullReviewThread>> {
        let (owner, name) = split_repo(repo)?;
        let query = r#"
query PatchHiveReviewThreads($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      reviewThreads(first: 100) {
        nodes {
          id
          path
          isResolved
          isOutdated
          comments(first: 30) {
            nodes {
              id
              body
              url
              createdAt
              author {
                login
              }
            }
          }
        }
      }
    }
  }
}
"#;

        let value = self
            .post_json_with_headers(
                "/graphql",
                &json!({
                    "query": query,
                    "variables": {
                        "owner": owner,
                        "name": name,
                        "number": pr_number,
                    }
                }),
                "application/vnd.github+json",
            )
            .await?;

        if let Some(errors) = value["errors"].as_array() {
            if !errors.is_empty() {
                let messages = errors
                    .iter()
                    .filter_map(|error| error["message"].as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(anyhow!("GitHub GraphQL error: {messages}"));
            }
        }

        let threads = value["data"]["repository"]["pullRequest"]["reviewThreads"]["nodes"]
            .as_array()
            .ok_or_else(|| anyhow!("GitHub review thread response was not an array"))?;

        Ok(threads
            .iter()
            .map(|thread| GitHubPullReviewThread {
                id: thread["id"].as_str().unwrap_or("").to_string(),
                path: thread["path"].as_str().unwrap_or("").to_string(),
                is_resolved: thread["isResolved"].as_bool().unwrap_or(false),
                is_outdated: thread["isOutdated"].as_bool().unwrap_or(false),
                comments: thread["comments"]["nodes"]
                    .as_array()
                    .map(|comments| {
                        comments
                            .iter()
                            .map(|comment| GitHubPullReviewThreadComment {
                                id: comment["id"].as_str().unwrap_or("").to_string(),
                                body: comment["body"].as_str().unwrap_or("").to_string(),
                                url: comment["url"].as_str().unwrap_or("").to_string(),
                                created_at: comment["createdAt"].as_str().unwrap_or("").to_string(),
                                author_login: comment["author"]["login"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string(),
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            })
            .collect())
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

fn split_repo(repo: &str) -> Result<(&str, &str)> {
    let mut parts = repo.split('/');
    let owner = parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("Repository owner was missing"))?;
    let name = parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("Repository name was missing"))?;

    if parts.next().is_some() {
        return Err(anyhow!("Repository must be in owner/name format"));
    }

    Ok((owner, name))
}

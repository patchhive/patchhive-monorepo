use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT},
    Client,
};
use serde_json::{json, Value};

use crate::models::{GitHubReportOutcome, ReviewResult};

const GH_API: &str = "https://api.github.com";
const STATUS_CONTEXT: &str = "trustgate/recommendation";
const CHECK_RUN_NAME: &str = "TrustGate";

pub fn github_token() -> Option<String> {
    std::env::var("BOT_GITHUB_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("GITHUB_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
}

pub fn github_token_configured() -> bool {
    github_token().is_some()
}

pub fn webhook_secret() -> Option<String> {
    std::env::var("TRUST_GITHUB_WEBHOOK_SECRET")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

pub fn webhook_secret_configured() -> bool {
    webhook_secret().is_some()
}

fn gh_headers(token: Option<&str>, accept: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("trust-gate/0.1"));
    headers.insert("X-GitHub-Api-Version", HeaderValue::from_static("2022-11-28"));
    headers.insert(ACCEPT, HeaderValue::from_str(accept)?);

    if let Some(token) = token.filter(|value| !value.trim().is_empty()) {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))?,
        );
    }

    Ok(headers)
}

async fn gh_get_json(client: &Client, path: &str, token: Option<&str>) -> Result<Value> {
    let response = client
        .get(format!("{GH_API}{path}"))
        .headers(gh_headers(token, "application/vnd.github+json")?)
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

async fn gh_get_text(client: &Client, path: &str, accept: &str, token: Option<&str>) -> Result<String> {
    let response = client
        .get(format!("{GH_API}{path}"))
        .headers(gh_headers(token, accept)?)
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

async fn gh_post(client: &Client, path: &str, body: &Value, token: &str) -> Result<Value> {
    let response = client
        .post(format!("{GH_API}{path}"))
        .headers(gh_headers(Some(token), "application/vnd.github+json")?)
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

pub async fn fetch_pull_request(client: &Client, repo: &str, pr_number: i64) -> Result<Value> {
    let token = github_token();
    gh_get_json(client, &format!("/repos/{repo}/pulls/{pr_number}"), token.as_deref()).await
}

pub async fn fetch_pull_request_diff(client: &Client, repo: &str, pr_number: i64) -> Result<String> {
    let token = github_token();
    gh_get_text(
        client,
        &format!("/repos/{repo}/pulls/{pr_number}"),
        "application/vnd.github.v3.diff",
        token.as_deref(),
    )
    .await
}

fn details_url(review: &ReviewResult) -> Option<String> {
    let base = std::env::var("TRUSTGATE_PUBLIC_URL").ok()?;
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    Some(format!("{trimmed}/history/{}", review.id))
}

fn check_conclusion(review: &ReviewResult) -> &'static str {
    match review.recommendation.as_str() {
        "safe" => "success",
        "warn" => "action_required",
        _ => "failure",
    }
}

fn commit_state(review: &ReviewResult) -> &'static str {
    match review.recommendation.as_str() {
        "safe" => "success",
        "warn" => "pending",
        _ => "failure",
    }
}

fn check_output_text(review: &ReviewResult) -> String {
    if review.findings.is_empty() {
        return "TrustGate found no active warnings against the current repo rules.".into();
    }

    review
        .findings
        .iter()
        .take(10)
        .map(|finding| {
            if finding.evidence.is_empty() {
                format!("- [{}] {}: {}", finding.severity, finding.label, finding.detail)
            } else {
                format!(
                    "- [{}] {}: {} ({})",
                    finding.severity,
                    finding.label,
                    finding.detail,
                    finding.evidence.join("; ")
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn publish_review_outcome(client: &Client, review: &ReviewResult) -> GitHubReportOutcome {
    let Some(github) = review.github.as_ref() else {
        return GitHubReportOutcome {
            attempted: false,
            delivered: false,
            method: "none".into(),
            state: "skipped".into(),
            message: "This review was not tied to a GitHub pull request.".into(),
            details: Vec::new(),
        };
    };

    let Some(token) = github_token() else {
        return GitHubReportOutcome {
            attempted: true,
            delivered: false,
            method: "none".into(),
            state: "missing_token".into(),
            message: "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required to report TrustGate results back to GitHub.".into(),
            details: vec![
                "PR diff ingestion still works for public repos without a token.".into(),
                "GitHub status/check publishing is disabled until a token is configured.".into(),
            ],
        };
    };

    let target_repo = if github.head_repo.trim().is_empty() {
        review.repo.as_str()
    } else {
        github.head_repo.as_str()
    };

    let mut details = Vec::new();
    let check_body = json!({
        "name": CHECK_RUN_NAME,
        "head_sha": github.head_sha,
        "status": "completed",
        "conclusion": check_conclusion(review),
        "external_id": review.id,
        "details_url": details_url(review),
        "output": {
            "title": format!("TrustGate: {}", review.recommendation.to_uppercase()),
            "summary": review.summary,
            "text": check_output_text(review),
        }
    });

    match gh_post(
        client,
        &format!("/repos/{target_repo}/check-runs"),
        &check_body,
        &token,
    )
    .await
    {
        Ok(value) => {
            let html_url = value["html_url"].as_str().unwrap_or("").to_string();
            details.push(if html_url.is_empty() {
                "Created a GitHub check run.".into()
            } else {
                format!("Created GitHub check run: {html_url}")
            });
            return GitHubReportOutcome {
                attempted: true,
                delivered: true,
                method: "check_run".into(),
                state: review.recommendation.clone(),
                message: "TrustGate posted a GitHub check run for this PR.".into(),
                details,
            };
        }
        Err(err) => details.push(format!("Check run failed, falling back to commit status: {err}")),
    }

    let status_body = json!({
        "state": commit_state(review),
        "context": STATUS_CONTEXT,
        "description": match review.recommendation.as_str() {
            "safe" => "TrustGate marked this diff safe.",
            "warn" => "TrustGate found warnings that need review.",
            _ => "TrustGate found blocking issues.",
        },
        "target_url": details_url(review),
    });

    match gh_post(
        client,
        &format!("/repos/{target_repo}/statuses/{}", github.head_sha),
        &status_body,
        &token,
    )
    .await
    {
        Ok(_) => {
            details.push("Created a commit status fallback.".into());
            GitHubReportOutcome {
                attempted: true,
                delivered: true,
                method: "commit_status".into(),
                state: review.recommendation.clone(),
                message: "TrustGate posted a GitHub commit status for this PR.".into(),
                details,
            }
        }
        Err(err) => {
            details.push(format!("Commit status failed: {err}"));
            GitHubReportOutcome {
                attempted: true,
                delivered: false,
                method: "none".into(),
                state: "report_failed".into(),
                message: "TrustGate reviewed the PR but could not post the result back to GitHub.".into(),
                details,
            }
        }
    }
}

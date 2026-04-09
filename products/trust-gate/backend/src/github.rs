use anyhow::{anyhow, Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT},
    Client,
};
use serde_json::{json, Value};

use crate::models::{GitHubReportOutcome, ReviewResult};

const GH_API: &str = "https://api.github.com";

pub fn github_token() -> Option<String> {
    std::env::var("BOT_GITHUB_TOKEN")
        .ok()
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn webhook_secret() -> Option<String> {
    std::env::var("TRUST_GITHUB_WEBHOOK_SECRET")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn github_headers(token: Option<&str>, accept: &str) -> Result<HeaderMap> {
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

pub async fn get_json(client: &Client, path: &str, token: Option<&str>) -> Result<Value> {
    let response = client
        .get(format!("{GH_API}{path}"))
        .headers(github_headers(token, "application/vnd.github+json")?)
        .send()
        .await
        .with_context(|| format!("GitHub request failed for {path}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("GitHub request failed for {path}: {status} {body}"));
    }

    response
        .json()
        .await
        .with_context(|| format!("Failed to decode GitHub JSON for {path}"))
}

pub async fn get_text(client: &Client, path: &str, accept: &str, token: Option<&str>) -> Result<String> {
    let response = client
        .get(format!("{GH_API}{path}"))
        .headers(github_headers(token, accept)?)
        .send()
        .await
        .with_context(|| format!("GitHub request failed for {path}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("GitHub request failed for {path}: {status} {body}"));
    }

    response
        .text()
        .await
        .with_context(|| format!("Failed to decode GitHub text for {path}"))
}

fn report_target_url(review: &ReviewResult) -> Option<String> {
    let base = std::env::var("TRUSTGATE_PUBLIC_URL").ok()?;
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    Some(format!("{trimmed}/history/{}", review.id))
}

fn check_run_conclusion(recommendation: &str) -> &'static str {
    match recommendation {
        "safe" => "success",
        "warn" => "action_required",
        _ => "failure",
    }
}

fn commit_status_state(recommendation: &str) -> &'static str {
    match recommendation {
        "safe" => "success",
        "warn" => "pending",
        _ => "failure",
    }
}

fn report_body(review: &ReviewResult) -> String {
    if review.findings.is_empty() {
        return "TrustGate did not surface active warnings for this diff.".into();
    }

    review
        .findings
        .iter()
        .take(8)
        .map(|finding| {
            let evidence = if finding.evidence.is_empty() {
                String::new()
            } else {
                format!("\n- {}", finding.evidence.join("\n- "))
            };
            format!(
                "### {} [{}]\n{}\n{}",
                finding.label,
                finding.severity,
                finding.detail,
                evidence
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

async fn create_check_run(client: &Client, repo: &str, head_sha: &str, review: &ReviewResult) -> Result<()> {
    let token = github_token().ok_or_else(|| anyhow!("GitHub token is not configured"))?;
    let mut body = json!({
        "name": "TrustGate",
        "head_sha": head_sha,
        "status": "completed",
        "conclusion": check_run_conclusion(&review.recommendation),
        "external_id": review.id,
        "output": {
            "title": format!("TrustGate: {}", review.recommendation.to_uppercase()),
            "summary": review.summary,
            "text": report_body(review),
        },
    });

    if let Some(target_url) = report_target_url(review) {
        body["details_url"] = Value::String(target_url);
    }

    let response = client
        .post(format!("{GH_API}/repos/{repo}/check-runs"))
        .headers(github_headers(Some(&token), "application/vnd.github+json")?)
        .json(&body)
        .send()
        .await
        .context("failed to create GitHub check run")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("GitHub check run failed: {status} {text}"));
    }

    Ok(())
}

async fn create_commit_status(client: &Client, repo: &str, head_sha: &str, review: &ReviewResult) -> Result<()> {
    let token = github_token().ok_or_else(|| anyhow!("GitHub token is not configured"))?;
    let mut body = json!({
        "state": commit_status_state(&review.recommendation),
        "context": "trustgate/recommendation",
        "description": review.summary.chars().take(140).collect::<String>(),
    });

    if let Some(target_url) = report_target_url(review) {
        body["target_url"] = Value::String(target_url);
    }

    let response = client
        .post(format!("{GH_API}/repos/{repo}/statuses/{head_sha}"))
        .headers(github_headers(Some(&token), "application/vnd.github+json")?)
        .json(&body)
        .send()
        .await
        .context("failed to create GitHub commit status")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("GitHub commit status failed: {status} {text}"));
    }

    Ok(())
}

pub async fn publish_review_outcome(client: &Client, review: &ReviewResult) -> GitHubReportOutcome {
    let Some(github) = review.github.as_ref() else {
        return GitHubReportOutcome {
            attempted: false,
            delivered: false,
            method: "none".into(),
            state: "not_applicable".into(),
            message: "This review was not tied to a GitHub pull request.".into(),
            details: vec![],
        };
    };

    let target_repo = if github.head_repo.trim().is_empty() {
        github.repo.clone()
    } else {
        github.head_repo.clone()
    };

    if github.head_sha.trim().is_empty() {
        return GitHubReportOutcome {
            attempted: true,
            delivered: false,
            method: "none".into(),
            state: "missing_head_sha".into(),
            message: "TrustGate could not determine the PR head SHA for GitHub reporting.".into(),
            details: vec![],
        };
    }

    if github_token().is_none() {
        return GitHubReportOutcome {
            attempted: true,
            delivered: false,
            method: "none".into(),
            state: "missing_token".into(),
            message: "GitHub token missing. TrustGate reviewed the PR but could not publish the decision back to GitHub.".into(),
            details: vec![
                "Set BOT_GITHUB_TOKEN or GITHUB_TOKEN to enable status/check output.".into(),
            ],
        };
    }

    let mut details = Vec::new();
    match create_check_run(client, &target_repo, &github.head_sha, review).await {
        Ok(()) => GitHubReportOutcome {
            attempted: true,
            delivered: true,
            method: "check_run".into(),
            state: review.recommendation.clone(),
            message: "Published a TrustGate check run to the PR head commit.".into(),
            details,
        },
        Err(check_err) => {
            details.push(format!("Check run fallback: {check_err}"));
            match create_commit_status(client, &target_repo, &github.head_sha, review).await {
                Ok(()) => GitHubReportOutcome {
                    attempted: true,
                    delivered: true,
                    method: "commit_status".into(),
                    state: review.recommendation.clone(),
                    message: "Published a TrustGate commit status to the PR head commit.".into(),
                    details,
                },
                Err(status_err) => {
                    details.push(format!("Commit status failed: {status_err}"));
                    GitHubReportOutcome {
                        attempted: true,
                        delivered: false,
                        method: "none".into(),
                        state: "delivery_failed".into(),
                        message: "TrustGate reviewed the PR, but GitHub rejected both check-run and commit-status output.".into(),
                        details,
                    }
                }
            }
        }
    }
}

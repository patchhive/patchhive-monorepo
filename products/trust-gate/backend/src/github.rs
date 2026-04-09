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
const COMMENT_MARKER: &str = "<!-- patchhive-trustgate-report -->";

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

async fn gh_patch(client: &Client, path: &str, body: &Value, token: &str) -> Result<Value> {
    let response = client
        .patch(format!("{GH_API}{path}"))
        .headers(gh_headers(Some(token), "application/vnd.github+json")?)
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

fn recommendation_emoji(review: &ReviewResult) -> &'static str {
    match review.recommendation.as_str() {
        "safe" => "🟢",
        "warn" => "🟡",
        _ => "🔴",
    }
}

fn check_summary(review: &ReviewResult) -> String {
    let metrics = &review.metrics;
    format!(
        "{emoji} TrustGate recommends **{rec}** for this PR.\n\n{summary}\n\nFiles changed: **{files}**  |  Additions: **+{adds}**  |  Deletions: **-{dels}**  |  Tests changed: **{tests}**  |  Generated files: **{generated}**",
        emoji = recommendation_emoji(review),
        rec = review.recommendation.to_uppercase(),
        summary = review.summary,
        files = metrics.files_changed,
        adds = metrics.additions,
        dels = metrics.deletions,
        tests = metrics.tests_changed,
        generated = metrics.generated_files,
    )
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

fn markdown_findings(review: &ReviewResult) -> String {
    if review.findings.is_empty() {
        return "- No active warnings.\n".into();
    }

    review
        .findings
        .iter()
        .take(8)
        .map(|finding| {
            let evidence = if finding.evidence.is_empty() {
                String::new()
            } else {
                format!(" Evidence: {}.", finding.evidence.join("; "))
            };
            format!(
                "- **{}** (`{}`): {}.{}",
                finding.label, finding.severity, finding.detail, evidence
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_top_files(review: &ReviewResult) -> String {
    let top = review
        .files
        .iter()
        .filter(|file| file.status != "safe" || file.generated || !file.path_policy.is_empty())
        .take(6)
        .map(|file| {
            let mut suffix = Vec::new();
            if file.generated {
                suffix.push("generated".to_string());
            }
            if !file.path_policy.is_empty() {
                suffix.push(file.path_policy.clone());
            }
            let extra = if suffix.is_empty() {
                String::new()
            } else {
                format!(" — {}", suffix.join(" | "))
            };
            format!("- `{}`: **{}**{}", file.path, file.status, extra)
        })
        .collect::<Vec<_>>();

    if top.is_empty() {
        "- No file-level hotspots beyond the current summary.".into()
    } else {
        top.join("\n")
    }
}

fn build_pr_comment(review: &ReviewResult) -> String {
    let details = details_url(review)
        .map(|url| format!("[Open TrustGate review]({url})"))
        .unwrap_or_else(|| "TrustGate review details are local to the current PatchHive host.".into());

    format!(
        "{marker}\n## {emoji} TrustGate: {rec}\n\n{summary}\n\n### Risk snapshot\n- Risk score: **{score}**\n- Files changed: **{files}**\n- Additions / deletions: **+{adds} / -{dels}**\n- Tests changed: **{tests}**\n- Generated files: **{generated}**\n- Blocking findings: **{blocks}**\n- Warning findings: **{warns}**\n\n### Findings\n{findings}\n\n### File hotspots\n{files_section}\n\n### Next move\n{next_move}\n\n{details}\n\n*TrustGate by PatchHive*",
        marker = COMMENT_MARKER,
        emoji = recommendation_emoji(review),
        rec = review.recommendation.to_uppercase(),
        summary = review.summary,
        score = review.risk_score,
        files = review.metrics.files_changed,
        adds = review.metrics.additions,
        dels = review.metrics.deletions,
        tests = review.metrics.tests_changed,
        generated = review.metrics.generated_files,
        blocks = review.metrics.blocked_findings,
        warns = review.metrics.warning_findings,
        findings = markdown_findings(review),
        files_section = markdown_top_files(review),
        next_move = match review.recommendation.as_str() {
            "safe" => "This patch is within the current repo rules. Review normally, but TrustGate did not find a reason to stop it.",
            "warn" => "A human should look at the flagged areas before merge. The patch may still be fine, but it no longer looks routine.",
            _ => "Do not move this patch forward yet. The repo rules say the current risk profile is too high without intervention.",
        },
        details = details,
    )
}

async fn upsert_pr_comment(
    client: &Client,
    repo: &str,
    pr_number: i64,
    body: &str,
    token: &str,
) -> Result<(String, String)> {
    let comments = gh_get_json(
        client,
        &format!("/repos/{repo}/issues/{pr_number}/comments?per_page=100"),
        Some(token),
    )
    .await?;

    if let Some(existing) = comments.as_array().and_then(|items| {
        items.iter().find(|item| {
            item["body"]
                .as_str()
                .map(|text| text.contains(COMMENT_MARKER))
                .unwrap_or(false)
        })
    }) {
        let id = existing["id"]
            .as_i64()
            .ok_or_else(|| anyhow!("Existing TrustGate comment was missing an id"))?;
        let updated = gh_patch(
            client,
            &format!("/repos/{repo}/issues/comments/{id}"),
            &json!({ "body": body }),
            token,
        )
        .await?;
        return Ok((
            "updated".into(),
            updated["html_url"].as_str().unwrap_or("").to_string(),
        ));
    }

    let created = gh_post(
        client,
        &format!("/repos/{repo}/issues/{pr_number}/comments"),
        &json!({ "body": body }),
        token,
    )
    .await?;
    Ok((
        "created".into(),
        created["html_url"].as_str().unwrap_or("").to_string(),
    ))
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
            check_url: String::new(),
            status_url: String::new(),
            comment_url: String::new(),
            comment_mode: String::new(),
            report_markdown: String::new(),
        };
    };

    let report_markdown = build_pr_comment(review);

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
            check_url: String::new(),
            status_url: String::new(),
            comment_url: String::new(),
            comment_mode: String::new(),
            report_markdown,
        };
    };

    let target_repo = if github.head_repo.trim().is_empty() {
        review.repo.as_str()
    } else {
        github.head_repo.as_str()
    };

    let mut details = Vec::new();
    let mut method = "none".to_string();
    let mut delivered = false;
    let mut check_url = String::new();
    let mut status_url = String::new();
    let mut comment_url = String::new();
    let mut comment_mode = String::new();

    let check_body = json!({
        "name": CHECK_RUN_NAME,
        "head_sha": github.head_sha,
        "status": "completed",
        "conclusion": check_conclusion(review),
        "external_id": review.id,
        "details_url": details_url(review),
        "output": {
            "title": format!("TrustGate: {}", review.recommendation.to_uppercase()),
            "summary": check_summary(review),
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
            check_url = value["html_url"].as_str().unwrap_or("").to_string();
            details.push(if check_url.is_empty() {
                "Created a GitHub check run.".into()
            } else {
                format!("Created GitHub check run: {check_url}")
            });
            method = "check_run".into();
            delivered = true;
        }
        Err(err) => details.push(format!("Check run failed, falling back to commit status: {err}")),
    }

    if !delivered {
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
            Ok(value) => {
                status_url = value["url"].as_str().unwrap_or("").to_string();
                details.push(if status_url.is_empty() {
                    "Created a commit status fallback.".into()
                } else {
                    format!("Created commit status fallback: {status_url}")
                });
                method = "commit_status".into();
                delivered = true;
            }
            Err(err) => {
                details.push(format!("Commit status failed: {err}"));
            }
        }
    }

    match upsert_pr_comment(client, &github.repo, github.pr_number, &report_markdown, &token).await {
        Ok((mode, url)) => {
            comment_mode = mode;
            comment_url = url.clone();
            if method == "none" {
                method = "pr_comment".into();
            }
            details.push(if url.is_empty() {
                format!("{} TrustGate PR comment.", comment_mode)
            } else {
                format!("{} TrustGate PR comment: {url}", comment_mode)
            });
            delivered = true;
        }
        Err(err) => details.push(format!("PR comment upsert failed: {err}")),
    }

    GitHubReportOutcome {
        attempted: true,
        delivered,
        method,
        state: if delivered {
            review.recommendation.clone()
        } else {
            "report_failed".into()
        },
        message: if delivered {
            "TrustGate published its review back to GitHub with a maintained PR comment and status signal.".into()
        } else {
            "TrustGate reviewed the PR but could not publish the result back to GitHub.".into()
        },
        details,
        check_url,
        status_url,
        comment_url,
        comment_mode,
        report_markdown,
    }
}

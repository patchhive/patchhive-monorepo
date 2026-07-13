use anyhow::{anyhow, Context, Result};
use patchhive_product_core::repo_memory::{
    fetch_repo_memory_context, repo_memory_url, RepoMemoryContextRequest,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::{RepoMemoryContextPreview, ReviewBeeContext, TrustGateContext};

#[derive(Debug, Deserialize)]
struct ReviewBeeChecklistItem {
    #[serde(default)]
    title: String,
    #[serde(default)]
    status: String,
}

#[derive(Debug, Default, Deserialize)]
struct ReviewBeeMetrics {
    #[serde(default)]
    actionable_threads: u32,
    #[serde(default)]
    open_items: u32,
}

#[derive(Debug, Deserialize)]
struct ReviewBeeResult {
    #[serde(default)]
    status: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    metrics: ReviewBeeMetrics,
    #[serde(default)]
    checklist: Vec<ReviewBeeChecklistItem>,
}

#[derive(Debug, Default, Deserialize)]
struct TrustGateMetrics {
    #[serde(default)]
    blocked_findings: u32,
    #[serde(default)]
    warning_findings: u32,
}

#[derive(Debug, Deserialize)]
struct TrustGateFinding {
    #[serde(default)]
    label: String,
    #[serde(default)]
    severity: String,
    #[serde(default)]
    detail: String,
}

#[derive(Debug, Deserialize)]
struct TrustGateResult {
    #[serde(default)]
    recommendation: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    risk_score: u32,
    #[serde(default)]
    metrics: TrustGateMetrics,
    #[serde(default)]
    findings: Vec<TrustGateFinding>,
}

#[derive(Debug, Serialize)]
struct ReviewBeeRequest<'a> {
    repo: &'a str,
    pr_number: i64,
    publish_comment: bool,
}

#[derive(Debug, Serialize)]
struct TrustGateRequest<'a> {
    repo: &'a str,
    pr_number: i64,
    ai_source: &'a str,
    publish_status: bool,
}

fn trim_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn env_value(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

pub fn review_bee_url() -> Option<String> {
    env_value(&["PATCHHIVE_REVIEW_BEE_URL", "REVIEW_BEE_URL"]).map(|value| trim_url(&value))
}

pub fn trust_gate_url() -> Option<String> {
    env_value(&[
        "PATCHHIVE_TRUST_GATE_URL",
        "PATCHHIVE_TRUSTGATE_URL",
        "TRUST_GATE_URL",
        "TRUSTGATE_URL",
    ])
    .map(|value| trim_url(&value))
}

fn review_bee_api_key() -> Option<String> {
    env_value(&["PATCHHIVE_REVIEW_BEE_API_KEY", "REVIEW_BEE_API_KEY"])
}

fn trust_gate_api_key() -> Option<String> {
    env_value(&[
        "PATCHHIVE_TRUST_GATE_API_KEY",
        "PATCHHIVE_TRUSTGATE_API_KEY",
        "TRUST_GATE_API_KEY",
        "TRUSTGATE_API_KEY",
    ])
}

pub fn review_bee_configured() -> bool {
    review_bee_url().is_some()
}

pub fn trust_gate_configured() -> bool {
    trust_gate_url().is_some()
}

pub fn repo_memory_configured() -> bool {
    repo_memory_url().is_some()
}

async fn post_json<TReq: Serialize, TRes: for<'de> Deserialize<'de>>(
    client: &Client,
    url: String,
    api_key: Option<String>,
    body: &TReq,
) -> Result<TRes> {
    let mut request = client
        .post(url)
        .timeout(std::time::Duration::from_secs(30))
        .json(body);
    if let Some(api_key) = api_key {
        request = request.header("X-API-Key", api_key);
    }

    let response = request.send().await.context("Integration request failed")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Integration request failed: {status} {body}"));
    }

    response
        .json::<TRes>()
        .await
        .context("Could not decode integration response")
}

pub async fn fetch_review_bee_context(
    client: &Client,
    repo: &str,
    pr_number: i64,
) -> Result<Option<ReviewBeeContext>> {
    let Some(base_url) = review_bee_url() else {
        return Ok(None);
    };

    let response = post_json::<_, ReviewBeeResult>(
        client,
        format!("{base_url}/review/github/pr"),
        review_bee_api_key(),
        &ReviewBeeRequest {
            repo,
            pr_number,
            publish_comment: false,
        },
    )
    .await?;

    Ok(Some(ReviewBeeContext {
        status: response.status,
        summary: response.summary,
        open_items: response.metrics.open_items,
        actionable_threads: response.metrics.actionable_threads,
        top_items: response
            .checklist
            .into_iter()
            .filter(|item| item.status == "open" || item.status == "mixed")
            .take(4)
            .map(|item| item.title)
            .collect(),
    }))
}

pub async fn fetch_trust_gate_context(
    client: &Client,
    repo: &str,
    pr_number: i64,
) -> Result<Option<TrustGateContext>> {
    let Some(base_url) = trust_gate_url() else {
        return Ok(None);
    };

    let response = post_json::<_, TrustGateResult>(
        client,
        format!("{base_url}/review/github/pr"),
        trust_gate_api_key(),
        &TrustGateRequest {
            repo,
            pr_number,
            ai_source: "mergekeeper",
            publish_status: false,
        },
    )
    .await?;

    Ok(Some(TrustGateContext {
        recommendation: response.recommendation,
        summary: response.summary,
        risk_score: response.risk_score,
        blocked_findings: response.metrics.blocked_findings,
        warning_findings: response.metrics.warning_findings,
        top_findings: response
            .findings
            .into_iter()
            .take(4)
            .map(|finding| {
                format!(
                    "{} [{}]: {}",
                    finding.label,
                    if finding.severity.trim().is_empty() {
                        "signal"
                    } else {
                        finding.severity.as_str()
                    },
                    finding.detail
                )
            })
            .collect(),
    }))
}

pub async fn fetch_repo_memory_preview(
    client: &Client,
    request: &RepoMemoryContextRequest,
) -> Result<Option<RepoMemoryContextPreview>> {
    let Some(response) = fetch_repo_memory_context(client, request).await? else {
        return Ok(None);
    };

    let policy_entries = response
        .entries
        .iter()
        .filter(|entry| entry.disposition == "policy")
        .count() as u32;
    let pinned_entries = response.entries.iter().filter(|entry| entry.pinned).count() as u32;
    let failguard_warnings = response
        .guardrails
        .iter()
        .map(|guardrail| format!("{}: {}", guardrail.title, guardrail.instruction))
        .take(4)
        .collect();

    Ok(Some(RepoMemoryContextPreview {
        summary: response.summary,
        prompt_lines: response.prompt_lines.into_iter().take(4).collect(),
        policy_entries,
        pinned_entries,
        failguard_warnings,
        top_entries: response
            .entries
            .into_iter()
            .take(4)
            .map(|entry| {
                if entry.title.trim().is_empty() {
                    entry.detail
                } else {
                    format!("{}: {}", entry.title, entry.detail)
                }
            })
            .collect(),
    }))
}

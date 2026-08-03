use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
struct ReviewRequest<'a> {
    repo: &'a str,
    diff: &'a str,
    ai_source: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrustGateReview {
    pub id: String,
    pub recommendation: String,
    pub risk_score: u32,
    pub summary: String,
    #[serde(default)]
    pub findings: Vec<Value>,
}

impl TrustGateReview {
    pub fn permits_publication(&self) -> bool {
        self.recommendation.eq_ignore_ascii_case("safe")
    }
}

pub fn trust_gate_url() -> Option<String> {
    std::env::var("PATCHHIVE_TRUST_GATE_URL")
        .ok()
        .or_else(|| std::env::var("TRUST_GATE_URL").ok())
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var("PATCHHIVE_SUITE_BASE_URL")
                .ok()
                .and_then(|value| {
                    let base = value.trim().trim_end_matches('/');
                    (!base.is_empty()).then(|| format!("{base}/api/products/trust-gate"))
                })
        })
}

fn apply_auth(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    if let Some(token) = std::env::var("PATCHHIVE_TRUST_GATE_SERVICE_TOKEN")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    {
        return request.header(crate::auth::SERVICE_TOKEN_HEADER, token);
    }
    if let Some(key) = std::env::var("PATCHHIVE_TRUST_GATE_API_KEY")
        .ok()
        .or_else(|| std::env::var("TRUST_GATE_API_KEY").ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    {
        return request.header("X-API-Key", key);
    }
    request
}

/// Review a generated diff through TrustGate and fail closed on absence,
/// transport failure, malformed evidence, or any recommendation other than safe.
pub async fn require_safe_review(
    client: &Client,
    repository: &str,
    diff: &str,
    ai_source: &str,
) -> Result<TrustGateReview> {
    let base_url = trust_gate_url().ok_or_else(|| {
        anyhow!("PATCHHIVE_TRUST_GATE_URL is required before RepoReaper may publish a pull request")
    })?;
    let response = apply_auth(client.post(format!("{base_url}/review")))
        .json(&ReviewRequest {
            repo: repository,
            diff,
            ai_source,
        })
        .send()
        .await
        .context("TrustGate review request failed")?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("TrustGate review returned HTTP {status}: {body}"));
    }
    let review = serde_json::from_str::<TrustGateReview>(&body)
        .context("TrustGate returned malformed review evidence")?;
    if !review.permits_publication() {
        return Err(anyhow!(
            "TrustGate recommendation '{}' blocks publication: {}",
            review.recommendation,
            review.summary
        ));
    }
    Ok(review)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_safe_is_publication_authority() {
        let review = TrustGateReview {
            id: "review-1".into(),
            recommendation: "warn".into(),
            risk_score: 50,
            summary: "review required".into(),
            findings: Vec::new(),
        };
        assert!(!review.permits_publication());
    }
}

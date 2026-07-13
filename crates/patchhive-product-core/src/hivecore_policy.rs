use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepositoryPolicyDecisionRequest {
    pub repository: String,
    pub product: String,
    pub operation: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepositoryPolicyDecision {
    pub repository: String,
    pub product: String,
    pub operation: String,
    pub decision: String,
    pub reason: String,
    pub trusted: bool,
    pub operator_excluded: bool,
    pub public_opt_out_checked: bool,
    pub public_opted_out: bool,
    pub policy_version: String,
    pub evaluated_at: String,
}

impl RepositoryPolicyDecision {
    pub fn allowed(&self) -> bool {
        self.decision == "allowed"
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrReservationRequest {
    pub product: String,
    pub repository: String,
    pub run_id: String,
    pub action: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrReservation {
    pub id: String,
    pub product: String,
    pub repository: String,
    pub run_id: String,
    pub action: String,
    pub status: String,
    pub pr_url: String,
    pub created_at: String,
    pub expires_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrReservationResponse {
    pub granted: bool,
    pub reason: String,
    pub limiting_layer: String,
    pub product_limit: u32,
    pub product_used: u32,
    pub suite_limit: u32,
    pub suite_used: u32,
    pub reservation: Option<PrReservation>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrReservationCommitRequest {
    pub pr_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrReservationReleaseRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrRunReleaseRequest {
    pub product: String,
    pub run_id: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    status: String,
    data: Option<T>,
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

pub fn hivecore_url() -> Option<String> {
    std::env::var("PATCHHIVE_HIVECORE_URL")
        .ok()
        .or_else(|| std::env::var("HIVECORE_URL").ok())
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

fn apply_auth(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    if let Some(token) = std::env::var("PATCHHIVE_HIVECORE_SERVICE_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return request.header("X-PatchHive-Service-Token", token);
    }
    if let Some(key) = std::env::var("PATCHHIVE_HIVECORE_API_KEY")
        .ok()
        .or_else(|| std::env::var("HIVECORE_API_KEY").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return request.header("X-API-Key", key);
    }
    request
}

async fn decode_envelope<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("Could not read HiveCore response")?;
    let envelope = serde_json::from_str::<ApiEnvelope<T>>(&body)
        .with_context(|| format!("Could not decode HiveCore response: {body}"))?;
    if !status.is_success() || envelope.status != "ok" {
        return Err(anyhow!(
            "HiveCore request failed: {}",
            envelope
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| status.to_string())
        ));
    }
    envelope
        .data
        .ok_or_else(|| anyhow!("HiveCore returned no response data"))
}

pub async fn check_repository_policy(
    client: &Client,
    request: &RepositoryPolicyDecisionRequest,
) -> Result<Option<RepositoryPolicyDecision>> {
    let Some(base_url) = hivecore_url() else {
        return Ok(None);
    };
    let response = apply_auth(
        client
            .post(format!("{base_url}/repository-policy/check"))
            .json(request),
    )
    .send()
    .await
    .context("HiveCore repository-policy request failed")?;
    decode_envelope(response).await.map(Some)
}

pub async fn reserve_pr_slot(
    client: &Client,
    request: &PrReservationRequest,
) -> Result<Option<PrReservationResponse>> {
    let Some(base_url) = hivecore_url() else {
        return Ok(None);
    };
    let response = apply_auth(
        client
            .post(format!("{base_url}/pr-budgets/reservations"))
            .json(request),
    )
    .send()
    .await
    .context("HiveCore PR reservation request failed")?;
    decode_envelope(response).await.map(Some)
}

pub async fn commit_pr_slot(
    client: &Client,
    reservation_id: &str,
    pr_url: &str,
) -> Result<Option<PrReservation>> {
    let Some(base_url) = hivecore_url() else {
        return Ok(None);
    };
    let response = apply_auth(
        client
            .post(format!(
                "{base_url}/pr-budgets/reservations/{reservation_id}/commit"
            ))
            .json(&PrReservationCommitRequest {
                pr_url: pr_url.into(),
            }),
    )
    .send()
    .await
    .context("HiveCore PR reservation commit failed")?;
    decode_envelope(response).await.map(Some)
}

pub async fn release_pr_slot(
    client: &Client,
    reservation_id: &str,
    reason: &str,
) -> Result<Option<PrReservation>> {
    let Some(base_url) = hivecore_url() else {
        return Ok(None);
    };
    let response = apply_auth(
        client
            .post(format!(
                "{base_url}/pr-budgets/reservations/{reservation_id}/release"
            ))
            .json(&PrReservationReleaseRequest {
                reason: reason.into(),
            }),
    )
    .send()
    .await
    .context("HiveCore PR reservation release failed")?;
    decode_envelope(response).await.map(Some)
}

pub async fn release_pr_slots_for_run(
    client: &Client,
    request: &PrRunReleaseRequest,
) -> Result<Option<Vec<PrReservation>>> {
    let Some(base_url) = hivecore_url() else {
        return Ok(None);
    };
    let response = apply_auth(
        client
            .post(format!("{base_url}/pr-budgets/releases"))
            .json(request),
    )
    .send()
    .await
    .context("HiveCore PR run release failed")?;
    decode_envelope(response).await.map(Some)
}

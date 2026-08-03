use axum::{extract::State, http::StatusCode, Json};
use patchhive_product_core::hivecore_kernel::{
    AdmissionEvidence, AiSpendEvidence, DeploymentTopology, Evidence, GithubRateEvidence,
    PauseRecord, PauseTarget, ReputationSummary, ResourcePolicy, SandboxEvidence, SmokeAuthority,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    db,
    models::{ok, ApiEnvelope},
    state::{product_catalog, AppState},
};

use super::types::api_error;

type BoxedApiError = Box<(StatusCode, Json<ApiEnvelope<Value>>)>;

#[derive(Debug, Clone, Serialize)]
pub struct GovernanceStatus {
    pub topology: DeploymentTopology,
    pub pauses: Vec<PauseRecord>,
    pub smoke_authority: SmokeAuthority,
    pub resource_policy: Evidence<ResourcePolicy>,
    pub github_rate: Evidence<GithubRateEvidence>,
    pub ai_spend: Evidence<AiSpendEvidence>,
    pub sandbox: Evidence<SandboxEvidence>,
    pub reputation: Evidence<ReputationSummary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveResourcePolicyRequest {
    pub github_min_remaining: u32,
    pub suite_ai_daily_limit_cents: u64,
    pub sandbox_slots: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PauseTargetRequest {
    pub target: PauseTarget,
    #[serde(default)]
    pub reason: String,
}

pub async fn governance_status(
    State(state): State<AppState>,
) -> Result<Json<ApiEnvelope<GovernanceStatus>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    db::reconcile_pause_drains().map_err(|error| {
        tracing::error!(%error, "could not reconcile pause drain state");
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "pause_drain_unavailable",
            "HiveCore could not reconcile durable pause drain state.",
        )
    })?;
    let pauses = db::pause_records().map_err(|error| {
        tracing::error!(%error, "could not read pause authority");
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "pause_authority_unavailable",
            "HiveCore could not read durable pause authority.",
        )
    })?;
    let now = crate::models::now_rfc3339();
    let resource_policy = match db::resource_policy() {
        Ok(value) => Evidence::Observed {
            value,
            observed_at: now.clone(),
        },
        Err(error) => Evidence::Failed {
            reason: format!("Could not read durable resource policy: {error}"),
        },
    };
    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let ai_spend = match (&resource_policy, db::ai_spend_for_day(&day)) {
        (Evidence::Observed { value: policy, .. }, Ok((spent_cents, reserved_cents))) => {
            Evidence::Observed {
                value: AiSpendEvidence {
                    daily_limit_cents: policy.suite_ai_daily_limit_cents,
                    spent_cents,
                    reserved_cents,
                    mandate_daily_limit_cents: None,
                    mandate_spent_cents: 0,
                    mandate_reserved_cents: 0,
                    day,
                },
                observed_at: now.clone(),
            }
        }
        (_, Err(error)) => Evidence::Failed {
            reason: format!("Could not read AI spend ledger: {error}"),
        },
        _ => Evidence::NotObserved {
            reason: "Resource policy is unavailable, so AI capacity cannot be evaluated.".into(),
        },
    };
    let sandbox = match (&resource_policy, db::sandbox_slots_in_use()) {
        (Evidence::Observed { value: policy, .. }, Ok(in_use)) => Evidence::Observed {
            value: SandboxEvidence {
                slots: policy.sandbox_slots,
                in_use,
            },
            observed_at: now.clone(),
        },
        (_, Err(error)) => Evidence::Failed {
            reason: format!("Could not read sandbox leases: {error}"),
        },
        _ => Evidence::NotObserved {
            reason: "Resource policy is unavailable, so sandbox capacity cannot be evaluated."
                .into(),
        },
    };
    let github_rate = observe_github_rate(&state, now).await;
    let reputation = match db::reputation_summary() {
        Ok(value) => Evidence::Observed {
            value,
            observed_at: crate::models::now_rfc3339(),
        },
        Err(error) => Evidence::Failed {
            reason: format!("Could not read outcome reputation evidence: {error}"),
        },
    };
    Ok(Json(ok(GovernanceStatus {
        topology: crate::runtime_topology(),
        pauses,
        smoke_authority: db::smoke_authority(),
        resource_policy,
        github_rate,
        ai_spend,
        sandbox,
        reputation,
    })))
}

pub async fn save_resource_policy(
    Json(request): Json<SaveResourcePolicyRequest>,
) -> Result<Json<ApiEnvelope<ResourcePolicy>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    let policy = ResourcePolicy {
        github_min_remaining: request.github_min_remaining,
        suite_ai_daily_limit_cents: request.suite_ai_daily_limit_cents,
        sandbox_slots: request.sandbox_slots,
        updated_at: String::new(),
    };
    policy.validate().map_err(|message| {
        api_error(StatusCode::BAD_REQUEST, "invalid_resource_policy", message)
    })?;
    db::save_resource_policy(policy)
        .map(|policy| Json(ok(policy)))
        .map_err(|error| {
            tracing::error!(%error, "could not save resource policy");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "resource_policy_save_failed",
                "HiveCore could not save resource policy.",
            )
        })
}

pub(crate) async fn discovery_admission_evidence(state: &AppState) -> AdmissionEvidence {
    let observed_at = crate::models::now_rfc3339();
    AdmissionEvidence {
        github_rate: observe_github_rate(state, observed_at).await,
        ai_spend: Evidence::NotApplicable {
            reason: "SignalHive discovery does not execute an AI model.".into(),
        },
        sandbox: Evidence::NotApplicable {
            reason: "SignalHive discovery does not claim a patch sandbox.".into(),
        },
        owner_politeness: Evidence::NotApplicable {
            reason: "Discovery does not publish or reserve a pull request.".into(),
        },
    }
}

pub(crate) async fn observe_github_rate(
    state: &AppState,
    observed_at: String,
) -> Evidence<GithubRateEvidence> {
    let token = std::env::var("PATCHHIVE_GITHUB_TOKEN_RO")
        .ok()
        .or_else(|| std::env::var("BOT_GITHUB_TOKEN").ok())
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let Some(token) = token else {
        return Evidence::NotObserved {
            reason: "PATCHHIVE_GITHUB_TOKEN_RO is not configured.".into(),
        };
    };
    let response = match state
        .client
        .get("https://api.github.com/rate_limit")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .bearer_auth(token)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return Evidence::Failed {
                reason: format!("GitHub rate-limit request failed: {error}"),
            }
        }
    };
    let status = response.status();
    let body = match response.json::<Value>().await {
        Ok(body) => body,
        Err(error) => {
            return Evidence::Failed {
                reason: format!("GitHub rate-limit response could not be decoded: {error}"),
            }
        }
    };
    if !status.is_success() {
        return Evidence::Failed {
            reason: format!("GitHub rate-limit request returned HTTP {status}."),
        };
    }
    let core = &body["resources"]["core"];
    let Some(limit) = core["limit"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
    else {
        return Evidence::Failed {
            reason: "GitHub rate-limit response omitted a valid core limit.".into(),
        };
    };
    let Some(remaining) = core["remaining"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
    else {
        return Evidence::Failed {
            reason: "GitHub rate-limit response omitted valid core remaining capacity.".into(),
        };
    };
    let Some(reset) = core["reset"].as_i64() else {
        return Evidence::Failed {
            reason: "GitHub rate-limit response omitted a valid reset time.".into(),
        };
    };
    let Some(reset_at) = chrono::DateTime::from_timestamp(reset, 0) else {
        return Evidence::Failed {
            reason: "GitHub rate-limit reset time was outside the supported range.".into(),
        };
    };
    Evidence::Observed {
        value: GithubRateEvidence {
            limit,
            remaining,
            reset_at: reset_at.to_rfc3339(),
        },
        observed_at,
    }
}

pub async fn pause(
    Json(request): Json<PauseTargetRequest>,
) -> Result<Json<ApiEnvelope<PauseRecord>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    let target = validate_target(request.target).map_err(|error| *error)?;
    let reason = request.reason.trim();
    if reason.is_empty() || reason.len() > 1_000 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_pause_reason",
            "A pause reason between 1 and 1000 characters is required.",
        ));
    }
    let observed_in_flight = db::in_flight_for_pause_target(&target).map_err(|error| {
        tracing::error!(%error, "could not count in-flight work before pausing");
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "pause_drain_unavailable",
            "HiveCore could not establish the drain state for this pause target.",
        )
    })?;
    db::pause_target(target, reason.to_owned(), observed_in_flight)
        .map(|record| Json(ok(record)))
        .map_err(|error| {
            tracing::error!(%error, "could not persist pause authority");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "pause_save_failed",
                "HiveCore could not persist the pause.",
            )
        })
}

pub async fn resume(
    Json(request): Json<PauseTargetRequest>,
) -> Result<Json<ApiEnvelope<PauseRecord>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    let target = validate_target(request.target).map_err(|error| *error)?;
    db::resume_target(target)
        .map(|record| Json(ok(record)))
        .map_err(|error| {
            tracing::error!(%error, "could not persist resumed authority");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "resume_save_failed",
                "HiveCore could not resume this scope.",
            )
        })
}

fn validate_target(target: PauseTarget) -> Result<PauseTarget, BoxedApiError> {
    match target {
        PauseTarget::Suite => Ok(PauseTarget::Suite),
        PauseTarget::Product { product_slug } => {
            let product_slug = product_slug.trim().to_ascii_lowercase();
            if !product_catalog()
                .iter()
                .any(|product| product.slug == product_slug)
            {
                return Err(Box::new(api_error(
                    StatusCode::BAD_REQUEST,
                    "unknown_pause_product",
                    format!("{product_slug} is not a registered PatchHive product."),
                )));
            }
            Ok(PauseTarget::Product { product_slug })
        }
        PauseTarget::Mandate { mandate_id } => {
            let mandate_id = mandate_id.trim().to_owned();
            match db::mandate(&mandate_id) {
                Ok(Some(_)) => Ok(PauseTarget::Mandate { mandate_id }),
                Ok(None) => Err(Box::new(api_error(
                    StatusCode::BAD_REQUEST,
                    "unknown_pause_mandate",
                    format!("Mandate {mandate_id} was not found."),
                ))),
                Err(error) => {
                    tracing::error!(%error, "could not validate pause mandate");
                    Err(Box::new(api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "pause_authority_unavailable",
                        "HiveCore could not validate the mandate pause target.",
                    )))
                }
            }
        }
        PauseTarget::Repository { repository } => {
            patchhive_product_core::scope_policy::normalize_repo_name(&repository)
                .map(|repository| PauseTarget::Repository { repository })
                .ok_or_else(|| {
                    Box::new(api_error(
                        StatusCode::BAD_REQUEST,
                        "invalid_pause_repository",
                        "Repository pause targets must use owner/repository format.",
                    ))
                })
        }
    }
}

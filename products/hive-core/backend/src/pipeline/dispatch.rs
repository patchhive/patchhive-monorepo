use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use patchhive_product_core::approvals::{ApprovalOrigin, ApprovalState, ApprovalSubject};
use patchhive_product_core::contract;
use reqwest::Method;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    db,
    models::{
        now_rfc3339, ok, ApprovalConsumptionOutcome, DispatchActionResponse, ProductActionEvent,
    },
    state::{product_catalog, AppState},
};

use super::{
    api_error, authorized_request, build_target_url, fetch_product_auth_status,
    fetch_product_capabilities, parse_response_body, resolve_api_url, DispatchActionInput,
    ProductAuthStatusBody, ProductStoredAuth,
};

pub(super) async fn recent_actions() -> Json<crate::models::ApiEnvelope<Vec<ProductActionEvent>>> {
    Json(ok(db::recent_action_events(30)))
}

pub(super) async fn dispatch_product_action(
    State(state): State<AppState>,
    Path((slug, action_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<
    Json<crate::models::ApiEnvelope<DispatchActionResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    dispatch_once(&state, &slug, &action_id, body)
        .await
        .map(|response| Json(ok(response)))
}

/// One dispatch, callable without an HTTP request.
///
/// Suite runs execute steps through this rather than re-implementing dispatch, so a
/// step is evaluated for exactly the same reasons a manual dispatch is — destructive,
/// approval-gated, PR-opening, missing or unscoped service token. Approval-gated steps
/// become durable pending approvals instead of bypassing the shared guard path.
pub(super) async fn dispatch_once(
    state: &AppState,
    slug: &str,
    action_id: &str,
    body: Value,
) -> Result<DispatchActionResponse, (StatusCode, Json<crate::models::ApiEnvelope<Value>>)> {
    dispatch_with_approval(
        state,
        slug,
        action_id,
        body,
        ApprovalOrigin::OperatorDispatch,
        None,
    )
    .await
}

pub(super) async fn dispatch_with_approval(
    state: &AppState,
    slug: &str,
    action_id: &str,
    body: Value,
    origin: ApprovalOrigin,
    approval_id: Option<&str>,
) -> Result<DispatchActionResponse, (StatusCode, Json<crate::models::ApiEnvelope<Value>>)> {
    let definition = product_catalog()
        .iter()
        .find(|product| product.slug == slug)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "unknown_product", "Unknown product."))?;

    if definition.slug == "hive-core" {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "unsupported_action",
            "HiveCore self-actions are handled by native HiveCore routes.",
        ));
    }

    let overrides = db::product_overrides();
    let override_item = overrides.get(&definition.slug);
    let enabled = override_item.map(|item| item.enabled).unwrap_or(true);
    if !enabled {
        return Err(api_error(
            StatusCode::CONFLICT,
            "product_disabled",
            "HiveCore will not dispatch actions to a disabled product.",
        ));
    }

    let api_url = resolve_api_url(override_item.map(|item| item.api_url.as_str()), definition);
    if api_url.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "product_unconfigured",
            "Configure this product API URL before dispatching actions.",
        ));
    }

    let auth = ProductStoredAuth::from_override(override_item);
    if !auth.machine_auth_configured() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "product_service_token_missing",
            "Save or provision this product's service token in HiveCore settings before dispatching protected actions.",
        ));
    }

    let capabilities = fetch_product_capabilities(&state.client, &api_url, &auth)
        .await
        .map_err(|message| {
            api_error(StatusCode::BAD_GATEWAY, "capabilities_unavailable", message)
        })?;
    let safety = crate::state::product_safety(&definition.slug).ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "product_safety_unavailable",
            "HiveCore refuses dispatch until the manifest-backed product safety registry is loaded.",
        )
    })?;
    let action = capabilities
        .actions
        .iter()
        .find(|action| action.id == action_id)
        .cloned()
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "unknown_action",
                "The product did not advertise that action.",
            )
        })?;

    if action.opens_pull_request() && !safety.opens_pull_requests {
        return Err(api_error(
            StatusCode::CONFLICT,
            "safety_contract_mismatch",
            "The live action claims it opens pull requests, but the product manifest does not.",
        ));
    }
    if action.effect.writes_external_state() && !safety.writes_external_state {
        return Err(api_error(
            StatusCode::CONFLICT,
            "safety_contract_mismatch",
            "The live action claims it writes external state, but the product manifest does not declare that boundary.",
        ));
    }
    if action.effect.mutates_repository() && !safety.mutates_repositories {
        return Err(api_error(
            StatusCode::CONFLICT,
            "safety_contract_mismatch",
            "The live action claims it mutates repositories, but the product manifest does not declare that boundary.",
        ));
    }
    if action.is_mutating() && safety.requires_operator_approval && !action.requires_approval() {
        return Err(api_error(
            StatusCode::CONFLICT,
            "safety_contract_mismatch",
            "The product manifest requires operator approval, but the live mutating action omits that gate.",
        ));
    }

    if action.destructive {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "destructive_action_blocked",
            "HiveCore does not dispatch destructive actions yet.",
        ));
    }

    if auth.legacy_api_key_configured() && !action.required_scopes.is_empty() {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "legacy_api_key_unscoped",
            "This action requires scoped machine credentials. Pair the product with a service token before dispatching it.",
        ));
    }

    if auth.service_token_configured() && !action.required_scopes.is_empty() {
        let auth_status = fetch_product_auth_status(&state.client, &api_url)
            .await
            .map_err(|message| {
                api_error(StatusCode::BAD_GATEWAY, "auth_status_unavailable", message)
            })?;

        if let Some((code, message)) =
            dispatch_service_token_issue(&definition.title, &action, &auth_status)
        {
            return Err(api_error(StatusCode::FORBIDDEN, code, message));
        }
    }

    let input = parse_dispatch_input(body);
    let path = fill_path_template(&action.path, &input.path_params)
        .map_err(|message| api_error(StatusCode::BAD_REQUEST, "invalid_action_path", message))?;
    let target_url = build_target_url(&api_url, &path, &input.query)
        .map_err(|message| api_error(StatusCode::BAD_REQUEST, "invalid_action_url", message))?;
    let method = Method::from_bytes(action.method.as_bytes()).map_err(|_| {
        api_error(
            StatusCode::BAD_REQUEST,
            "invalid_action_method",
            "The product advertised an invalid HTTP method.",
        )
    })?;

    let approval_required = action.requires_approval() || action.opens_pull_request();
    let approval_subject = approval_required.then(|| {
        let repository = approval_string_field(
            &input.payload,
            &["repo", "repository", "repository_full_name", "target_repo"],
        );
        let run_id = match &origin {
            ApprovalOrigin::SuiteRun { run_id } => Some(run_id.clone()),
            ApprovalOrigin::OperatorDispatch => {
                approval_string_field(&input.payload, &["run_id", "scan_id", "job_id"])
            }
        };
        ApprovalSubject::for_dispatch(
            definition.slug.clone(),
            &action,
            &input,
            repository,
            run_id,
            origin.clone(),
        )
    });

    let claimed_approval_id = if let Some(subject) = approval_subject {
        match approval_id {
            None => {
                let now = Utc::now();
                let approval = db::create_or_get_approval(
                    subject,
                    input.clone(),
                    (now + Duration::hours(i64::from(db::approval_ttl_hours()))).to_rfc3339(),
                    now.to_rfc3339(),
                )
                .map_err(|error| {
                    api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "approval_save_failed",
                        format!("HiveCore could not record the approval request: {error}"),
                    )
                })?;
                return Ok(DispatchActionResponse::ApprovalRequired {
                    approval: Box::new(approval),
                });
            }
            Some(id) => {
                let existing = db::approval(id)
                    .map_err(|error| {
                        api_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "approval_read_failed",
                            format!("HiveCore could not read the approval: {error}"),
                        )
                    })?
                    .ok_or_else(|| {
                        api_error(
                            StatusCode::NOT_FOUND,
                            "approval_not_found",
                            "Approval was not found.",
                        )
                    })?;
                if existing.subject != subject || existing.dispatch != input {
                    return Err(api_error(
                        StatusCode::CONFLICT,
                        "approval_subject_mismatch",
                        "This approval does not authorize the current product, action, safety contract, origin, or input.",
                    ));
                }
                if !matches!(existing.lifecycle, ApprovalState::Granted { .. }) {
                    return Err(api_error(
                        StatusCode::CONFLICT,
                        "approval_not_granted",
                        format!(
                            "Approval cannot be consumed from state '{}'.",
                            existing.lifecycle.label()
                        ),
                    ));
                }
                let claimed = db::claim_approval(id, &subject.fingerprint, &now_rfc3339())
                    .map_err(|error| {
                        api_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "approval_claim_failed",
                            format!("HiveCore could not claim the approval: {error}"),
                        )
                    })?
                    .ok_or_else(|| {
                        api_error(
                            StatusCode::NOT_FOUND,
                            "approval_not_found",
                            "Approval was not found.",
                        )
                    })?;
                if !matches!(claimed.lifecycle, ApprovalState::Consuming { .. }) {
                    return Err(api_error(
                        StatusCode::CONFLICT,
                        "approval_already_claimed",
                        "This single-use approval was already consumed or changed state.",
                    ));
                }
                Some(id.to_string())
            }
        }
    } else {
        if approval_id.is_some() {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "approval_not_applicable",
                "This action is automatic and cannot consume an operator approval.",
            ));
        }
        None
    };

    let event_id = format!("evt_{}", Uuid::now_v7());
    let mut event = ProductActionEvent {
        id: event_id,
        product_slug: definition.slug.clone(),
        action_id: action.id.clone(),
        action_label: action.label.clone(),
        method: action.method.clone(),
        path: path.clone(),
        target_url: target_url.to_string(),
        status: "dispatching".into(),
        remote_status: None,
        request_json: input.payload.clone(),
        response_json: Value::Null,
        error: String::new(),
        created_at: now_rfc3339(),
    };

    // The dispatch client, not the polling one: this is a product doing real work.
    let mut request = authorized_request(
        state.dispatch_client.request(method.clone(), target_url),
        &auth,
    );
    if method != Method::GET && method != Method::HEAD {
        request = request.json(&input.payload);
    }

    let approval_outcome = match request.send().await {
        Ok(response) => {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            event.remote_status = Some(status.as_u16());
            event.status = if status.is_success() {
                "dispatched".into()
            } else {
                "failed".into()
            };
            event.response_json = parse_response_body(&text);
            if !status.is_success() {
                event.error = format!("Product returned HTTP {status}");
            }
            if status.is_success() {
                ApprovalConsumptionOutcome::Accepted {
                    remote_status: status.as_u16(),
                }
            } else {
                ApprovalConsumptionOutcome::Rejected {
                    remote_status: Some(status.as_u16()),
                    reason: event.error.clone(),
                }
            }
        }
        Err(err) => {
            event.status = "failed".into();
            // Name a timeout as a timeout. reqwest's Display for a timed-out request
            // is "error sending request for url (...)", which reads as "the product is
            // unreachable" when what actually happened is that HiveCore stopped
            // waiting for a product that was still working.
            event.error = if err.is_timeout() {
                format!(
                    "HiveCore stopped waiting after {}s. The product may still be running this action; \
                     raise HIVE_CORE_DISPATCH_TIMEOUT_SECS if this action legitimately takes longer.",
                    crate::state::dispatch_timeout_secs()
                )
            } else if err.is_connect() {
                format!(
                    "Could not connect to the product at {}: {err}",
                    event.target_url
                )
            } else {
                err.to_string()
            };
            event.response_json = json!({ "error": event.error });
            ApprovalConsumptionOutcome::Uncertain {
                reason: event.error.clone(),
            }
        }
    };

    if let Err(err) = db::record_action_event(&event) {
        tracing::warn!("failed to record HiveCore product action event: {err}");
    }

    if let Some(approval_id) = claimed_approval_id {
        if let Err(error) =
            db::consume_approval(&approval_id, &event.id, approval_outcome, &now_rfc3339())
        {
            tracing::error!(
                approval_id,
                event_id = %event.id,
                %error,
                "dispatch completed but HiveCore could not finalize its consumed approval"
            );
        }
    }

    Ok(DispatchActionResponse::Dispatched {
        event: Box::new(event),
        started_run: action.starts_run,
    })
}

fn approval_string_field(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        payload
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

pub(super) fn dispatch_service_token_issue(
    product_title: &str,
    action: &contract::ProductAction,
    auth_status: &ProductAuthStatusBody,
) -> Option<(&'static str, String)> {
    if !auth_status.service_auth_enabled || action.required_scopes.is_empty() {
        return None;
    }

    if auth_status.service_auth_expired {
        return Some((
            "service_token_expired",
            format!(
                "The saved service token for {} is expired. Rotate it in HiveCore Settings before dispatching actions.",
                product_title
            ),
        ));
    }

    if !auth_status.service_auth_scoped || auth_status.service_auth_legacy {
        return Some((
            "service_token_rotation_required",
            format!(
                "The saved service token for {} is legacy and only grants runs:read access. Rotate it in HiveCore Settings before dispatching actions.",
                product_title
            ),
        ));
    }

    let missing_scopes = action
        .required_scopes
        .iter()
        .filter(|scope| {
            !auth_status
                .service_auth_scopes
                .iter()
                .any(|item| item == *scope)
        })
        .cloned()
        .collect::<Vec<_>>();

    if missing_scopes.is_empty() {
        None
    } else {
        Some((
            "service_token_scope_missing",
            format!(
                "The saved service token for {} is missing required scopes: {}.",
                product_title,
                missing_scopes.join(", ")
            ),
        ))
    }
}

pub(super) fn parse_dispatch_input(raw: Value) -> DispatchActionInput {
    contract::parse_dispatch_input(raw)
}

pub(super) fn fill_path_template(
    path: &str,
    path_params: &HashMap<String, String>,
) -> Result<String, String> {
    let mut resolved = path.to_string();
    for (key, value) in path_params {
        if value.is_empty()
            || matches!(value.as_str(), "." | "..")
            || value
                .chars()
                .any(|character| matches!(character, '/' | '\\' | '?' | '#' | '{' | '}' | '%'))
        {
            return Err(format!(
                "Action path parameter '{key}' contains unsupported path characters."
            ));
        }
        resolved = resolved.replace(&format!("{{{key}}}"), value);
    }

    if resolved.contains('{') || resolved.contains('}') {
        return Err(format!(
            "Action path '{path}' requires path_params for every template value."
        ));
    }
    Ok(resolved)
}

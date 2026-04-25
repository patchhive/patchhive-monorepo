use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use patchhive_product_core::contract;
use reqwest::Method;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    db,
    models::{now_rfc3339, ok, DispatchActionResponse, ProductActionEvent},
    state::{product_catalog, AppState},
};

use super::{
    api_error, authorized_request, build_target_url, fetch_product_auth_status,
    fetch_product_capabilities, parse_response_body, pick_url, DispatchActionInput,
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
    let override_item = overrides.get(definition.slug);
    let enabled = override_item.map(|item| item.enabled).unwrap_or(true);
    if !enabled {
        return Err(api_error(
            StatusCode::CONFLICT,
            "product_disabled",
            "HiveCore will not dispatch actions to a disabled product.",
        ));
    }

    let api_url = pick_url(
        override_item.map(|item| item.api_url.as_str()),
        definition.default_api_url,
    );
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

    if action.destructive {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "destructive_action_blocked",
            "HiveCore does not dispatch destructive actions yet.",
        ));
    }

    if auth.service_token_configured() && !action.required_scopes.is_empty() {
        let auth_status = fetch_product_auth_status(&state.client, &api_url)
            .await
            .map_err(|message| {
                api_error(StatusCode::BAD_GATEWAY, "auth_status_unavailable", message)
            })?;

        if let Some((code, message)) =
            dispatch_service_token_issue(definition.title, &action, &auth_status)
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

    let event_id = format!("evt_{}", Uuid::now_v7());
    let mut event = ProductActionEvent {
        id: event_id,
        product_slug: definition.slug.into(),
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

    let mut request = authorized_request(state.client.request(method.clone(), target_url), &auth);
    if method != Method::GET && method != Method::HEAD {
        request = request.json(&input.payload);
    }

    match request.send().await {
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
        }
        Err(err) => {
            event.status = "failed".into();
            event.error = err.to_string();
            event.response_json = json!({ "error": err.to_string() });
        }
    }

    if let Err(err) = db::record_action_event(&event) {
        tracing::warn!("failed to record HiveCore product action event: {err}");
    }

    let response = DispatchActionResponse {
        event,
        started_run: action.starts_run,
    };
    Ok(Json(ok(response)))
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
    let Some(object) = raw.as_object() else {
        return DispatchActionInput {
            payload: raw,
            ..DispatchActionInput::default()
        };
    };

    let has_wrapper_keys = object.contains_key("payload")
        || object.contains_key("path_params")
        || object.contains_key("query");
    if !has_wrapper_keys {
        return DispatchActionInput {
            payload: raw,
            ..DispatchActionInput::default()
        };
    }

    DispatchActionInput {
        payload: object.get("payload").cloned().unwrap_or(Value::Null),
        path_params: string_map_from_value(object.get("path_params")),
        query: string_map_from_value(object.get("query")),
    }
}

pub(super) fn string_map_from_value(value: Option<&Value>) -> HashMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        value
                            .as_str()
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| value.to_string()),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn fill_path_template(
    path: &str,
    path_params: &HashMap<String, String>,
) -> Result<String, String> {
    let mut resolved = path.to_string();
    for (key, value) in path_params {
        resolved = resolved.replace(&format!("{{{key}}}"), value);
    }

    if resolved.contains('{') || resolved.contains('}') {
        return Err(format!(
            "Action path '{path}' requires path_params for every template value."
        ));
    }
    Ok(resolved)
}

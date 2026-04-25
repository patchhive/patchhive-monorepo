use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::Value;

use crate::{
    models::{
        now_rfc3339, ok, ProductOverride, ProvisionServiceTokenRequest,
        ProvisionServiceTokenResponse,
    },
    state::{product_catalog, AppState},
};

use super::settings::build_settings_product_item;
use super::{
    api_error, fetch_product_auth_status, parse_response_body, persist_product_override, pick_url,
    remote_error_message,
};
use crate::db;

pub(super) async fn provision_service_token(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(body): Json<ProvisionServiceTokenRequest>,
) -> Result<
    Json<crate::models::ApiEnvelope<ProvisionServiceTokenResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    let definition = product_catalog()
        .iter()
        .find(|product| product.slug == slug)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "unknown_product", "Unknown product."))?;

    if definition.slug == "hive-core" {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "unsupported_product",
            "HiveCore does not provision a product service token for itself.",
        ));
    }

    let overrides = db::product_overrides();
    let override_item = overrides.get(definition.slug);
    let api_url_override = body.api_url.unwrap_or_default().trim().to_string();
    let effective_api_url = if api_url_override.is_empty() {
        pick_url(
            override_item.map(|item| item.api_url.as_str()),
            definition.default_api_url,
        )
    } else {
        api_url_override.clone()
    };

    if effective_api_url.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "product_unconfigured",
            "Configure this product API URL before provisioning a service token.",
        ));
    }

    let auth_status = fetch_product_auth_status(&state.client, &effective_api_url)
        .await
        .map_err(|message| {
            api_error(StatusCode::BAD_GATEWAY, "auth_status_unavailable", message)
        })?;

    if !auth_status.service_auth_supported {
        return Err(api_error(
            StatusCode::CONFLICT,
            "service_auth_unsupported",
            "This product does not advertise service-token auth yet.",
        ));
    }

    let operator_api_key = body.operator_api_key.unwrap_or_default().trim().to_string();
    if auth_status.auth_enabled && operator_api_key.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "operator_api_key_required",
            "This product already requires operator login. Paste a one-time operator API key so HiveCore can mint or rotate a dedicated service token.",
        ));
    }

    let normalized = effective_api_url.trim_end_matches('/');
    let token_path = if auth_status.service_auth_enabled {
        "/auth/rotate-service-token"
    } else {
        "/auth/generate-service-token"
    };
    let token_url = format!("{normalized}{token_path}");
    let mut request = state.client.post(token_url).timeout(Duration::from_secs(5));
    if auth_status.auth_enabled {
        request = request.header("X-API-Key", operator_api_key);
    }

    let response = request.send().await.map_err(|_| {
        api_error(
            StatusCode::BAD_GATEWAY,
            "service_token_provision_failed",
            "HiveCore could not reach the product service-token endpoint.",
        )
    })?;

    let remote_status = response.status();
    let remote_body = response.text().await.unwrap_or_default();
    let remote_json = parse_response_body(&remote_body);

    if !remote_status.is_success() {
        let message = remote_error_message(&remote_json)
            .unwrap_or_else(|| format!("{token_path} returned HTTP {remote_status}"));
        return Err(api_error(
            if remote_status.is_client_error() {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::BAD_GATEWAY
            },
            "service_token_provision_rejected",
            message,
        ));
    }

    let token = remote_json
        .get("service_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            api_error(
                StatusCode::BAD_GATEWAY,
                "service_token_missing",
                "The product did not return a service token.",
            )
        })?;

    let updated_override = ProductOverride {
        slug: definition.slug.into(),
        frontend_url: override_item
            .map(|item| item.frontend_url.clone())
            .unwrap_or_default(),
        api_url: if api_url_override.is_empty() {
            override_item
                .map(|item| item.api_url.clone())
                .unwrap_or_default()
        } else {
            api_url_override
        },
        service_token: token.to_string(),
        legacy_api_key: String::new(),
        enabled: override_item.map(|item| item.enabled).unwrap_or(true),
        notes: override_item
            .map(|item| item.notes.clone())
            .unwrap_or_default(),
        updated_at: now_rfc3339(),
    };

    persist_product_override(updated_override.clone()).map_err(|message| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            format!("HiveCore could not save the provisioned service token: {message}"),
        )
    })?;

    let product = build_settings_product_item(definition, Some(&updated_override));
    let message = if auth_status.service_auth_enabled && auth_status.auth_enabled {
        format!(
            "HiveCore rotated the existing service token for {} using a one-time operator API key and stored only the replacement service token.",
            definition.title
        )
    } else if auth_status.service_auth_enabled {
        format!(
            "HiveCore rotated the existing service token for {} through the product bootstrap flow and stored only the replacement service token.",
            definition.title
        )
    } else if auth_status.auth_enabled {
        format!(
            "HiveCore provisioned a dedicated service token for {} using a one-time operator API key and stored only the service token.",
            definition.title
        )
    } else {
        format!(
            "HiveCore provisioned a dedicated service token for {} through the product bootstrap flow and stored it server-side.",
            definition.title
        )
    };

    Ok(Json(ok(ProvisionServiceTokenResponse { product, message })))
}

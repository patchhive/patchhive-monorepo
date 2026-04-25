use std::collections::HashMap;

use axum::{http::StatusCode, Json};
use serde_json::Value;

use crate::{
    db,
    models::{
        error, now_rfc3339, ok, ProductOverride, ProductOverrideInput, ProductSettingsItem,
        SaveSettingsRequest, SettingsResponse, SuiteSettings, SuiteSettingsInput, PRODUCT_TAGLINE,
        PRODUCT_TITLE,
    },
    state::{product_catalog, ProductDefinition},
};

use super::{
    resolved_auth_mode, resolved_legacy_api_key_configured, resolved_machine_auth_configured,
    resolved_service_token_configured, ProductStoredAuth,
};

pub(super) async fn settings() -> Json<crate::models::ApiEnvelope<SettingsResponse>> {
    Json(ok(build_settings_response()))
}

pub(super) async fn save_settings(
    Json(body): Json<SaveSettingsRequest>,
) -> Result<
    Json<crate::models::ApiEnvelope<SettingsResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    let settings = sanitize_suite_settings(body.suite_settings);
    let existing_overrides = db::product_overrides();
    let products = sanitize_product_overrides(body.products, &existing_overrides);

    let known: Vec<&str> = product_catalog()
        .iter()
        .map(|product| product.slug)
        .collect();
    for item in &products {
        if !known.contains(&item.slug.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(error(
                    "invalid_request",
                    format!("Unknown product slug '{}'.", item.slug),
                    false,
                )),
            ));
        }
    }

    db::save_suite_settings(&settings).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(error(
                "internal_error",
                "HiveCore could not save suite settings.",
                true,
            )),
        )
    })?;
    db::replace_product_overrides(&products).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(error(
                "internal_error",
                format!("HiveCore could not save product overrides: {err}"),
                true,
            )),
        )
    })?;

    Ok(Json(ok(build_settings_response())))
}

pub(super) fn build_settings_response() -> SettingsResponse {
    let suite_settings = db::suite_settings();
    let overrides = db::product_overrides();

    let products = product_catalog()
        .iter()
        .map(|definition| build_settings_product_item(definition, overrides.get(definition.slug)))
        .collect();

    SettingsResponse {
        product: PRODUCT_TITLE,
        tagline: PRODUCT_TAGLINE,
        suite_settings,
        products,
    }
}

pub(super) fn build_settings_product_item(
    definition: &ProductDefinition,
    override_item: Option<&ProductOverride>,
) -> ProductSettingsItem {
    let auth = ProductStoredAuth::from_override(override_item);
    ProductSettingsItem {
        slug: definition.slug.into(),
        title: definition.title.into(),
        icon: definition.icon.into(),
        lane: definition.lane.into(),
        role: definition.role.into(),
        repo: definition.repo.into(),
        default_frontend_url: definition.default_frontend_url.into(),
        default_api_url: definition.default_api_url.into(),
        override_frontend_url: override_item
            .map(|item| item.frontend_url.clone())
            .unwrap_or_default(),
        override_api_url: override_item
            .map(|item| item.api_url.clone())
            .unwrap_or_default(),
        auth_mode: resolved_auth_mode(definition, &auth),
        machine_auth_configured: resolved_machine_auth_configured(definition, &auth),
        service_token_configured: resolved_service_token_configured(definition, &auth),
        legacy_api_key_configured: resolved_legacy_api_key_configured(definition, &auth),
        enabled: override_item.map(|item| item.enabled).unwrap_or(true),
        notes: override_item
            .map(|item| item.notes.clone())
            .unwrap_or_default(),
        updated_at: override_item
            .map(|item| item.updated_at.clone())
            .unwrap_or_default(),
    }
}

pub(super) fn sanitize_suite_settings(input: SuiteSettingsInput) -> SuiteSettings {
    SuiteSettings {
        operator_label: input.operator_label.trim().to_string(),
        mission: input.mission.trim().to_string(),
        default_topics: input.default_topics.trim().to_string(),
        default_languages: input.default_languages.trim().to_string(),
        repo_allowlist: input.repo_allowlist.trim().to_string(),
        repo_denylist: input.repo_denylist.trim().to_string(),
        opt_out_notes: input.opt_out_notes.trim().to_string(),
        preferred_launch_product: input.preferred_launch_product.trim().to_string(),
        notes: input.notes.trim().to_string(),
        updated_at: now_rfc3339(),
    }
}

pub(super) fn sanitize_product_overrides(
    products: Vec<ProductOverrideInput>,
    existing: &HashMap<String, ProductOverride>,
) -> Vec<ProductOverride> {
    let mut deduped = HashMap::new();
    for product in products {
        let slug = product.slug.trim().to_string();
        let existing_item = existing.get(&slug);
        let supplied_service_token = product.service_token.unwrap_or_default().trim().to_string();
        let supplied_legacy_api_key = product
            .legacy_api_key
            .unwrap_or_default()
            .trim()
            .to_string();

        let service_token = if slug == "hive-core" {
            String::new()
        } else if supplied_service_token.is_empty() {
            existing_item
                .map(|item| item.service_token.clone())
                .unwrap_or_default()
        } else {
            supplied_service_token.clone()
        };

        let legacy_api_key = if slug == "hive-core" {
            String::new()
        } else if !supplied_service_token.is_empty() && supplied_legacy_api_key.is_empty() {
            String::new()
        } else if supplied_legacy_api_key.is_empty() {
            existing_item
                .map(|item| item.legacy_api_key.clone())
                .unwrap_or_default()
        } else {
            supplied_legacy_api_key
        };

        deduped.insert(
            slug.clone(),
            ProductOverride {
                slug,
                frontend_url: product.frontend_url.trim().to_string(),
                api_url: product.api_url.trim().to_string(),
                service_token,
                legacy_api_key,
                enabled: product.enabled,
                notes: product.notes.trim().to_string(),
                updated_at: now_rfc3339(),
            },
        );
    }
    let mut rows = deduped.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| left.slug.cmp(&right.slug));
    rows
}

use std::{collections::HashMap, time::Duration};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use patchhive_product_core::startup::count_errors;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    auth::{auth_enabled, generate_and_save_key, verify_token},
    db,
    models::{
        error, now_rfc3339, ok, OverviewResponse, OverviewSummary, ProductHealthSnapshot,
        ProductOverride, ProductOverrideInput, ProductRuntimeItem, ProductSettingsItem,
        SaveSettingsRequest, SettingsResponse, SuiteSettings, SuiteSettingsInput, PRODUCT_TAGLINE,
        PRODUCT_TITLE, PRODUCT_VERSION,
    },
    startup,
    state::{product_catalog, AppState, ProductDefinition},
};

#[derive(Deserialize)]
pub struct LoginBody {
    api_key: String,
}

#[derive(Deserialize)]
struct ProductHealthBody {
    status: Option<String>,
    version: Option<String>,
    config_errors: Option<u32>,
    db_ok: Option<bool>,
}

#[derive(Deserialize)]
struct StartupChecksBody {
    checks: Vec<patchhive_product_core::startup::StartupCheck>,
}

pub async fn auth_status() -> Json<Value> {
    Json(crate::auth::auth_status_payload())
}

pub async fn login(Json(body): Json<LoginBody>) -> Result<Json<Value>, StatusCode> {
    if !auth_enabled() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if !verify_token(&body.api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(
        json!({"ok": true, "auth_enabled": true, "auth_configured": true}),
    ))
}

pub async fn gen_key(headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
    if auth_enabled() {
        return Err(StatusCode::FORBIDDEN);
    }
    if !crate::auth::bootstrap_request_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let key = generate_and_save_key().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        json!({"api_key": key, "message": "Store this — it won't be shown again"}),
    ))
}

pub async fn health() -> Json<Value> {
    let checks = startup::startup_checks();
    let errors = count_errors(&checks);
    let db_ok = db::health_check();

    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": PRODUCT_VERSION,
        "product": format!("{PRODUCT_TITLE} by PatchHive"),
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "product_override_count": db::product_override_count(),
        "mode": "control-plane",
    }))
}

pub async fn startup_checks_route() -> Json<Value> {
    Json(json!({ "checks": startup::startup_checks() }))
}

pub async fn overview(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<OverviewResponse>> {
    let suite_settings = db::suite_settings();
    let products = build_runtime_products(&state).await;
    let summary = summarize_products(&products);
    Json(ok(OverviewResponse {
        product: PRODUCT_TITLE,
        tagline: PRODUCT_TAGLINE,
        suite_settings,
        summary,
        products,
    }))
}

pub async fn products(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<Vec<ProductRuntimeItem>>> {
    Json(ok(build_runtime_products(&state).await))
}

pub async fn settings() -> Json<crate::models::ApiEnvelope<SettingsResponse>> {
    Json(ok(build_settings_response()))
}

pub async fn save_settings(
    Json(body): Json<SaveSettingsRequest>,
) -> Result<
    Json<crate::models::ApiEnvelope<SettingsResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    let settings = sanitize_suite_settings(body.suite_settings);
    let products = sanitize_product_overrides(body.products);

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
    db::replace_product_overrides(&products).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(error(
                "internal_error",
                "HiveCore could not save product overrides.",
                true,
            )),
        )
    })?;

    Ok(Json(ok(build_settings_response())))
}

async fn build_runtime_products(state: &AppState) -> Vec<ProductRuntimeItem> {
    let overrides = db::product_overrides();
    let mut products = Vec::new();

    for definition in product_catalog() {
        let runtime =
            build_product_runtime(state, definition, overrides.get(definition.slug)).await;
        products.push(runtime);
    }

    products
}

fn build_settings_response() -> SettingsResponse {
    let suite_settings = db::suite_settings();
    let overrides = db::product_overrides();

    let products = product_catalog()
        .iter()
        .map(|definition| {
            let override_item = overrides.get(definition.slug);
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
                enabled: override_item.map(|item| item.enabled).unwrap_or(true),
                notes: override_item
                    .map(|item| item.notes.clone())
                    .unwrap_or_default(),
                updated_at: override_item
                    .map(|item| item.updated_at.clone())
                    .unwrap_or_default(),
            }
        })
        .collect();

    SettingsResponse {
        product: PRODUCT_TITLE,
        tagline: PRODUCT_TAGLINE,
        suite_settings,
        products,
    }
}

async fn build_product_runtime(
    state: &AppState,
    definition: &ProductDefinition,
    override_item: Option<&ProductOverride>,
) -> ProductRuntimeItem {
    let enabled = override_item.map(|item| item.enabled).unwrap_or(true);
    let frontend_url = pick_url(
        override_item.map(|item| item.frontend_url.as_str()),
        definition.default_frontend_url,
    );
    let api_url = pick_url(
        override_item.map(|item| item.api_url.as_str()),
        definition.default_api_url,
    );
    let notes = override_item
        .map(|item| item.notes.clone())
        .unwrap_or_default();

    let health = if definition.slug == "hive-core" {
        local_hive_core_health()
    } else if !enabled {
        ProductHealthSnapshot {
            status: "disabled".into(),
            checked_at: now_rfc3339(),
            ..ProductHealthSnapshot::default()
        }
    } else if api_url.is_empty() {
        ProductHealthSnapshot {
            status: "unconfigured".into(),
            checked_at: now_rfc3339(),
            ..ProductHealthSnapshot::default()
        }
    } else {
        fetch_product_health(&state.client, &api_url).await
    };

    ProductRuntimeItem {
        slug: definition.slug.into(),
        title: definition.title.into(),
        icon: definition.icon.into(),
        lane: definition.lane.into(),
        role: definition.role.into(),
        repo: definition.repo.into(),
        enabled,
        frontend_url,
        api_url,
        notes,
        status: health.status.clone(),
        health,
    }
}

async fn fetch_product_health(client: &reqwest::Client, api_url: &str) -> ProductHealthSnapshot {
    let normalized = api_url.trim_end_matches('/');
    let checked_at = now_rfc3339();
    let health_url = format!("{normalized}/health");

    let health_response = client.get(&health_url).send().await;
    let Ok(health_response) = health_response else {
        return ProductHealthSnapshot {
            status: "offline".into(),
            checked_at,
            error: "Could not reach /health.".into(),
            ..ProductHealthSnapshot::default()
        };
    };

    if !health_response.status().is_success() {
        return ProductHealthSnapshot {
            status: "offline".into(),
            checked_at,
            error: format!("/health returned HTTP {}", health_response.status()),
            ..ProductHealthSnapshot::default()
        };
    }

    let health_body =
        health_response
            .json::<ProductHealthBody>()
            .await
            .unwrap_or(ProductHealthBody {
                status: Some("unknown".into()),
                version: None,
                config_errors: Some(0),
                db_ok: None,
            });

    let checks_url = format!("{normalized}/startup/checks");
    let checks_response = client
        .get(&checks_url)
        .timeout(Duration::from_secs(3))
        .send()
        .await;

    let (startup_errors, startup_warns, startup_infos, extra_error) = match checks_response {
        Ok(response) if response.status().is_success() => {
            let body = response
                .json::<StartupChecksBody>()
                .await
                .unwrap_or(StartupChecksBody { checks: Vec::new() });
            let (errors, warns, infos) = startup::summarize_check_levels(&body.checks);
            (errors, warns, infos, String::new())
        }
        Ok(response) => (
            0,
            0,
            0,
            format!("/startup/checks returned HTTP {}", response.status()),
        ),
        Err(_) => (0, 0, 0, "Could not reach /startup/checks.".into()),
    };

    let config_errors = health_body.config_errors.unwrap_or(0);
    let base_status = health_body.status.unwrap_or_else(|| "unknown".into());
    let status = if startup_errors > 0 || config_errors > 0 || base_status != "ok" {
        "degraded"
    } else {
        "online"
    };

    ProductHealthSnapshot {
        status: status.into(),
        reachable: true,
        version: health_body.version.unwrap_or_default(),
        config_errors,
        startup_errors,
        startup_warns,
        startup_infos,
        db_ok: health_body.db_ok,
        checked_at,
        error: extra_error,
    }
}

fn local_hive_core_health() -> ProductHealthSnapshot {
    let checks = startup::startup_checks();
    let (startup_errors, startup_warns, startup_infos) = startup::summarize_check_levels(&checks);
    let db_ok = db::health_check();
    let status = if startup_errors > 0 || !db_ok {
        "degraded"
    } else {
        "online"
    };
    ProductHealthSnapshot {
        status: status.into(),
        reachable: true,
        version: PRODUCT_VERSION.into(),
        config_errors: startup_errors,
        startup_errors,
        startup_warns,
        startup_infos,
        db_ok: Some(db_ok),
        checked_at: now_rfc3339(),
        error: String::new(),
    }
}

fn pick_url(override_url: Option<&str>, default_url: &str) -> String {
    let override_url = override_url.unwrap_or("").trim();
    if override_url.is_empty() {
        default_url.to_string()
    } else {
        override_url.to_string()
    }
}

fn sanitize_suite_settings(input: SuiteSettingsInput) -> SuiteSettings {
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

fn sanitize_product_overrides(products: Vec<ProductOverrideInput>) -> Vec<ProductOverride> {
    let mut deduped = HashMap::new();
    for product in products {
        let slug = product.slug.trim().to_string();
        deduped.insert(
            slug.clone(),
            ProductOverride {
                slug,
                frontend_url: product.frontend_url.trim().to_string(),
                api_url: product.api_url.trim().to_string(),
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

fn summarize_products(products: &[ProductRuntimeItem]) -> OverviewSummary {
    let mut summary = OverviewSummary {
        total_products: products.len() as u32,
        ..OverviewSummary::default()
    };

    for product in products {
        match product.status.as_str() {
            "online" => {
                summary.enabled_products += 1;
                summary.online_products += 1;
            }
            "degraded" => {
                summary.enabled_products += 1;
                summary.degraded_products += 1;
            }
            "offline" => {
                summary.enabled_products += 1;
                summary.offline_products += 1;
            }
            "unconfigured" => {
                summary.enabled_products += 1;
                summary.unconfigured_products += 1;
            }
            "disabled" => {
                summary.disabled_products += 1;
            }
            _ => {
                summary.enabled_products += 1;
                summary.offline_products += 1;
            }
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::{pick_url, summarize_products};
    use crate::models::{ProductHealthSnapshot, ProductRuntimeItem};

    #[test]
    fn summarize_products_counts_each_runtime_status() {
        let products = vec![
            ProductRuntimeItem {
                slug: "signal-hive".into(),
                title: "SignalHive".into(),
                icon: "📡".into(),
                lane: "Visibility".into(),
                role: "role".into(),
                repo: "patchhive/signalhive".into(),
                enabled: true,
                frontend_url: String::new(),
                api_url: String::new(),
                notes: String::new(),
                status: "online".into(),
                health: ProductHealthSnapshot::default(),
            },
            ProductRuntimeItem {
                slug: "repo-reaper".into(),
                title: "RepoReaper".into(),
                icon: "⚔".into(),
                lane: "Action".into(),
                role: "role".into(),
                repo: "patchhive/reporeaper".into(),
                enabled: true,
                frontend_url: String::new(),
                api_url: String::new(),
                notes: String::new(),
                status: "degraded".into(),
                health: ProductHealthSnapshot::default(),
            },
            ProductRuntimeItem {
                slug: "review-bee".into(),
                title: "ReviewBee".into(),
                icon: "🐝".into(),
                lane: "Review".into(),
                role: "role".into(),
                repo: "patchhive/reviewbee".into(),
                enabled: true,
                frontend_url: String::new(),
                api_url: String::new(),
                notes: String::new(),
                status: "unconfigured".into(),
                health: ProductHealthSnapshot::default(),
            },
            ProductRuntimeItem {
                slug: "merge-keeper".into(),
                title: "MergeKeeper".into(),
                icon: "🔗".into(),
                lane: "Merge".into(),
                role: "role".into(),
                repo: "patchhive/mergekeeper".into(),
                enabled: false,
                frontend_url: String::new(),
                api_url: String::new(),
                notes: String::new(),
                status: "disabled".into(),
                health: ProductHealthSnapshot::default(),
            },
        ];

        let summary = summarize_products(&products);
        assert_eq!(summary.total_products, 4);
        assert_eq!(summary.enabled_products, 3);
        assert_eq!(summary.online_products, 1);
        assert_eq!(summary.degraded_products, 1);
        assert_eq!(summary.unconfigured_products, 1);
        assert_eq!(summary.disabled_products, 1);
    }

    #[test]
    fn pick_url_prefers_non_empty_override() {
        assert_eq!(
            pick_url(Some(" https://example.com "), "http://localhost:8010"),
            "https://example.com"
        );
        assert_eq!(
            pick_url(Some(""), "http://localhost:8010"),
            "http://localhost:8010"
        );
        assert_eq!(
            pick_url(None, "http://localhost:8010"),
            "http://localhost:8010"
        );
    }
}

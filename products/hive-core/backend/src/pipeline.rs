use std::{collections::HashMap, time::Duration};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use patchhive_product_core::contract;
use patchhive_product_core::startup::count_errors;
use reqwest::{Method, Url};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    auth::{auth_enabled, generate_and_save_key, verify_token},
    db,
    models::{
        error, now_rfc3339, ok, DispatchActionResponse, OverviewResponse, OverviewSummary,
        ProductActionEvent, ProductHealthSnapshot, ProductOverride, ProductOverrideInput,
        ProductRuntimeItem, ProductSettingsItem, SaveSettingsRequest, SettingsResponse,
        SuiteSettings, SuiteSettingsInput, PRODUCT_TAGLINE, PRODUCT_TITLE, PRODUCT_VERSION,
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

struct ProductProbeSnapshot {
    health: ProductHealthSnapshot,
    actions: Vec<contract::ProductAction>,
}

#[derive(Debug, Default)]
struct DispatchActionInput {
    payload: Value,
    path_params: HashMap<String, String>,
    query: HashMap<String, String>,
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

pub async fn capabilities() -> Json<contract::ProductCapabilities> {
    let mut caps = contract::capabilities(
        "hive-core",
        "HiveCore",
        vec![contract::action(
            "save_settings",
            "Save suite settings",
            "PUT",
            "/settings",
            "Persist suite-wide defaults and per-product launch/API overrides.",
            false,
        )],
        vec![
            contract::link("overview", "Overview", "/overview"),
            contract::link("products", "Products", "/products"),
            contract::link("settings", "Settings", "/settings"),
        ],
    );
    caps.hivecore.can_apply_settings = true;
    caps.routes.settings_apply = Some("/settings".into());
    Json(caps)
}

pub async fn runs() -> Json<contract::ProductRunsResponse> {
    Json(contract::runs_from_values("hive-core", Vec::new()))
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

pub async fn recent_actions() -> Json<crate::models::ApiEnvelope<Vec<ProductActionEvent>>> {
    Json(ok(db::recent_action_events(30)))
}

pub async fn save_settings(
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

pub async fn dispatch_product_action(
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

    let api_key = override_item
        .map(|item| item.api_key.trim().to_string())
        .unwrap_or_default();
    if api_key.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "product_api_key_missing",
            "Save this product's API key in HiveCore settings before dispatching protected actions.",
        ));
    }

    let capabilities = fetch_product_capabilities(&state.client, &api_url)
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

    let mut request = state
        .client
        .request(method.clone(), target_url)
        .header("X-API-Key", api_key);
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
                api_key_configured: override_item
                    .map(|item| !item.api_key.trim().is_empty())
                    .unwrap_or(false),
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

    let probe = if definition.slug == "hive-core" {
        local_hive_core_probe()
    } else if !enabled {
        ProductProbeSnapshot {
            health: ProductHealthSnapshot {
                status: "disabled".into(),
                checked_at: now_rfc3339(),
                ..ProductHealthSnapshot::default()
            },
            actions: Vec::new(),
        }
    } else if api_url.is_empty() {
        ProductProbeSnapshot {
            health: ProductHealthSnapshot {
                status: "unconfigured".into(),
                checked_at: now_rfc3339(),
                ..ProductHealthSnapshot::default()
            },
            actions: Vec::new(),
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
        api_key_configured: override_item
            .map(|item| !item.api_key.trim().is_empty())
            .unwrap_or(false),
        notes,
        status: probe.health.status.clone(),
        health: probe.health,
        actions: probe.actions,
    }
}

async fn fetch_product_health(client: &reqwest::Client, api_url: &str) -> ProductProbeSnapshot {
    let normalized = api_url.trim_end_matches('/');
    let checked_at = now_rfc3339();
    let health_url = format!("{normalized}/health");

    let health_response = client.get(&health_url).send().await;
    let Ok(health_response) = health_response else {
        return ProductProbeSnapshot {
            health: ProductHealthSnapshot {
                status: "offline".into(),
                checked_at,
                error: "Could not reach /health.".into(),
                ..ProductHealthSnapshot::default()
            },
            actions: Vec::new(),
        };
    };

    if !health_response.status().is_success() {
        return ProductProbeSnapshot {
            health: ProductHealthSnapshot {
                status: "offline".into(),
                checked_at,
                error: format!("/health returned HTTP {}", health_response.status()),
                ..ProductHealthSnapshot::default()
            },
            actions: Vec::new(),
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

    let (capabilities_ok, actions, capabilities_error) =
        match fetch_product_capabilities(client, api_url).await {
            Ok(body) => (true, body.actions, String::new()),
            Err(message) => (false, Vec::new(), message),
        };
    let action_count = actions.len() as u32;

    let config_errors = health_body.config_errors.unwrap_or(0);
    let base_status = health_body.status.unwrap_or_else(|| "unknown".into());
    let status =
        if startup_errors > 0 || config_errors > 0 || base_status != "ok" || !capabilities_ok {
            "degraded"
        } else {
            "online"
        };
    let error = [extra_error, capabilities_error]
        .into_iter()
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    ProductProbeSnapshot {
        health: ProductHealthSnapshot {
            status: status.into(),
            reachable: true,
            version: health_body.version.unwrap_or_default(),
            capabilities_ok,
            action_count,
            config_errors,
            startup_errors,
            startup_warns,
            startup_infos,
            db_ok: health_body.db_ok,
            checked_at,
            error,
        },
        actions,
    }
}

fn local_hive_core_probe() -> ProductProbeSnapshot {
    let checks = startup::startup_checks();
    let (startup_errors, startup_warns, startup_infos) = startup::summarize_check_levels(&checks);
    let db_ok = db::health_check();
    let status = if startup_errors > 0 || !db_ok {
        "degraded"
    } else {
        "online"
    };
    let actions = vec![contract::action(
        "save_settings",
        "Save suite settings",
        "PUT",
        "/settings",
        "Persist suite-wide defaults and per-product launch/API overrides.",
        false,
    )];
    ProductProbeSnapshot {
        health: ProductHealthSnapshot {
            status: status.into(),
            reachable: true,
            version: PRODUCT_VERSION.into(),
            capabilities_ok: true,
            action_count: actions.len() as u32,
            config_errors: startup_errors,
            startup_errors,
            startup_warns,
            startup_infos,
            db_ok: Some(db_ok),
            checked_at: now_rfc3339(),
            error: String::new(),
        },
        actions,
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

fn sanitize_product_overrides(
    products: Vec<ProductOverrideInput>,
    existing: &HashMap<String, ProductOverride>,
) -> Vec<ProductOverride> {
    let mut deduped = HashMap::new();
    for product in products {
        let slug = product.slug.trim().to_string();
        let supplied_api_key = product.api_key.unwrap_or_default().trim().to_string();
        let api_key = if supplied_api_key.is_empty() {
            existing
                .get(&slug)
                .map(|item| item.api_key.clone())
                .unwrap_or_default()
        } else {
            supplied_api_key
        };
        deduped.insert(
            slug.clone(),
            ProductOverride {
                slug,
                frontend_url: product.frontend_url.trim().to_string(),
                api_url: product.api_url.trim().to_string(),
                api_key,
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

fn api_error(
    status: StatusCode,
    code: impl Into<String>,
    message: impl Into<String>,
) -> (StatusCode, Json<crate::models::ApiEnvelope<Value>>) {
    (status, Json(error(code, message, false)))
}

async fn fetch_product_capabilities(
    client: &reqwest::Client,
    api_url: &str,
) -> Result<contract::ProductCapabilities, String> {
    let normalized = api_url.trim_end_matches('/');
    let capabilities_url = format!("{normalized}/capabilities");
    let response = client
        .get(&capabilities_url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|_| "Could not reach /capabilities.".to_string())?;

    if !response.status().is_success() {
        return Err(format!("/capabilities returned HTTP {}", response.status()));
    }

    response
        .json::<contract::ProductCapabilities>()
        .await
        .map_err(|err| format!("Could not parse /capabilities: {err}"))
}

fn parse_dispatch_input(raw: Value) -> DispatchActionInput {
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

fn string_map_from_value(value: Option<&Value>) -> HashMap<String, String> {
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

fn fill_path_template(path: &str, path_params: &HashMap<String, String>) -> Result<String, String> {
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

fn build_target_url(
    api_url: &str,
    path: &str,
    query: &HashMap<String, String>,
) -> Result<Url, String> {
    let normalized = api_url.trim_end_matches('/');
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut url = Url::parse(&format!("{normalized}{path}"))
        .map_err(|err| format!("Could not build product action URL: {err}"))?;
    if !query.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query {
            pairs.append_pair(key, value);
        }
    }
    Ok(url)
}

fn parse_response_body(text: &str) -> Value {
    if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(text).unwrap_or_else(|_| json!({ "raw": text }))
    }
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
    use super::{fill_path_template, parse_dispatch_input, pick_url, summarize_products};
    use crate::models::{ProductHealthSnapshot, ProductRuntimeItem};
    use serde_json::json;
    use std::collections::HashMap;

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
                api_key_configured: false,
                notes: String::new(),
                status: "online".into(),
                health: ProductHealthSnapshot::default(),
                actions: Vec::new(),
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
                api_key_configured: false,
                notes: String::new(),
                status: "degraded".into(),
                health: ProductHealthSnapshot::default(),
                actions: Vec::new(),
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
                api_key_configured: false,
                notes: String::new(),
                status: "unconfigured".into(),
                health: ProductHealthSnapshot::default(),
                actions: Vec::new(),
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
                api_key_configured: false,
                notes: String::new(),
                status: "disabled".into(),
                health: ProductHealthSnapshot::default(),
                actions: Vec::new(),
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

    #[test]
    fn parse_dispatch_input_accepts_wrapped_payloads() {
        let input = parse_dispatch_input(json!({
            "path_params": { "name": "daily" },
            "query": { "dry": true },
            "payload": { "repo": "patchhive/example" }
        }));

        assert_eq!(input.path_params["name"], "daily");
        assert_eq!(input.query["dry"], "true");
        assert_eq!(input.payload["repo"], "patchhive/example");
    }

    #[test]
    fn parse_dispatch_input_treats_plain_object_as_payload() {
        let input = parse_dispatch_input(json!({ "repo": "patchhive/example" }));
        assert_eq!(input.payload["repo"], "patchhive/example");
        assert!(input.path_params.is_empty());
    }

    #[test]
    fn fill_path_template_requires_all_path_params() {
        let mut params = HashMap::new();
        params.insert("name".into(), "daily".into());
        assert_eq!(
            fill_path_template("/schedules/{name}/run", &params).unwrap(),
            "/schedules/daily/run"
        );
        assert!(fill_path_template("/schedules/{missing}/run", &params).is_err());
    }
}

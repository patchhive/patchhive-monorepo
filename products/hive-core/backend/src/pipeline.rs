use std::{collections::HashMap, time::Duration};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use patchhive_product_core::auth::SERVICE_TOKEN_HEADER;
use patchhive_product_core::contract;
use patchhive_product_core::startup::count_errors;
use reqwest::Url;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    auth::{
        auth_enabled, generate_and_save_key, generate_and_save_service_token,
        rotate_and_save_service_token, service_auth_enabled, service_token_generation_allowed,
        service_token_rotation_allowed, verify_token,
    },
    db,
    models::{
        error, DispatchActionResponse, OverviewResponse, OverviewSummary, ProductActionEvent,
        ProductContractCheck, ProductHealthSnapshot, ProductOverride, ProductOverrideInput,
        ProductRunDetailResponse, ProductRunsSnapshotResponse, ProductRuntimeItem,
        ProvisionServiceTokenRequest, ProvisionServiceTokenResponse, SaveSettingsRequest,
        SettingsResponse, PRODUCT_TITLE, PRODUCT_VERSION,
    },
    startup,
    state::{AppState, ProductDefinition},
};

mod dispatch;
mod overview;
mod provision;
mod settings;

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

#[derive(Debug, Clone, Default)]
struct ProductStoredAuth {
    service_token: String,
    legacy_api_key: String,
}

impl ProductStoredAuth {
    fn from_override(override_item: Option<&ProductOverride>) -> Self {
        Self {
            service_token: override_item
                .map(|item| item.service_token.trim().to_string())
                .unwrap_or_default(),
            legacy_api_key: override_item
                .map(|item| item.legacy_api_key.trim().to_string())
                .unwrap_or_default(),
        }
    }

    fn service_token_configured(&self) -> bool {
        !self.service_token.is_empty()
    }

    fn legacy_api_key_configured(&self) -> bool {
        !self.legacy_api_key.is_empty()
    }

    fn machine_auth_configured(&self) -> bool {
        self.service_token_configured() || self.legacy_api_key_configured()
    }

    fn auth_mode(&self) -> &'static str {
        if self.service_token_configured() {
            "service_token"
        } else if self.legacy_api_key_configured() {
            "legacy_api_key"
        } else {
            "none"
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ProductAuthStatusBody {
    #[serde(default)]
    auth_enabled: bool,
    #[serde(default)]
    service_auth_supported: bool,
    #[serde(default)]
    service_auth_enabled: bool,
    #[serde(default)]
    service_auth_scoped: bool,
    #[serde(default)]
    service_auth_legacy: bool,
    #[serde(default)]
    service_auth_scopes: Vec<String>,
    #[serde(default)]
    service_auth_expired: bool,
}

struct ProductProbeSnapshot {
    health: ProductHealthSnapshot,
    hivecore: Option<contract::HiveCoreLifecycleSupport>,
    actions: Vec<contract::ProductAction>,
    links: Vec<contract::ProductLink>,
    contract_checks: Vec<ProductContractCheck>,
    run_detail_template: String,
    recent_runs: Vec<contract::ProductRunSummary>,
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

pub async fn gen_key(
    headers: HeaderMap,
) -> Result<Json<Value>, patchhive_product_core::auth::JsonApiError> {
    if auth_enabled() {
        return Err(patchhive_product_core::auth::auth_already_configured_error());
    }
    if !crate::auth::bootstrap_request_allowed(&headers) {
        return Err(patchhive_product_core::auth::bootstrap_localhost_required_error());
    }
    let key = generate_and_save_key()
        .map_err(|err| patchhive_product_core::auth::key_generation_failed_error(&err))?;
    Ok(Json(
        json!({"api_key": key, "message": "Store this — it won't be shown again"}),
    ))
}

pub async fn gen_service_token(
    headers: HeaderMap,
) -> Result<Json<Value>, patchhive_product_core::auth::JsonApiError> {
    if service_auth_enabled() {
        return Err(patchhive_product_core::auth::service_auth_already_configured_error());
    }
    if !service_token_generation_allowed(&headers) {
        return Err(patchhive_product_core::auth::service_token_generation_forbidden_error());
    }
    let token = generate_and_save_service_token()
        .map_err(|err| patchhive_product_core::auth::service_token_generation_failed_error(&err))?;
    Ok(Json(json!({
        "service_token": token,
        "message": "Store this for HiveCore or other PatchHive service callers — it won't be shown again"
    })))
}

pub async fn rotate_service_token(
    headers: HeaderMap,
) -> Result<Json<Value>, patchhive_product_core::auth::JsonApiError> {
    if !service_auth_enabled() {
        return Err(patchhive_product_core::auth::service_auth_not_configured_error());
    }
    if !service_token_rotation_allowed(&headers) {
        return Err(patchhive_product_core::auth::service_token_rotation_forbidden_error());
    }
    let token = rotate_and_save_service_token()
        .map_err(|err| patchhive_product_core::auth::service_token_rotation_failed_error(&err))?;
    Ok(Json(json!({
        "service_token": token,
        "message": "Store this replacement service token for HiveCore or other PatchHive service callers — it won't be shown again"
    })))
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
    Json(contract::runs_from_values(
        "hive-core",
        hive_core_action_run_values(30),
    ))
}

pub async fn run_detail(
    Path(id): Path<String>,
) -> Result<Json<ProductActionEvent>, (StatusCode, Json<crate::models::ApiEnvelope<Value>>)> {
    db::action_event(&id)
        .map(Json)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "run_not_found", "Run was not found."))
}

pub async fn overview(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<OverviewResponse>> {
    overview::overview(State(state)).await
}

pub async fn products(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<Vec<ProductRuntimeItem>>> {
    overview::products(State(state)).await
}

pub async fn product_runs(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<
    Json<crate::models::ApiEnvelope<ProductRunsSnapshotResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    overview::product_runs(State(state), Path(slug)).await
}

pub async fn product_run_detail(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Result<
    Json<crate::models::ApiEnvelope<ProductRunDetailResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    overview::product_run_detail(State(state), Path((slug, id))).await
}

pub async fn settings() -> Json<crate::models::ApiEnvelope<SettingsResponse>> {
    settings::settings().await
}

pub async fn recent_actions() -> Json<crate::models::ApiEnvelope<Vec<ProductActionEvent>>> {
    dispatch::recent_actions().await
}

pub async fn provision_service_token(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(body): Json<ProvisionServiceTokenRequest>,
) -> Result<
    Json<crate::models::ApiEnvelope<ProvisionServiceTokenResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    provision::provision_service_token(State(state), Path(slug), Json(body)).await
}

pub async fn save_settings(
    Json(body): Json<SaveSettingsRequest>,
) -> Result<
    Json<crate::models::ApiEnvelope<SettingsResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    settings::save_settings(Json(body)).await
}

pub async fn dispatch_product_action(
    State(state): State<AppState>,
    Path((slug, action_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<
    Json<crate::models::ApiEnvelope<DispatchActionResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    dispatch::dispatch_product_action(State(state), Path((slug, action_id)), Json(body)).await
}

fn resolved_auth_mode(definition: &ProductDefinition, auth: &ProductStoredAuth) -> String {
    if definition.slug == "hive-core" {
        "native".into()
    } else {
        auth.auth_mode().into()
    }
}

fn resolved_machine_auth_configured(
    definition: &ProductDefinition,
    auth: &ProductStoredAuth,
) -> bool {
    definition.slug == "hive-core" || auth.machine_auth_configured()
}

fn resolved_service_token_configured(
    definition: &ProductDefinition,
    auth: &ProductStoredAuth,
) -> bool {
    definition.slug != "hive-core" && auth.service_token_configured()
}

fn resolved_legacy_api_key_configured(
    definition: &ProductDefinition,
    auth: &ProductStoredAuth,
) -> bool {
    definition.slug != "hive-core" && auth.legacy_api_key_configured()
}

fn pick_url(override_url: Option<&str>, default_url: &str) -> String {
    let override_url = override_url.unwrap_or("").trim();
    if override_url.is_empty() {
        default_url.to_string()
    } else {
        override_url.to_string()
    }
}

fn contract_check(
    id: impl Into<String>,
    label: impl Into<String>,
    path: impl Into<String>,
    ok: bool,
    status: impl Into<String>,
    error: impl Into<String>,
) -> ProductContractCheck {
    ProductContractCheck {
        id: id.into(),
        label: label.into(),
        path: path.into(),
        ok,
        status: status.into(),
        error: error.into(),
    }
}

fn contract_drift_count(checks: &[ProductContractCheck]) -> u32 {
    checks
        .iter()
        .filter(|check| {
            !check.ok
                && !matches!(
                    check.status.as_str(),
                    "locked" | "skipped" | "disabled" | "unconfigured"
                )
        })
        .count() as u32
}

fn contract_checks_for_unavailable_product(status: &str) -> Vec<ProductContractCheck> {
    let reason = if status == "disabled" {
        "Product is disabled in HiveCore settings."
    } else {
        "Product API URL is not configured."
    };
    [
        ("health", "Health", "/health"),
        ("startup_checks", "Startup checks", "/startup/checks"),
        ("capabilities", "Capabilities", "/capabilities"),
        ("runs", "Runs", "/runs"),
        ("run_detail", "Run detail", "/runs/{id}"),
    ]
    .into_iter()
    .map(|(id, label, path)| contract_check(id, label, path, false, status, reason))
    .collect()
}

fn contract_checks_for_failed_health() -> Vec<ProductContractCheck> {
    contract_checks_with_health_error("Could not reach /health.")
}

fn contract_checks_with_health_error(error: impl Into<String>) -> Vec<ProductContractCheck> {
    let error = error.into();
    vec![
        contract_check("health", "Health", "/health", false, "failed", error),
        contract_check(
            "startup_checks",
            "Startup checks",
            "/startup/checks",
            false,
            "skipped",
            "Health must pass before startup checks are meaningful.",
        ),
        contract_check(
            "capabilities",
            "Capabilities",
            "/capabilities",
            false,
            "skipped",
            "Health must pass before capabilities are meaningful.",
        ),
        contract_check(
            "runs",
            "Runs",
            "/runs",
            false,
            "skipped",
            "Health must pass before run history is meaningful.",
        ),
        contract_check(
            "run_detail",
            "Run detail",
            "/runs/{id}",
            false,
            "skipped",
            "Health must pass before run detail support is meaningful.",
        ),
    ]
}

fn hive_core_action_run_values(limit: u32) -> Vec<Value> {
    db::recent_action_events(limit)
        .into_iter()
        .map(|event| {
            let summary = if event.error.is_empty() {
                format!(
                    "{} {} returned {}",
                    event.method,
                    event.path,
                    event
                        .remote_status
                        .map(|status| status.to_string())
                        .unwrap_or_else(|| "no remote status".into())
                )
            } else {
                event.error.clone()
            };
            json!({
                "id": event.id.clone(),
                "status": event.status.clone(),
                "title": format!("{} · {}", event.product_slug, event.action_label),
                "summary": summary,
                "created_at": event.created_at.clone(),
                "updated_at": event.created_at.clone(),
                "raw": event,
            })
        })
        .collect()
}

fn sanitize_product_overrides(
    products: Vec<ProductOverrideInput>,
    existing: &HashMap<String, ProductOverride>,
) -> Vec<ProductOverride> {
    settings::sanitize_product_overrides(products, existing)
}

fn persist_product_override(product: ProductOverride) -> Result<(), String> {
    let mut overrides = db::product_overrides();
    overrides.insert(product.slug.clone(), product);
    let mut rows = overrides.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| left.slug.cmp(&right.slug));
    db::replace_product_overrides(&rows).map_err(|err| {
        tracing::warn!("failed to persist HiveCore product override: {err}");
        err.to_string()
    })
}

fn dispatch_service_token_issue(
    product_title: &str,
    action: &contract::ProductAction,
    auth_status: &ProductAuthStatusBody,
) -> Option<(&'static str, String)> {
    dispatch::dispatch_service_token_issue(product_title, action, auth_status)
}

fn api_error(
    status: StatusCode,
    code: impl Into<String>,
    message: impl Into<String>,
) -> (StatusCode, Json<crate::models::ApiEnvelope<Value>>) {
    (status, Json(error(code, message, false)))
}

async fn fetch_product_auth_status(
    client: &reqwest::Client,
    api_url: &str,
) -> Result<ProductAuthStatusBody, String> {
    let normalized = api_url.trim_end_matches('/');
    let auth_status_url = format!("{normalized}/auth/status");
    let response = client
        .get(&auth_status_url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|_| "Could not reach /auth/status.".to_string())?;

    if !response.status().is_success() {
        return Err(format!("/auth/status returned HTTP {}", response.status()));
    }

    response
        .json::<ProductAuthStatusBody>()
        .await
        .map_err(|err| format!("Could not parse /auth/status: {err}"))
}

async fn fetch_product_capabilities(
    client: &reqwest::Client,
    api_url: &str,
    auth: &ProductStoredAuth,
) -> Result<contract::ProductCapabilities, String> {
    let normalized = api_url.trim_end_matches('/');
    let capabilities_url = format!("{normalized}/capabilities");
    let response = authorized_get(client, &capabilities_url, auth)
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

async fn fetch_product_runs(
    client: &reqwest::Client,
    api_url: &str,
    auth: &ProductStoredAuth,
) -> (bool, Vec<contract::ProductRunSummary>, String) {
    if !auth.machine_auth_configured() {
        return (
            false,
            Vec::new(),
            "Product service token missing; recent runs unavailable.".into(),
        );
    }

    let normalized = api_url.trim_end_matches('/');
    let runs_url = format!("{normalized}/runs");
    let response = authorized_get(client, &runs_url, auth)
        .timeout(Duration::from_secs(3))
        .send()
        .await;

    let Ok(response) = response else {
        return (false, Vec::new(), "Could not reach /runs.".into());
    };

    if !response.status().is_success() {
        return (
            false,
            Vec::new(),
            format!("/runs returned HTTP {}", response.status()),
        );
    }

    match response.json::<contract::ProductRunsResponse>().await {
        Ok(body) => (true, body.runs.into_iter().take(6).collect(), String::new()),
        Err(err) => (false, Vec::new(), format!("Could not parse /runs: {err}")),
    }
}

fn remote_error_message(value: &Value) -> Option<String> {
    value
        .get("error")
        .and_then(Value::as_str)
        .map(|message| message.to_string())
        .or_else(|| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .map(|message| message.to_string())
        })
}

fn authorized_get(
    client: &reqwest::Client,
    url: &str,
    auth: &ProductStoredAuth,
) -> reqwest::RequestBuilder {
    let request = client.get(url);
    authorized_request(request, auth)
}

fn authorized_request(
    request: reqwest::RequestBuilder,
    auth: &ProductStoredAuth,
) -> reqwest::RequestBuilder {
    if auth.service_token_configured() {
        request.header(SERVICE_TOKEN_HEADER, auth.service_token.trim())
    } else if auth.legacy_api_key_configured() {
        request.header("X-API-Key", auth.legacy_api_key.trim())
    } else {
        request
    }
}

fn parse_dispatch_input(raw: Value) -> DispatchActionInput {
    dispatch::parse_dispatch_input(raw)
}

fn fill_path_template(path: &str, path_params: &HashMap<String, String>) -> Result<String, String> {
    dispatch::fill_path_template(path, path_params)
}

fn build_run_detail_path(template: &str, id: &str) -> Result<String, String> {
    overview::build_run_detail_path(template, id)
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
    overview::summarize_products(products)
}

#[cfg(test)]
mod tests {
    use super::{
        authorized_request, build_run_detail_path, contract_check, contract_drift_count,
        dispatch_service_token_issue, fill_path_template, parse_dispatch_input, pick_url,
        sanitize_product_overrides, summarize_products, ProductAuthStatusBody, ProductStoredAuth,
        SERVICE_TOKEN_HEADER,
    };
    use crate::models::{
        now_rfc3339, ProductHealthSnapshot, ProductOverride, ProductOverrideInput,
        ProductRuntimeItem,
    };
    use patchhive_product_core::contract;
    use reqwest::Client;
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
                auth_mode: "none".into(),
                machine_auth_configured: false,
                service_token_configured: false,
                legacy_api_key_configured: false,
                notes: String::new(),
                status: "online".into(),
                health: ProductHealthSnapshot::default(),
                hivecore: None,
                actions: Vec::new(),
                links: Vec::new(),
                contract_checks: Vec::new(),
                contract_drift_count: 0,
                run_detail_template: String::new(),
                recent_runs: Vec::new(),
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
                auth_mode: "none".into(),
                machine_auth_configured: false,
                service_token_configured: false,
                legacy_api_key_configured: false,
                notes: String::new(),
                status: "degraded".into(),
                health: ProductHealthSnapshot::default(),
                hivecore: None,
                actions: Vec::new(),
                links: Vec::new(),
                contract_checks: Vec::new(),
                contract_drift_count: 0,
                run_detail_template: String::new(),
                recent_runs: Vec::new(),
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
                auth_mode: "none".into(),
                machine_auth_configured: false,
                service_token_configured: false,
                legacy_api_key_configured: false,
                notes: String::new(),
                status: "unconfigured".into(),
                health: ProductHealthSnapshot::default(),
                hivecore: None,
                actions: Vec::new(),
                links: Vec::new(),
                contract_checks: Vec::new(),
                contract_drift_count: 0,
                run_detail_template: String::new(),
                recent_runs: Vec::new(),
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
                auth_mode: "none".into(),
                machine_auth_configured: false,
                service_token_configured: false,
                legacy_api_key_configured: false,
                notes: String::new(),
                status: "disabled".into(),
                health: ProductHealthSnapshot::default(),
                hivecore: None,
                actions: Vec::new(),
                links: Vec::new(),
                contract_checks: Vec::new(),
                contract_drift_count: 0,
                run_detail_template: String::new(),
                recent_runs: Vec::new(),
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

    #[test]
    fn build_run_detail_path_rejects_unsafe_ids() {
        assert_eq!(
            build_run_detail_path("/runs/{id}", "run_123").unwrap(),
            "/runs/run_123"
        );
        assert!(build_run_detail_path("/runs/{id}", "../secret").is_err());
        assert!(build_run_detail_path("/runs/{id}", "run?x=1").is_err());
    }

    #[test]
    fn contract_drift_ignores_operator_locked_states() {
        let checks = vec![
            contract_check("health", "Health", "/health", true, "ok", ""),
            contract_check("runs", "Runs", "/runs", false, "locked", "API key missing"),
            contract_check("detail", "Run detail", "/runs/{id}", false, "missing", ""),
        ];
        assert_eq!(contract_drift_count(&checks), 1);
    }

    #[test]
    fn sanitize_product_overrides_replaces_legacy_key_when_service_token_is_supplied() {
        let existing = HashMap::from([(
            "signal-hive".to_string(),
            ProductOverride {
                slug: "signal-hive".into(),
                frontend_url: String::new(),
                api_url: "http://localhost:8010".into(),
                service_token: String::new(),
                legacy_api_key: "signal-operator".into(),
                enabled: true,
                notes: String::new(),
                updated_at: now_rfc3339(),
            },
        )]);

        let sanitized = sanitize_product_overrides(
            vec![ProductOverrideInput {
                slug: "signal-hive".into(),
                frontend_url: String::new(),
                api_url: "http://localhost:8010".into(),
                service_token: Some("svc_signal".into()),
                legacy_api_key: None,
                enabled: true,
                notes: String::new(),
            }],
            &existing,
        );

        assert_eq!(sanitized.len(), 1);
        assert_eq!(sanitized[0].service_token, "svc_signal");
        assert!(sanitized[0].legacy_api_key.is_empty());
    }

    #[test]
    fn authorized_request_prefers_service_token_header() {
        let request = authorized_request(
            Client::new().get("http://example.com"),
            &ProductStoredAuth {
                service_token: "svc_signal".into(),
                legacy_api_key: "signal-operator".into(),
            },
        )
        .build()
        .expect("request should build");

        assert_eq!(
            request
                .headers()
                .get(SERVICE_TOKEN_HEADER)
                .and_then(|value| value.to_str().ok()),
            Some("svc_signal")
        );
        assert!(request.headers().get("X-API-Key").is_none());
    }

    #[test]
    fn dispatch_service_token_issue_blocks_legacy_machine_tokens() {
        let action = contract::ProductAction {
            id: "scan".into(),
            label: "Run signal scan".into(),
            method: "POST".into(),
            path: "/scan".into(),
            description: String::new(),
            starts_run: true,
            destructive: false,
            required_scopes: vec!["actions:dispatch".into()],
        };
        let auth_status = ProductAuthStatusBody {
            service_auth_enabled: true,
            service_auth_scoped: false,
            service_auth_legacy: true,
            service_auth_scopes: vec!["runs:read".into()],
            ..ProductAuthStatusBody::default()
        };

        let issue = dispatch_service_token_issue("SignalHive", &action, &auth_status)
            .expect("legacy token should be rejected");
        assert_eq!(issue.0, "service_token_rotation_required");
    }

    #[test]
    fn dispatch_service_token_issue_blocks_expired_machine_tokens() {
        let action = contract::ProductAction {
            id: "scan".into(),
            label: "Run signal scan".into(),
            method: "POST".into(),
            path: "/scan".into(),
            description: String::new(),
            starts_run: true,
            destructive: false,
            required_scopes: vec!["actions:dispatch".into()],
        };
        let auth_status = ProductAuthStatusBody {
            service_auth_enabled: true,
            service_auth_scoped: true,
            service_auth_expired: true,
            service_auth_scopes: vec!["actions:dispatch".into()],
            ..ProductAuthStatusBody::default()
        };

        let issue = dispatch_service_token_issue("SignalHive", &action, &auth_status)
            .expect("expired token should be rejected");
        assert_eq!(issue.0, "service_token_expired");
    }
}

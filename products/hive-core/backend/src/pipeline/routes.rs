use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use patchhive_product_core::contract;
use patchhive_product_core::startup::count_errors;
use serde_json::{json, Value};

use crate::{
    auth::{
        auth_enabled, generate_and_save_key, generate_and_save_service_token,
        rotate_and_save_service_token, service_auth_enabled, service_token_generation_allowed,
        service_token_rotation_allowed, verify_token,
    },
    db,
    models::{
        DispatchActionResponse, OverviewResponse, ProductActionEvent, ProductRunDetailResponse,
        ProductRunsSnapshotResponse, ProductRuntimeItem, ProvisionServiceTokenRequest,
        ProvisionServiceTokenResponse, SaveSettingsRequest, SettingsResponse, PRODUCT_TITLE,
        PRODUCT_VERSION,
    },
    startup,
    state::AppState,
};

use super::types::{api_error, LoginBody};

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
        super::hive_core_action_run_values(30),
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
    super::overview::overview(State(state)).await
}

pub async fn products(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<Vec<ProductRuntimeItem>>> {
    super::overview::products(State(state)).await
}

pub async fn product_runs(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<
    Json<crate::models::ApiEnvelope<ProductRunsSnapshotResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::overview::product_runs(State(state), Path(slug)).await
}

pub async fn product_run_detail(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Result<
    Json<crate::models::ApiEnvelope<ProductRunDetailResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::overview::product_run_detail(State(state), Path((slug, id))).await
}

pub async fn settings() -> Json<crate::models::ApiEnvelope<SettingsResponse>> {
    super::settings::settings().await
}

pub async fn recent_actions() -> Json<crate::models::ApiEnvelope<Vec<ProductActionEvent>>> {
    super::dispatch::recent_actions().await
}

pub async fn provision_service_token(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(body): Json<ProvisionServiceTokenRequest>,
) -> Result<
    Json<crate::models::ApiEnvelope<ProvisionServiceTokenResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::provision::provision_service_token(State(state), Path(slug), Json(body)).await
}

pub async fn save_settings(
    Json(body): Json<SaveSettingsRequest>,
) -> Result<
    Json<crate::models::ApiEnvelope<SettingsResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::settings::save_settings(Json(body)).await
}

pub async fn first_stack_status(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<crate::models::FirstStackSetupResponse>> {
    super::setup::first_stack_status(State(state)).await
}

pub async fn start_first_stack(
    State(state): State<AppState>,
) -> Result<
    Json<crate::models::ApiEnvelope<crate::models::FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::setup::start_first_stack(State(state)).await
}

pub async fn pair_first_stack(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<crate::models::FirstStackSetupResponse>> {
    super::setup::pair_first_stack(State(state)).await
}

pub async fn run_first_stack_smoke(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<crate::models::FirstStackSetupResponse>> {
    super::smoke::run_first_stack_smoke(State(state)).await
}

pub async fn stop_first_stack(
    State(state): State<AppState>,
) -> Result<
    Json<crate::models::ApiEnvelope<crate::models::FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::setup::stop_first_stack(State(state)).await
}

pub async fn start_setup_product(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<
    Json<crate::models::ApiEnvelope<crate::models::FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::setup::start_setup_product(State(state), Path(slug)).await
}

pub async fn stop_setup_product(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<
    Json<crate::models::ApiEnvelope<crate::models::FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::setup::stop_setup_product(State(state), Path(slug)).await
}

pub async fn restart_setup_product(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<
    Json<crate::models::ApiEnvelope<crate::models::FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::setup::restart_setup_product(State(state), Path(slug)).await
}

pub async fn setup_product_logs(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    axum::extract::Query(query): axum::extract::Query<super::setup::ProductLogsQuery>,
) -> Result<
    Json<crate::models::ApiEnvelope<crate::models::SetupProductLogsResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::setup::setup_product_logs(State(state), Path(slug), axum::extract::Query(query)).await
}

pub async fn dispatch_product_action(
    State(state): State<AppState>,
    Path((slug, action_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<
    Json<crate::models::ApiEnvelope<DispatchActionResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    super::dispatch::dispatch_product_action(State(state), Path((slug, action_id)), Json(body))
        .await
}

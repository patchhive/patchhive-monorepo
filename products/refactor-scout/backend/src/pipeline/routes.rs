use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use patchhive_product_core::contract;
use patchhive_product_core::startup::count_errors;
use serde_json::json;

use crate::{
    auth::{
        auth_enabled, generate_and_save_key, generate_and_save_service_token,
        rotate_and_save_service_token, service_auth_enabled, service_token_generation_allowed,
        service_token_rotation_allowed, verify_token,
    },
    db,
    models::{HistoryItem, OverviewPayload, RefactorScanResult, ScanRequest},
    state::AppState,
    STARTUP_CHECKS,
};

use super::analysis::scan_request_allowed;
use super::scanning::{build_scan_result, MAX_SCAN_FILES};

type ApiError = (StatusCode, Json<serde_json::Value>);
pub type JsonResult<T> = Result<Json<T>, ApiError>;

#[derive(serde::Deserialize)]
pub struct LoginBody {
    pub(crate) api_key: String,
}

pub async fn capabilities() -> Json<contract::ProductCapabilities> {
    Json(contract::capabilities(
        "refactor-scout",
        "RefactorScout",
        vec![contract::action(
            "scan_local_repo",
            "Scan local repo",
            "POST",
            "/scan/local",
            "Surface safe refactor opportunities from an allowed local repository path.",
            true,
        )],
        vec![
            contract::link("overview", "Overview", "/overview"),
            contract::link("history", "History", "/history"),
        ],
    ))
}

pub async fn runs() -> Json<contract::ProductRunsResponse> {
    Json(contract::runs_from_history(
        "refactor-scout",
        db::history(30),
    ))
}

pub async fn auth_status() -> Json<serde_json::Value> {
    Json(crate::auth::auth_status_payload())
}

pub async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
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
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
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
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
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
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
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

pub async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();
    let counts = db::overview_counts();

    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "RefactorScout by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "scan_count": counts.scans,
        "repo_count": counts.repos,
        "opportunity_count": counts.opportunities,
        "high_safety_count": counts.high_safety,
        "allowed_roots": state.allowed_root_labels(),
        "remote_fs_enabled": state.remote_fs_enabled,
        "mode": "local-refactor-scout",
    }))
}

pub async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

pub async fn overview(State(state): State<AppState>) -> Json<OverviewPayload> {
    let counts = db::overview_counts();
    Json(OverviewPayload {
        product: "RefactorScout by PatchHive".into(),
        tagline: "Surface safe, high-value refactors before code quality drift turns expensive."
            .into(),
        scan_count: counts.scans,
        repo_count: counts.repos,
        opportunity_count: counts.opportunities,
        high_safety_count: counts.high_safety,
        large_file_count: counts.large_file_count,
        long_function_count: counts.long_function_count,
        repeated_literal_count: counts.repeated_literal_count,
        last_repo: counts.last_repo,
        allowed_roots: state.allowed_root_labels(),
        remote_fs_enabled: state.remote_fs_enabled,
    })
}

pub async fn history() -> Json<Vec<HistoryItem>> {
    Json(db::history(30))
}

pub async fn history_detail(AxumPath(id): AxumPath<String>) -> JsonResult<RefactorScanResult> {
    db::get_scan(&id)
        .map(Json)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "RefactorScout scan not found"))
}

pub async fn scan_local_repo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ScanRequest>,
) -> JsonResult<RefactorScanResult> {
    if !scan_request_allowed(&headers, state.remote_fs_enabled) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Filesystem scans are limited to localhost callers unless REFACTOR_SCOUT_ALLOW_REMOTE_FS=true.",
        ));
    }

    let repo_path = request.repo_path.trim();
    if repo_path.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Repository path is required.",
        ));
    }

    let max_files = request.max_files.clamp(25, MAX_SCAN_FILES);
    let result = build_scan_result(&state, repo_path, max_files)
        .map_err(|err| api_error(StatusCode::BAD_REQUEST, err.to_string()))?;

    db::save_scan(&result)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(result))
}

pub(crate) fn api_error(status: StatusCode, error: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": error.into() })))
}

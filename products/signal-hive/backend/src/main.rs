mod auth;
mod db;
mod github;
mod models;
mod pipeline;
mod startup;
mod state;

use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use patchhive_product_core::rate_limit::rate_limit_middleware;
use patchhive_product_core::startup::{
    cors_layer, count_errors, listen_addr, log_checks, StartupCheck,
};
use serde_json::json;
use tracing::info;

use crate::auth::{auth_enabled, generate_and_save_key, verify_token};
use crate::state::AppState;

static STARTUP_CHECKS: OnceCell<Vec<StartupCheck>> = OnceCell::new();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let _ = dotenvy::dotenv();

    if let Err(err) = db::init_db() {
        eprintln!("DB init failed: {err}");
        std::process::exit(1);
    }

    let state = AppState::new();
    let checks = startup::validate_config(&state.http).await;
    log_checks(&checks);
    let _ = STARTUP_CHECKS.set(checks);
    pipeline::start_scheduler(state.clone());

    let cors = cors_layer();

    let app = Router::new()
        .route("/auth/status", get(auth_status))
        .route("/auth/login", post(login))
        .route("/auth/generate-key", post(gen_key))
        .route("/health", get(health))
        .route("/startup/checks", get(startup_checks_route))
        .route("/capabilities", get(pipeline::capabilities))
        .route("/runs", get(pipeline::runs))
        .route("/runs/:id", get(pipeline::history_detail))
        .route("/presets", get(scan_presets).post(save_scan_preset))
        .route("/presets/:name", delete(delete_scan_preset))
        .route("/schedules", get(scan_schedules).post(save_scan_schedule))
        .route("/schedules/:name", delete(delete_scan_schedule))
        .route("/schedules/:name/run", post(run_scan_schedule_now))
        .route("/repo-lists", get(repo_lists).post(add_repo_list))
        .route("/repo-lists/*repo", delete(remove_repo_list))
        .route("/scan", post(pipeline::scan))
        .route("/history", get(pipeline::history))
        .route("/history/:id", get(pipeline::history_detail))
        .route("/history/:id/timeline", get(pipeline::timeline))
        .route("/history/:id/report", get(pipeline::report))
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(cors)
        .with_state(state);

    let addr = listen_addr("SIGNAL_PORT", 8010);
    info!("📡 SignalHive by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind SignalHive to {addr}: {err}"));
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|err| panic!("SignalHive server failed: {err}"));
}

async fn auth_status() -> Json<serde_json::Value> {
    Json(auth::auth_status_payload())
}

#[derive(serde::Deserialize)]
struct LoginBody {
    api_key: String,
}

async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
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

async fn gen_key(
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if auth_enabled() {
        return Err(patchhive_product_core::auth::auth_already_configured_error());
    }
    if !auth::bootstrap_request_allowed(&headers) {
        return Err(patchhive_product_core::auth::bootstrap_localhost_required_error());
    }
    let key = generate_and_save_key()
        .map_err(|err| patchhive_product_core::auth::key_generation_failed_error(&err))?;
    Ok(Json(
        json!({"api_key": key, "message": "Store this — it won't be shown again"}),
    ))
}

async fn health(State(_state): State<AppState>) -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();
    let repo_lists = db::list_repo_lists().unwrap_or_default();
    let schedules = db::list_scan_schedules().unwrap_or_default();
    let allowlist_count = repo_lists
        .iter()
        .filter(|row| row.list_type == "allowlist")
        .count();
    let denylist_count = repo_lists
        .iter()
        .filter(|row| row.list_type == "denylist")
        .count();
    let opt_out_count = repo_lists
        .iter()
        .filter(|row| row.list_type == "opt_out")
        .count();
    let enabled_schedule_count = schedules.iter().filter(|schedule| schedule.enabled).count();
    let next_run_at = schedules
        .iter()
        .filter(|schedule| schedule.enabled)
        .map(|schedule| schedule.next_run_at.clone())
        .min();

    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "SignalHive by PatchHive",
        "scan_count": db::scan_count(),
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "read_only": true,
        "repo_lists": {
            "allowlist": allowlist_count,
            "denylist": denylist_count,
            "opt_out": opt_out_count,
        },
        "schedules": {
            "total": schedules.len(),
            "enabled": enabled_schedule_count,
            "next_run_at": next_run_at,
        },
    }))
}

async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

async fn repo_lists() -> Json<serde_json::Value> {
    Json(json!({
        "repos": db::list_repo_lists().unwrap_or_default(),
    }))
}

async fn scan_presets() -> Json<serde_json::Value> {
    Json(json!({
        "presets": db::list_scan_presets().unwrap_or_default(),
    }))
}

async fn scan_schedules() -> Json<serde_json::Value> {
    Json(json!({
        "schedules": db::list_scan_schedules().unwrap_or_default(),
    }))
}

#[derive(serde::Deserialize)]
struct RepoListBody {
    repo: String,
    list_type: String,
}

#[derive(serde::Deserialize)]
struct ScanPresetBody {
    name: String,
    params: crate::models::ScanParams,
}

#[derive(serde::Deserialize)]
struct ScanScheduleBody {
    name: String,
    params: crate::models::ScanParams,
    cadence_hours: u32,
    enabled: bool,
}

async fn save_scan_preset(
    Json(body): Json<ScanPresetBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    db::save_scan_preset(name, &body.params).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true, "name": name })))
}

async fn delete_scan_preset(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    db::delete_scan_preset(&name).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true })))
}

async fn save_scan_schedule(
    Json(body): Json<ScanScheduleBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    db::save_scan_schedule(name, &body.params, body.cadence_hours.max(1), body.enabled)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true, "name": name })))
}

async fn delete_scan_schedule(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    db::delete_scan_schedule(&name).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true })))
}

async fn run_scan_schedule_now(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<crate::models::ScanRecord>, StatusCode> {
    if name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    pipeline::run_schedule_now(&state, &name)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn add_repo_list(
    Json(body): Json<RepoListBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(repo) = db::normalize_repo_name(&body.repo) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    let Some(list_type) = db::normalize_repo_list_type(&body.list_type) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    db::save_repo_list(&repo, list_type).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        json!({ "ok": true, "repo": repo, "list_type": list_type }),
    ))
}

async fn remove_repo_list(
    axum::extract::Path(repo): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(repo) = db::normalize_repo_name(&repo) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    db::delete_repo_list(&repo).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true })))
}

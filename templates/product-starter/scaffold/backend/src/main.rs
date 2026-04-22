mod auth;
mod db;
mod startup;
mod state;

use axum::{
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use patchhive_product_core::rate_limit::rate_limit_middleware;
use patchhive_product_core::startup::{
    cors_layer, count_errors, listen_addr, log_checks, StartupCheck,
};
use serde_json::json;
use tracing::info;

use crate::{
    auth::{auth_enabled, generate_and_save_key, verify_token},
    state::AppState,
};

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

    let checks = startup::validate_config().await;
    log_checks(&checks);
    let _ = STARTUP_CHECKS.set(checks);

    let cors = cors_layer();

    let app = Router::new()
        .route("/auth/status", get(auth_status))
        .route("/auth/login", post(login))
        .route("/auth/generate-key", post(gen_key))
        .route("/health", get(health))
        .route("/startup/checks", get(startup_checks_route))
        .route("/overview", get(overview))
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(cors)
        .with_state(AppState::new());

    let addr = listen_addr("__ENV_PREFIX___PORT", __BACKEND_PORT__);
    info!("__PRODUCT_ICON__ __PRODUCT_TITLE__ by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind __PRODUCT_TITLE__ to {addr}: {err}"));
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|err| panic!("__PRODUCT_TITLE__ server failed: {err}"));
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

async fn gen_key(headers: axum::http::HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
    if auth_enabled() {
        return Err(StatusCode::FORBIDDEN);
    }
    if !auth::bootstrap_request_allowed(&headers) {
        return Err(StatusCode::FORBIDDEN);
    }
    let key = generate_and_save_key().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        json!({"api_key": key, "message": "Store this — it won't be shown again"}),
    ))
}

async fn health() -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();

    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "__PRODUCT_TITLE__ by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "meta_count": db::meta_count(),
        "mode": "starter",
    }))
}

async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

async fn overview() -> Json<serde_json::Value> {
    Json(json!({
        "product": "__PRODUCT_TITLE__ by PatchHive",
        "tagline": "__PRODUCT_TAGLINE__",
        "starter": true,
        "message": "Replace this overview route with product-specific logic once the product loop is real."
    }))
}

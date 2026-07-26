patchhive_product_core::define_api_key_auth_module! {
    pub mod auth {
        patchhive_product_core::auth::ApiKeyAuthConfig::new("__ENV_PREFIX___API_KEY_HASH", "__PRODUCT_SLUG__-")
            .with_service_token("__ENV_PREFIX___SERVICE_TOKEN_HASH", "__PRODUCT_SLUG__-svc-")
            .with_service_default_name("hivecore")
            .with_public_paths([
                "/health",
                "/auth/login",
                "/auth/status",
                "/auth/generate-key",
                "/startup/checks",
                "/api/products/__PRODUCT_SLUG__/health",
                "/api/products/__PRODUCT_SLUG__/auth/login",
                "/api/products/__PRODUCT_SLUG__/auth/status",
                "/api/products/__PRODUCT_SLUG__/auth/generate-key",
                "/api/products/__PRODUCT_SLUG__/startup/checks",
            ])
    }
}

pub mod db;
pub mod startup;
pub mod state;

use anyhow::Result;
use axum::{
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use patchhive_product_core::rate_limit::rate_limit_middleware;
use patchhive_product_core::startup::{count_errors, log_checks, StartupCheck};
use serde_json::json;

use auth::{auth_enabled, generate_and_save_key, verify_token};

pub static STARTUP_CHECKS: OnceCell<Vec<StartupCheck>> = OnceCell::new();

pub async fn init_runtime() -> Result<()> {
    db::init_db()?;
    let checks = startup::validate_config().await;
    log_checks(&checks);
    let _ = STARTUP_CHECKS.set(checks);
    Ok(())
}

pub fn router() -> Router {
    Router::new()
        .route("/auth/status", get(auth_status))
        .route("/auth/login", post(login))
        .route("/auth/generate-key", post(gen_key))
        .route("/health", get(health))
        .route("/startup/checks", get(startup_checks_route))
        .route("/overview", get(overview))
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .with_state(state::AppState::new())
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
        "mode": "starter"
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
        "message": "Replace this route with product-specific logic as soon as the product loop is real."
    }))
}

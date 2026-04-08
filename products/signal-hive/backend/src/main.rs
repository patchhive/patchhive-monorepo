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
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::auth::{auth_enabled, generate_and_save_key, verify_token};
use crate::state::AppState;

static STARTUP_CHECKS: OnceCell<Vec<serde_json::Value>> = OnceCell::new();

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
    for check in &checks {
        match check["level"].as_str() {
            Some("error") => tracing::error!("Config: {}", check["msg"].as_str().unwrap_or("")),
            Some("warn") => tracing::warn!("Config: {}", check["msg"].as_str().unwrap_or("")),
            _ => info!("Config: {}", check["msg"].as_str().unwrap_or("")),
        }
    }
    let _ = STARTUP_CHECKS.set(checks);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/auth/status", get(auth_status))
        .route("/auth/login", post(login))
        .route("/auth/generate-key", post(gen_key))
        .route("/health", get(health))
        .route("/startup/checks", get(startup_checks_route))
        .route("/scan", post(pipeline::scan))
        .route("/history", get(pipeline::history))
        .route("/history/:id", get(pipeline::history_detail))
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:8000";
    info!("📡 SignalHive by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn auth_status() -> Json<serde_json::Value> {
    Json(json!({"auth_enabled": auth_enabled()}))
}

#[derive(serde::Deserialize)]
struct LoginBody {
    api_key: String,
}

async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !verify_token(&body.api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(json!({"ok": true, "auth_enabled": true})))
}

async fn gen_key() -> Result<Json<serde_json::Value>, StatusCode> {
    if auth_enabled() {
        return Err(StatusCode::FORBIDDEN);
    }
    let key = generate_and_save_key();
    Ok(Json(json!({"api_key": key, "message": "Store this — it won't be shown again"})))
}

async fn health(State(_state): State<AppState>) -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| checks.iter().filter(|check| check["level"] == "error").count())
        .unwrap_or(0);

    Json(json!({
        "status": if errors > 0 { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "SignalHive by PatchHive",
        "scan_count": db::scan_count(),
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_path": db::db_path(),
        "read_only": true,
    }))
}

async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

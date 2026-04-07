mod ai_local;
mod agents;
mod auth;
mod db;
mod fix_worker;
mod github;
mod git_ops;
mod pipeline;
mod routes;
mod startup;
mod state;

use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use once_cell::sync::OnceCell;

use crate::auth::{auth_enabled, generate_and_save_key, verify_token};
use crate::state::AppState;

static STARTUP_CHECKS: OnceCell<Vec<serde_json::Value>> = OnceCell::new();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let _ = dotenvy::dotenv();

    if let Err(e) = db::init_db() {
        eprintln!("DB init failed: {e}");
        std::process::exit(1);
    }

    let orphans = db::recover_orphaned_runs();
    if !orphans.is_empty() {
        tracing::warn!("Recovered {} orphaned run(s): {:?}", orphans.len(), orphans);
    }

    let state = AppState::new();

    if db::get_setting("watch_mode", "false") == "true" {
        state.watch_mode.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    let checks = startup::validate_config(&state.http).await;
    for check in &checks {
        match check["level"].as_str() {
            Some("error") => tracing::error!("Config: {}", check["msg"].as_str().unwrap_or("")),
            Some("warn")  => tracing::warn!("Config: {}",  check["msg"].as_str().unwrap_or("")),
            _             => info!("Config: {}",            check["msg"].as_str().unwrap_or("")),
        }
    }
    let _ = STARTUP_CHECKS.set(checks);

    let http_bg = state.http.clone();
    let state_sched = state.clone();
    tokio::spawn(startup::pr_poll_loop(http_bg));
    tokio::spawn(routes::webhook::scheduler_loop(state_sched));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/auth/status",       get(auth_status))
        .route("/auth/login",        post(login))
        .route("/auth/generate-key", post(gen_key))
        .route("/health",            get(health))
        .route("/startup/checks",    get(startup_checks_route))
        .route("/run",               post(pipeline::run))
        .route("/dry-run",           post(pipeline::dry_run))
        .merge(routes::config::router())
        .merge(routes::history::router())
        .merge(routes::webhook::router())
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:8000";
    info!("🔱 RepoReaper by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn auth_status() -> Json<serde_json::Value> {
    Json(json!({"auth_enabled": auth_enabled()}))
}

#[derive(serde::Deserialize)]
struct LoginBody { api_key: String }

async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !verify_token(&body.api_key) { return Err(StatusCode::UNAUTHORIZED); }
    Ok(Json(json!({"ok": true, "auth_enabled": true})))
}

async fn gen_key() -> Result<Json<serde_json::Value>, StatusCode> {
    if auth_enabled() { return Err(StatusCode::FORBIDDEN); }
    let key = generate_and_save_key();
    Ok(Json(json!({"api_key": key, "message": "Store this — it won't be shown again"})))
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let agents_count = state.agents.read().await.len();
    let errors = STARTUP_CHECKS.get()
        .map(|c| c.iter().filter(|v| v["level"] == "error").count())
        .unwrap_or(0);
    Json(json!({
        "status": if errors > 0 { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "RepoReaper by PatchHive",
        "bot": std::env::var("BOT_GITHUB_USER").unwrap_or_else(|_| "(not set)".into()),
        "agents": agents_count,
        "run_active": state.run_active.load(std::sync::atomic::Ordering::SeqCst),
        "watch_mode": state.watch_mode.load(std::sync::atomic::Ordering::SeqCst),
        "lifetime_cost": db::get_lifetime_cost(),
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
    }))
}

async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

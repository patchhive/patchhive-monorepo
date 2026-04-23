mod agents;
mod ai_local;
mod auth;
mod db;
mod fix_worker;
mod git_ops;
mod github;
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
use once_cell::sync::OnceCell;
use patchhive_product_core::contract;
use patchhive_product_core::rate_limit::rate_limit_middleware;
use patchhive_product_core::startup::{
    cors_layer, count_errors, listen_addr, log_checks, StartupCheck,
};
use serde_json::json;
use tracing::info;

use crate::auth::{
    auth_enabled, generate_and_save_key, generate_and_save_service_token,
    service_auth_enabled, service_token_generation_allowed, verify_token,
};
use crate::state::AppState;

static STARTUP_CHECKS: OnceCell<Vec<StartupCheck>> = OnceCell::new();

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
        state
            .watch_mode
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    let checks = startup::validate_config(&state.http).await;
    log_checks(&checks);
    let _ = STARTUP_CHECKS.set(checks);

    let http_bg = state.http.clone();
    let state_sched = state.clone();
    tokio::spawn(startup::pr_poll_loop(http_bg));
    tokio::spawn(routes::webhook::scheduler_loop(state_sched));

    let cors = cors_layer();

    let app = Router::new()
        .route("/auth/status", get(auth_status))
        .route("/auth/login", post(login))
        .route("/auth/generate-key", post(gen_key))
        .route("/auth/generate-service-token", post(gen_service_token))
        .route("/health", get(health))
        .route("/startup/checks", get(startup_checks_route))
        .route("/capabilities", get(capabilities))
        .route("/run", post(pipeline::run))
        .route("/dry-run", post(pipeline::dry_run))
        .merge(routes::config::router())
        .merge(routes::history::router())
        .merge(routes::webhook::router())
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(cors)
        .with_state(state);

    let addr = listen_addr("REAPER_PORT", 8000);
    info!("🔱 RepoReaper by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind RepoReaper to {addr}: {err}"));
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|err| panic!("RepoReaper server failed: {err}"));
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

async fn gen_service_token(
    headers: axum::http::HeaderMap,
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

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let agents_count = state.agents.read().await.len();
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();
    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "RepoReaper by PatchHive",
        "bot": std::env::var("BOT_GITHUB_USER").unwrap_or_else(|_| "(not set)".into()),
        "agents": agents_count,
        "run_active": state.run_active.load(std::sync::atomic::Ordering::SeqCst),
        "watch_mode": state.watch_mode.load(std::sync::atomic::Ordering::SeqCst),
        "lifetime_cost": db::get_lifetime_cost(),
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
    }))
}

async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

async fn capabilities() -> Json<contract::ProductCapabilities> {
    Json(contract::capabilities(
        "repo-reaper",
        "RepoReaper",
        vec![
            contract::action(
                "run",
                "Run autonomous patch hunt",
                "POST",
                "/run",
                "Find candidate issues, generate fixes, validate them, and open pull requests.",
                true,
            ),
            contract::action(
                "dry_run",
                "Run dry stalk",
                "POST",
                "/dry-run",
                "Discover and score candidate work without writing patches or opening pull requests.",
                true,
            ),
        ],
        vec![
            contract::link("history", "History", "/history"),
            contract::link("leaderboard", "Leaderboard", "/leaderboard"),
            contract::link("rejected", "Rejected patches", "/rejected"),
            contract::link("pr_tracking", "PR tracking", "/pr-tracking"),
        ],
    ))
}

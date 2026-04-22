mod auth;
mod db;
mod github;
mod models;
mod pipeline;
mod startup;
mod state;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use once_cell::sync::OnceCell;
use patchhive_product_core::rate_limit::rate_limit_middleware;
use patchhive_product_core::startup::cors_layer;
use patchhive_product_core::startup::{listen_addr, log_checks, StartupCheck};
use tracing::info;

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

    let checks = startup::validate_config().await;
    log_checks(&checks);
    let _ = STARTUP_CHECKS.set(checks);

    let cors = cors_layer();

    let app = Router::new()
        .route("/auth/status", get(pipeline::auth_status))
        .route("/auth/login", post(pipeline::login))
        .route("/auth/generate-key", post(pipeline::gen_key))
        .route("/health", get(pipeline::health))
        .route("/startup/checks", get(pipeline::startup_checks_route))
        .route("/capabilities", get(pipeline::capabilities))
        .route("/runs", get(pipeline::runs))
        .route("/runs/:id", get(pipeline::history_detail))
        .route("/overview", get(pipeline::overview))
        .route("/history", get(pipeline::history))
        .route("/history/:id", get(pipeline::history_detail))
        .route("/review/github/pr", post(pipeline::review_github_pr))
        .route("/webhooks/github", post(pipeline::github_webhook))
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(cors)
        .with_state(AppState::new());

    let addr = listen_addr("REVIEW_BEE_PORT", 8040);
    info!("🐝 ReviewBee by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind ReviewBee to {addr}: {err}"));
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|err| panic!("ReviewBee server failed: {err}"));
}

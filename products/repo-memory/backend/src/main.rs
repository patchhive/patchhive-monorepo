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
use patchhive_product_core::startup::{listen_addr, log_checks, StartupCheck};
use tower_http::cors::{Any, CorsLayer};
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

    let state = AppState::new();
    let checks = startup::validate_config(&state.http).await;
    log_checks(&checks);
    let _ = STARTUP_CHECKS.set(checks);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/auth/status", get(pipeline::auth_status))
        .route("/auth/login", post(pipeline::login))
        .route("/auth/generate-key", post(pipeline::gen_key))
        .route("/health", get(pipeline::health))
        .route("/startup/checks", get(pipeline::startup_checks_route))
        .route("/overview", get(pipeline::overview))
        .route("/repos", get(pipeline::known_repos))
        .route("/memories", get(pipeline::memories))
        .route("/memories/curation", post(pipeline::curate_memory))
        .route("/context", post(pipeline::context))
        .route("/history", get(pipeline::history))
        .route("/history/:id", get(pipeline::history_detail))
        .route("/history/:id/diff", get(pipeline::history_diff))
        .route("/history/:id/prompt-pack", get(pipeline::prompt_pack))
        .route("/ingest", post(pipeline::ingest))
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(cors)
        .with_state(state);

    let addr = listen_addr("REPO_MEMORY_PORT", 8030);
    info!("🧠 RepoMemory by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

mod auth;
mod db;
mod models;
mod pipeline;
mod startup;
mod state;

use axum::{middleware, routing::get, Router};
use patchhive_product_core::rate_limit::rate_limit_middleware;
use patchhive_product_core::startup::{cors_layer, listen_addr, log_checks};
use tracing::info;

use crate::state::AppState;

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
    let checks = startup::validate_config().await;
    log_checks(&checks);
    startup::set_startup_checks(checks);

    let app = Router::new()
        .route("/auth/status", get(pipeline::auth_status))
        .route("/auth/login", axum::routing::post(pipeline::login))
        .route("/auth/generate-key", axum::routing::post(pipeline::gen_key))
        .route("/health", get(pipeline::health))
        .route("/startup/checks", get(pipeline::startup_checks_route))
        .route("/capabilities", get(pipeline::capabilities))
        .route("/runs", get(pipeline::runs))
        .route("/runs/:id", get(pipeline::run_detail))
        .route("/overview", get(pipeline::overview))
        .route("/products", get(pipeline::products))
        .route("/products/:slug/runs", get(pipeline::product_runs))
        .route(
            "/products/:slug/runs/:id",
            get(pipeline::product_run_detail),
        )
        .route("/actions/recent", get(pipeline::recent_actions))
        .route(
            "/products/:slug/actions/:action_id",
            axum::routing::post(pipeline::dispatch_product_action),
        )
        .route(
            "/settings",
            get(pipeline::settings).put(pipeline::save_settings),
        )
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(cors_layer())
        .with_state(state);

    let addr = listen_addr("HIVE_CORE_PORT", 8100);
    info!("⬢ HiveCore by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind HiveCore to {addr}: {err}"));
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|err| panic!("HiveCore server failed: {err}"));
}

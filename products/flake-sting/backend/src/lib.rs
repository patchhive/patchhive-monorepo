patchhive_product_core::define_api_key_auth_module! {
    pub mod auth {
        patchhive_product_core::auth::ApiKeyAuthConfig::new("FLAKE_STING_API_KEY_HASH", "flake-sting-")
            .with_service_token("FLAKE_STING_SERVICE_TOKEN_HASH", "flake-sting-svc-")
            .with_service_default_name("hivecore")
            .with_service_dispatch_paths([
                "/scan/github/actions",
                "/api/products/flake-sting/scan/github/actions",
            ])
            .with_unauthorized_message("Unauthorized — provide X-API-Key or X-PatchHive-Service-Token.")
            .with_public_paths([
                "/health",
                "/auth/login",
                "/auth/status",
                "/auth/generate-key",
                "/auth/generate-service-token",
                "/auth/rotate-service-token",
                "/startup/checks",
                "/capabilities",
                "/api/products/flake-sting/health",
                "/api/products/flake-sting/auth/login",
                "/api/products/flake-sting/auth/status",
                "/api/products/flake-sting/auth/generate-key",
                "/api/products/flake-sting/auth/generate-service-token",
                "/api/products/flake-sting/auth/rotate-service-token",
                "/api/products/flake-sting/startup/checks",
                "/api/products/flake-sting/capabilities",
            ])
    }
}

pub mod db;
pub mod github;
pub mod models;
pub mod pipeline;
pub mod startup;
pub mod state;

use anyhow::Result;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use once_cell::sync::OnceCell;
use patchhive_product_core::rate_limit::rate_limit_middleware;
use patchhive_product_core::startup::{log_checks, StartupCheck};

pub static STARTUP_CHECKS: OnceCell<Vec<StartupCheck>> = OnceCell::new();

pub async fn init_runtime() -> Result<()> {
    db::init_db()?;
    let checks = startup::validate_config(&reqwest::Client::new()).await;
    log_checks(&checks);
    let _ = STARTUP_CHECKS.set(checks);
    Ok(())
}

pub fn router() -> Router {
    Router::new()
        .route("/auth/status", get(pipeline::auth_status))
        .route("/auth/login", post(pipeline::login))
        .route("/auth/generate-key", post(pipeline::gen_key))
        .route(
            "/auth/generate-service-token",
            post(pipeline::gen_service_token),
        )
        .route(
            "/auth/rotate-service-token",
            post(pipeline::rotate_service_token),
        )
        .route("/health", get(pipeline::health))
        .route("/startup/checks", get(pipeline::startup_checks_route))
        .route("/capabilities", get(pipeline::capabilities))
        .route("/runs", get(pipeline::runs))
        .route("/runs/:id", get(pipeline::history_detail))
        .route("/overview", get(pipeline::overview))
        .route("/history", get(pipeline::history))
        .route("/history/:id", get(pipeline::history_detail))
        .route("/scan/github/actions", post(pipeline::scan_github_actions))
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .with_state(state::AppState::new())
}

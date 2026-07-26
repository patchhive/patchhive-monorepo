// Suite-layer operator auth, generated from the same builder every product uses so
// there is one auth model in the codebase rather than two. Product routers keep
// their own guards; this protects the suite API in front of them.
patchhive_product_core::define_api_key_auth_module! {
    pub mod auth {
        patchhive_product_core::auth::ApiKeyAuthConfig::new(
            "PATCHHIVE_SUITE_API_KEY_HASH",
            "ph-suite-",
        )
        .with_unauthorized_message(
            "Unauthorized — provide X-API-Key. Generate the first key from localhost via POST /api/auth/generate-key.",
        )
        .with_public_paths(crate::suite_public_paths())
    }
}

/// Paths served without an operator key.
///
/// Suite routes only — product routes are not gated by this layer at all, so they
/// need no entries here. Health and the auth bootstrap trio stay open so an
/// unconfigured suite can be inspected and bootstrapped.
fn suite_public_paths() -> Vec<String> {
    [
        "/",
        "/health",
        "/api/health",
        "/api/auth/status",
        "/api/auth/login",
        "/api/auth/generate-key",
    ]
    .iter()
    .map(|path| path.to_string())
    .collect()
}

mod config;
mod db;
mod gateway;
mod models;
mod products;
mod registry;
mod routes;
mod state;

use anyhow::Result;
use axum::Router;
use config::Config;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = Config::from_env()?;
    let bind_addr = config.bind_addr;
    products::init_enabled_products(&config).await?;
    let state = Arc::new(AppState::new(config)?);

    // Auth is applied inside routes::router, to the suite routes only.
    let app = Router::new()
        .merge(routes::router(state))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    info!(%bind_addr, "patchhive-backend listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("patchhive_backend=info,tower_http=info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

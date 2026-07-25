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
        .with_public_paths([
            "/",
            "/health",
            "/api/health",
            "/api/auth/status",
            "/api/auth/login",
            "/api/auth/generate-key",
        ])
    }
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

    let app = Router::new()
        .merge(routes::router(state))
        // Auth wraps the suite API. Product routers are nested inside and keep their
        // own guards, so a request crossing into a product is checked twice — once
        // for the suite operator, once for the product's own credential rules.
        .layer(axum::middleware::from_fn(auth::auth_middleware))
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

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
mod models;
include!(concat!(env!("OUT_DIR"), "/product_inventory.rs"));
mod products;
mod registry;
mod routes;
mod state;

use anyhow::Result;
use axum::Router;
use config::Config;
use std::sync::Arc;
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
        .layer(patchhive_product_core::startup::cors_layer());

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
    // The default used to be `patchhive_backend=info`, which enabled this crate and
    // nothing else. Every product engine compiled in-process — and the shared crates
    // underneath them — had its logs discarded, so startup diagnostics, migration
    // reports and repository-policy conflict warnings were emitted into a filter that
    // dropped them. A warning nobody can see is worse than no warning: it reads as
    // "nothing happened".
    //
    // `info` is the floor for everything, with the noisy dependencies that motivated
    // a narrow filter in the first place named explicitly. RUST_LOG still overrides.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,hyper=warn,rustls=warn,h2=warn,reqwest=warn,sqlx=warn")
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

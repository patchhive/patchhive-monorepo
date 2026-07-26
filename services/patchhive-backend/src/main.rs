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
/// Beyond the suite's own health and bootstrap routes, this includes each product's
/// genuinely public contract endpoints. Products already declare `/health`,
/// `/capabilities`, and `/startup/checks` public in their own auth config, and the
/// suite layer must not be stricter than the engine it fronts — HiveCore polls those
/// endpoints in-process and has no suite key to present.
///
/// Everything with real data behind it — runs, auth posture, settings, dispatch,
/// the aggregates — stays protected.
fn suite_public_paths() -> Vec<String> {
    const PRODUCT_KEYS: [&str; 12] = [
        "hive-core",
        "signal-hive",
        "review-bee",
        "trust-gate",
        "repo-memory",
        "merge-keeper",
        "flake-sting",
        "dep-triage",
        "vuln-triage",
        "refactor-scout",
        "release-sentry",
        "repo-reaper",
    ];

    let mut paths: Vec<String> = [
        "/",
        "/health",
        "/api/health",
        "/api/auth/status",
        "/api/auth/login",
        "/api/auth/generate-key",
    ]
    .iter()
    .map(|path| path.to_string())
    .collect();

    for key in PRODUCT_KEYS {
        for suffix in ["health", "capabilities", "startup/checks", "auth/status"] {
            paths.push(format!("/api/products/{key}/{suffix}"));
        }
    }

    paths
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

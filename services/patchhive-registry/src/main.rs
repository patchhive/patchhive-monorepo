mod config;
mod db;
mod models;
mod routes;
mod state;

use anyhow::Result;
use axum::{middleware, Router};
use config::Config;
use patchhive_product_core::rate_limit::rate_limit_middleware;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    patchhive_product_core::environment::load_patchhive_env()?;
    init_tracing();

    let config = Config::from_env()?;
    let bind_addr = config.bind_addr;
    let state = Arc::new(AppState::new(config)?);

    let app = Router::new()
        .merge(routes::router())
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    info!(%bind_addr, "patchhive-registry listening");

    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("patchhive_registry=info,tower_http=info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

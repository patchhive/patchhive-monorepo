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

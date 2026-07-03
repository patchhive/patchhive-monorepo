use anyhow::Result;
use axum::Router;
use patchhive_product_core::startup::{cors_layer, listen_addr};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let _ = dotenvy::dotenv();
    release_sentry::init_runtime().await?;

    let app = Router::new()
        .merge(release_sentry::router())
        .layer(cors_layer());

    let addr = listen_addr("RELEASE_SENTRY_PORT", 8120);
    info!("🚦 ReleaseSentry by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

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
    signal_hive::init_runtime().await?;

    let app = Router::new()
        .merge(signal_hive::router())
        .layer(cors_layer());

    let addr = listen_addr("SIGNAL_PORT", 8010);
    info!("📡 SignalHive by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}

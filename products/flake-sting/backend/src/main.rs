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

    flake_sting::init_runtime().await?;

    let app = Router::new()
        .merge(flake_sting::router())
        .layer(cors_layer());

    let addr = listen_addr("FLAKE_STING_PORT", 8060);
    info!("🦂 FlakeSting by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind FlakeSting to {addr}: {err}"));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap_or_else(|err| panic!("FlakeSting server failed: {err}"));
    Ok(())
}

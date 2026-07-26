use anyhow::Result;
use axum::Router;
use patchhive_product_core::startup::{cors_layer, listen_addr};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    __PRODUCT_CRATE__::init_runtime().await?;
    let app = Router::new()
        .merge(__PRODUCT_CRATE__::router())
        .layer(cors_layer());
    let addr = listen_addr("__ENV_PREFIX___PORT", __BACKEND_PORT__);
    info!("__PRODUCT_ICON__ __PRODUCT_TITLE__ by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}

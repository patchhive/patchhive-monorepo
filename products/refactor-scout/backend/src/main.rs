use anyhow::Result;
use axum::Router;
use patchhive_product_core::startup::{cors_layer, listen_addr};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    patchhive_product_core::environment::load_patchhive_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    refactor_scout::init_runtime().await?;

    let app = Router::new()
        .merge(refactor_scout::router())
        .layer(cors_layer());

    let addr = listen_addr("REFACTOR_SCOUT_PORT", 8090);
    info!("🧭 RefactorScout by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}

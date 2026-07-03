use anyhow::Result;

use crate::config::Config;

pub async fn init_enabled_products(config: &Config) -> Result<()> {
    if config.product_selection.enables("merge-keeper") {
        merge_keeper::init_runtime().await?;
    }
    Ok(())
}

pub fn merge_keeper_router() -> axum::Router {
    merge_keeper::router()
}

use anyhow::Result;

use crate::{config::Config, db::RegistryStore};

pub struct AppState {
    pub store: RegistryStore,
    pub registration_key: Option<String>,
    pub opt_out_sync_key: Option<String>,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new(config: Config) -> Result<Self> {
        let store = RegistryStore::new(config.db_path.clone());
        store.init()?;
        Ok(Self {
            store,
            registration_key: config.registration_key,
            opt_out_sync_key: config.opt_out_sync_key,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()?,
        })
    }
}

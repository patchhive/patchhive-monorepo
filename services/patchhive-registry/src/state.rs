use anyhow::Result;

use crate::{config::Config, db::RegistryStore};

pub struct AppState {
    pub store: RegistryStore,
}

impl AppState {
    pub fn new(config: Config) -> Result<Self> {
        let store = RegistryStore::new(config.db_path.clone());
        store.init()?;
        Ok(Self { store })
    }
}

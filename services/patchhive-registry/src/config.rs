use anyhow::{Context, Result};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub db_path: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_addr = std::env::var("PATCHHIVE_REGISTRY_BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8130".to_string())
            .parse::<SocketAddr>()
            .context("PATCHHIVE_REGISTRY_BIND_ADDR must be a socket address")?;

        let db_path = std::env::var("PATCHHIVE_REGISTRY_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("patchhive-registry.db"));

        Ok(Self { bind_addr, db_path })
    }
}

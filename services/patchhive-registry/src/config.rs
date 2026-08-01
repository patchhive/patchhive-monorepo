use anyhow::{Context, Result};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub db_path: PathBuf,
    pub registration_key: Option<String>,
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

        let registration_key = std::env::var("PATCHHIVE_REGISTRY_REGISTRATION_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if !bind_addr.ip().is_loopback() && registration_key.is_none() {
            anyhow::bail!(
                "PATCHHIVE_REGISTRY_REGISTRATION_KEY is required for a non-loopback registry"
            );
        }

        Ok(Self {
            bind_addr,
            db_path,
            registration_key,
        })
    }
}

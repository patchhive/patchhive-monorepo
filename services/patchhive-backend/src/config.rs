use std::{collections::HashSet, env, net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub db_path: PathBuf,
    pub product_selection: ProductSelection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductSelection {
    All,
    Only(HashSet<String>),
}

impl Config {
    pub fn from_env() -> Result<Self> {
        patchhive_product_core::environment::load_patchhive_env()?;

        let bind_addr = env::var("PATCHHIVE_BIND_ADDR")
            .unwrap_or_else(|_| {
                let port = env::var("PATCHHIVE_PORT").unwrap_or_else(|_| "8100".to_string());
                format!("127.0.0.1:{port}")
            })
            .parse::<SocketAddr>()
            .context("PATCHHIVE_BIND_ADDR must be a socket address like 127.0.0.1:8100")?;

        let product_selection = ProductSelection::from_env_value(
            env::var("PATCHHIVE_PRODUCTS").unwrap_or_else(|_| "all".to_string()),
        );
        let db_path = env::var("PATCHHIVE_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("patchhive-backend.db"));

        Ok(Self {
            bind_addr,
            db_path,
            product_selection,
        })
    }
}

impl ProductSelection {
    pub fn from_env_value(value: impl AsRef<str>) -> Self {
        let raw = value.as_ref().trim();
        if raw.is_empty() || raw.eq_ignore_ascii_case("all") || raw == "*" {
            return Self::All;
        }

        let keys = raw
            .split(',')
            .map(|part| part.trim().to_ascii_lowercase())
            .filter(|part| !part.is_empty())
            .collect::<HashSet<_>>();

        if keys.is_empty() {
            Self::All
        } else {
            Self::Only(keys)
        }
    }

    pub fn enables(&self, key: &str) -> bool {
        match self {
            Self::All => true,
            Self::Only(keys) => keys.contains(key),
        }
    }

    pub fn mode_label(&self) -> &'static str {
        match self {
            Self::All => "suite",
            Self::Only(keys) if keys.len() == 1 => "product",
            Self::Only(_) => "partial-suite",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProductSelection;

    #[test]
    fn all_selection_accepts_all_and_star() {
        assert_eq!(
            ProductSelection::from_env_value("all"),
            ProductSelection::All
        );
        assert_eq!(ProductSelection::from_env_value("*"), ProductSelection::All);
        assert_eq!(ProductSelection::from_env_value(""), ProductSelection::All);
    }

    #[test]
    fn product_selection_normalizes_keys() {
        let selection = ProductSelection::from_env_value(" Signal-Hive, trust-gate ");

        assert!(selection.enables("signal-hive"));
        assert!(selection.enables("trust-gate"));
        assert!(!selection.enables("repo-reaper"));
        assert_eq!(selection.mode_label(), "partial-suite");
    }
}

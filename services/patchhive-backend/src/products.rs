use std::collections::HashMap;

use anyhow::{Context, Result};
use patchhive_product_core::peer_service::{
    configure_peer_service, PeerProduct, PeerServiceAuth, PeerServiceConfiguration,
};

use crate::config::Config;

pub async fn init_enabled_products(config: &Config) -> Result<()> {
    let suite_base_url = suite_base_url(config);
    let in_process_product_auth = configure_in_process_peers(config, &suite_base_url)?;

    macro_rules! init_one {
        (hive_core, $key:literal) => {
            if config.product_selection.enables($key) {
                hive_core::init_runtime_with_configuration(hive_core::RuntimeConfiguration {
                    topology: patchhive_product_core::hivecore_kernel::DeploymentTopology::UnifiedInProcess,
                    suite_base_url: Some(suite_base_url.clone()),
                    in_process_product_auth: in_process_product_auth.clone(),
                })
                .await?;
            }
        };
        ($module:ident, $key:literal) => {
            if config.product_selection.enables($key) {
                $module::init_runtime().await?;
            }
        };
    }
    macro_rules! init_all {
        ($(($module:ident, $key:literal)),* $(,)?) => {
            $(init_one!($module, $key);)*
        };
    }
    for_each_product!(init_all);
    Ok(())
}

fn configure_in_process_peers(
    config: &Config,
    suite_base_url: &str,
) -> Result<HashMap<String, PeerServiceAuth>> {
    let suite_base_url = suite_base_url.trim_end_matches('/');
    let mut credentials = HashMap::new();

    macro_rules! configure_one {
        ($module:ident, $key:literal) => {
            if config.product_selection.enables($key) {
                let token = $module::auth::configure_in_process_service_token()
                    .with_context(|| format!("could not configure runtime auth for {}", $key))?;
                credentials.insert($key.to_string(), PeerServiceAuth::service_token(token)?);
            }
        };
    }
    macro_rules! configure_all {
        ($(($module:ident, $key:literal)),* $(,)?) => {
            $(configure_one!($module, $key);)*
        };
    }
    for_each_product!(configure_all);

    for (product, slug) in [
        (PeerProduct::HiveCore, "hive-core"),
        (PeerProduct::RepoMemory, "repo-memory"),
        (PeerProduct::ReviewBee, "review-bee"),
        (PeerProduct::TrustGate, "trust-gate"),
    ] {
        if let Some(auth) = credentials.get(slug).cloned() {
            configure_peer(product, suite_base_url, slug, auth)?;
        }
    }

    Ok(credentials)
}

fn configure_peer(
    product: PeerProduct,
    suite_base_url: &str,
    slug: &str,
    auth: PeerServiceAuth,
) -> Result<()> {
    configure_peer_service(
        product,
        PeerServiceConfiguration::new(format!("{suite_base_url}/api/products/{slug}"), auth)?,
    )
}

/// The address other in-process engines should use to reach this runtime.
///
/// A wildcard bind is not dialable, so it resolves to loopback on the same port.
fn suite_base_url(config: &Config) -> String {
    if let Ok(explicit) = std::env::var("PATCHHIVE_SUITE_BASE_URL") {
        if !explicit.trim().is_empty() {
            return explicit.trim().to_string();
        }
    }
    let addr = config.bind_addr;
    let host = if addr.ip().is_unspecified() {
        "127.0.0.1".to_string()
    } else {
        addr.ip().to_string()
    };
    format!("http://{host}:{}", addr.port())
}

/// Run summaries for one product engine, contract-shaped.
#[derive(serde::Serialize)]
pub struct ProductRuns {
    pub key: String,
    pub runs:
        hive_core::models::Observation<Vec<patchhive_product_core::contract::ProductRunSummary>>,
}

/// Suite-wide run index from HiveCore's durable background snapshots.
///
/// The snapshot worker observed each product through its mounted HTTP router, so
/// this read performs no request-time fleet fan-out and bypasses no middleware.
pub fn materialized_runs(
    config: &Config,
    keys: impl IntoIterator<Item = String>,
) -> Vec<ProductRuns> {
    keys.into_iter()
        .map(|key| ProductRuns {
            runs: if config.product_selection.enables(&key) {
                hive_core::materialized_product_runs(&key)
            } else {
                hive_core::models::Observation::not_applicable(
                    "Product is disabled by PATCHHIVE_PRODUCTS.",
                )
            },
            key,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, path::PathBuf};

    use super::*;
    use crate::config::ProductSelection;
    use patchhive_product_core::peer_service::{peer_service, PeerServiceAuth};

    #[test]
    fn unified_peer_credentials_are_accepted_by_the_target_auth_modules() {
        let config = Config {
            bind_addr: "127.0.0.1:18110"
                .parse::<SocketAddr>()
                .expect("valid bind address"),
            db_path: PathBuf::from("unused-peer-wiring-test.db"),
            product_selection: ProductSelection::All,
        };

        let credentials = configure_in_process_peers(&config, "http://127.0.0.1:18110")
            .expect("peer wiring should configure");

        assert!(!credentials.is_empty());

        macro_rules! verify_one {
            ($module:ident, $key:literal) => {
                let PeerServiceAuth::ServiceToken(token) = credentials
                    .get($key)
                    .expect("runtime credential should exist")
                else {
                    panic!("unified peers must use scoped service tokens");
                };
                assert!(
                    $module::auth::verify_service_token(token),
                    "{} should accept its runtime credential",
                    $key
                );
            };
        }
        macro_rules! verify_all {
            ($(($module:ident, $key:literal)),* $(,)?) => {
                $(verify_one!($module, $key);)*
            };
        }
        for_each_product!(verify_all);

        for (product, verify) in [
            (
                PeerProduct::HiveCore,
                hive_core::auth::verify_service_token as fn(&str) -> bool,
            ),
            (
                PeerProduct::RepoMemory,
                repo_memory::auth::verify_service_token as fn(&str) -> bool,
            ),
            (
                PeerProduct::ReviewBee,
                review_bee::auth::verify_service_token as fn(&str) -> bool,
            ),
            (
                PeerProduct::TrustGate,
                trust_gate::auth::verify_service_token as fn(&str) -> bool,
            ),
        ] {
            let configured = peer_service(product).expect("peer should be configured");
            let PeerServiceAuth::ServiceToken(token) = configured.auth else {
                panic!("unified peers must use scoped service tokens");
            };
            assert!(verify(&token));
        }
    }
}

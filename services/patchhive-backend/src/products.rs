use anyhow::Result;

use crate::config::Config;

pub async fn init_enabled_products(config: &Config) -> Result<()> {
    let suite_base_url = suite_base_url(config);
    patchhive_product_core::trust_gate::configure_in_process_trust_gate_url(format!(
        "{}/api/products/trust-gate",
        suite_base_url.trim_end_matches('/')
    ))?;

    macro_rules! init_one {
        (hive_core, $key:literal) => {
            if config.product_selection.enables($key) {
                hive_core::init_runtime_with_configuration(hive_core::RuntimeConfiguration {
                    topology: patchhive_product_core::hivecore_kernel::DeploymentTopology::UnifiedInProcess,
                    suite_base_url: Some(suite_base_url.clone()),
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

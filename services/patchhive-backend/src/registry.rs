use std::env;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::models::{
    AuthStatusResponse, CapabilityMetadata, GatewayStatus, MigrationStage, ProductHealthResponse,
    ProductResponse, ProductStatus, RouteClaim, RuntimeProduct, SafetyBoundary, SetupProduct,
};

const MANIFEST_SOURCES: &[&str] = &[
    include_str!("../registry/products/hive-core.toml"),
    include_str!("../registry/products/signal-hive.toml"),
    include_str!("../registry/products/review-bee.toml"),
    include_str!("../registry/products/trust-gate.toml"),
    include_str!("../registry/products/repo-memory.toml"),
    include_str!("../registry/products/merge-keeper.toml"),
    include_str!("../registry/products/flake-sting.toml"),
    include_str!("../registry/products/dep-triage.toml"),
    include_str!("../registry/products/vuln-triage.toml"),
    include_str!("../registry/products/refactor-scout.toml"),
    include_str!("../registry/products/release-sentry.toml"),
    include_str!("../registry/products/repo-reaper.toml"),
];

#[derive(Clone, Debug)]
pub struct ProductRegistry {
    products: Vec<ProductManifest>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProductManifest {
    pub key: String,
    pub code: String,
    pub name: String,
    pub role: String,
    pub module_path: String,
    #[serde(default = "default_route_prefix")]
    pub route_prefix: String,
    #[serde(default)]
    pub migration_stage: Option<MigrationStage>,
    #[serde(default)]
    pub capabilities: Vec<CapabilityMetadata>,
    #[serde(default)]
    pub safety: SafetyBoundary,
    #[serde(default)]
    pub gateway: Option<GatewayConfig>,
    #[serde(default)]
    pub routes: Vec<RouteClaim>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct GatewayConfig {
    pub default_url: Option<String>,
    pub env_var: Option<String>,
}

fn default_route_prefix() -> String {
    String::new()
}

impl ProductRegistry {
    pub fn load() -> Result<Self> {
        let mut products = Vec::with_capacity(MANIFEST_SOURCES.len());
        for source in MANIFEST_SOURCES {
            let mut product = toml::from_str::<ProductManifest>(source)
                .context("could not parse product manifest")?;
            if product.route_prefix.is_empty() {
                product.route_prefix = format!("/api/products/{}", product.key);
            }
            products.push(product);
        }
        products.sort_by_key(|product| product.sort_index());
        Ok(Self { products })
    }

    pub fn products(&self) -> &[ProductManifest] {
        &self.products
    }

    pub fn find(&self, key: &str) -> Option<&ProductManifest> {
        self.products.iter().find(|product| product.key == key)
    }
}

impl ProductManifest {
    pub fn gateway_target_url(&self) -> Option<String> {
        self.gateway.as_ref().and_then(GatewayConfig::target_url)
    }

    pub fn route_claim_for(&self, method: &str, path: &str) -> Option<&RouteClaim> {
        self.routes.iter().find(|route| {
            route.method.eq_ignore_ascii_case(method) && route_path_matches(&route.path, path)
        })
    }

    pub fn to_response(&self, enabled: bool) -> ProductResponse {
        let gateway = self.gateway_status();
        ProductResponse {
            key: self.key.clone(),
            slug: self.key.clone(),
            name: self.name.clone(),
            title: self.name.clone(),
            code: self.code.clone(),
            role: self.role.clone(),
            module_path: self.module_path.clone(),
            enabled,
            status: product_status(enabled, gateway.configured),
            migration_stage: self.migration_stage(),
            route_prefix: self.route_prefix.clone(),
            capabilities: self
                .capabilities
                .iter()
                .map(|capability| capability.id.clone())
                .collect(),
            capability_metadata: self.capabilities.clone(),
            safety: self.safety.clone(),
            gateway,
            routes: self.routes.clone(),
        }
    }

    pub fn to_setup_product(&self, enabled: bool) -> SetupProduct {
        SetupProduct {
            runtime: RuntimeProduct {
                slug: self.key.clone(),
                icon: self.code.clone(),
                title: self.name.clone(),
                role: self.role.clone(),
                status: runtime_status(&self.key, enabled, self.gateway_target_url().is_some()),
                api_url: self.route_prefix.clone(),
                enabled,
                service_token_configured: false,
                legacy_api_key_configured: false,
                contract_drift_count: contract_drift_count(
                    &self.key,
                    enabled,
                    self.gateway_target_url().is_some(),
                ),
            },
            pairing_ready: false,
            auth_status: AuthStatusResponse {
                auth_enabled: false,
                bootstrap_required: false,
                service_auth_enabled: false,
                suite_bootstrap_enabled: false,
            },
            auth_status_error: None,
        }
    }

    pub fn to_health_response(&self, enabled: bool) -> ProductHealthResponse {
        ProductHealthResponse {
            key: self.key.clone(),
            name: self.name.clone(),
            enabled,
            status: product_status(enabled, self.gateway_target_url().is_some()),
            migration_stage: self.migration_stage(),
            message: if enabled {
                format!(
                    "{} is enabled in patchhive-backend, but its engine has not been migrated yet.",
                    self.name
                )
            } else {
                "Product is disabled by PATCHHIVE_PRODUCTS.".to_string()
            },
        }
    }

    fn migration_stage(&self) -> MigrationStage {
        self.migration_stage
            .clone()
            .unwrap_or(MigrationStage::NotStarted)
    }

    fn sort_index(&self) -> usize {
        PRODUCT_ORDER
            .iter()
            .position(|key| *key == self.key.as_str())
            .unwrap_or(usize::MAX)
    }

    fn gateway_status(&self) -> GatewayStatus {
        let target_url = self.gateway_target_url();
        GatewayStatus {
            configured: target_url.is_some(),
            target_url,
            env_var: self
                .gateway
                .as_ref()
                .and_then(|gateway| gateway.env_var.clone()),
        }
    }
}

impl GatewayConfig {
    pub fn target_url(&self) -> Option<String> {
        self.env_var
            .as_ref()
            .and_then(|key| env::var(key).ok())
            .filter(|value| !value.trim().is_empty())
            .or_else(|| self.default_url.clone())
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
    }
}

pub fn product_status(enabled: bool, gateway_configured: bool) -> ProductStatus {
    if !enabled {
        ProductStatus::Disabled
    } else if gateway_configured {
        ProductStatus::GatewayPending
    } else {
        ProductStatus::EnginePending
    }
}

pub fn runtime_status(key: &str, enabled: bool, gateway_configured: bool) -> &'static str {
    if !enabled {
        "disabled"
    } else if key == "hive-core" {
        "online"
    } else if gateway_configured {
        "gateway-pending"
    } else {
        "degraded"
    }
}

pub fn contract_drift_count(key: &str, enabled: bool, gateway_configured: bool) -> usize {
    if enabled && key != "hive-core" && !gateway_configured {
        1
    } else {
        0
    }
}

const PRODUCT_ORDER: &[&str] = &[
    "hive-core",
    "signal-hive",
    "review-bee",
    "trust-gate",
    "repo-memory",
    "merge-keeper",
    "flake-sting",
    "dep-triage",
    "vuln-triage",
    "refactor-scout",
    "release-sentry",
    "repo-reaper",
];

fn route_path_matches(pattern: &str, path: &str) -> bool {
    let pattern_parts = pattern.trim_matches('/').split('/').collect::<Vec<_>>();
    let path_parts = path.trim_matches('/').split('/').collect::<Vec<_>>();

    let mut path_index = 0;
    for part in pattern_parts {
        if part.starts_with('*') {
            return true;
        }
        let Some(path_part) = path_parts.get(path_index) else {
            return false;
        };
        if !part.starts_with(':') && part != *path_part {
            return false;
        }
        path_index += 1;
    }

    path_index == path_parts.len()
}

#[cfg(test)]
mod tests {
    use super::ProductRegistry;

    #[test]
    fn manifests_load_with_routes_and_capabilities() {
        let registry = ProductRegistry::load().expect("registry manifests should parse");
        let signal_hive = registry
            .find("signal-hive")
            .expect("SignalHive manifest should exist");

        assert_eq!(registry.products().len(), 12);
        assert!(!signal_hive.capabilities.is_empty());
        assert!(!signal_hive.routes.is_empty());
        assert!(signal_hive.safety.read_only);
        assert!(signal_hive.gateway_target_url().is_some());
        assert!(signal_hive
            .route_claim_for("GET", "/api/products/signal-hive/history/scan-1/timeline")
            .is_some());
        assert!(signal_hive
            .route_claim_for("DELETE", "/api/products/signal-hive/repo-lists/owner/repo")
            .is_some());
        assert!(signal_hive
            .route_claim_for("POST", "/api/products/signal-hive/not-a-real-route")
            .is_none());
        assert_eq!(signal_hive.module_path, "crate::products::signal_hive");
    }
}

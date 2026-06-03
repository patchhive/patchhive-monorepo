use anyhow::{Context, Result};
use serde::Deserialize;

use crate::models::{
    AuthStatusResponse, CapabilityMetadata, MigrationStage, ProductHealthResponse, ProductResponse,
    ProductStatus, RouteClaim, RuntimeProduct, SafetyBoundary, SetupProduct,
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
    pub routes: Vec<RouteClaim>,
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
    pub fn to_response(&self, enabled: bool) -> ProductResponse {
        ProductResponse {
            key: self.key.clone(),
            slug: self.key.clone(),
            name: self.name.clone(),
            title: self.name.clone(),
            code: self.code.clone(),
            role: self.role.clone(),
            module_path: self.module_path.clone(),
            enabled,
            status: product_status(enabled),
            migration_stage: self.migration_stage(),
            route_prefix: self.route_prefix.clone(),
            capabilities: self
                .capabilities
                .iter()
                .map(|capability| capability.id.clone())
                .collect(),
            capability_metadata: self.capabilities.clone(),
            safety: self.safety.clone(),
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
                status: runtime_status(&self.key, enabled),
                api_url: self.route_prefix.clone(),
                enabled,
                service_token_configured: false,
                legacy_api_key_configured: false,
                contract_drift_count: contract_drift_count(&self.key, enabled),
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
            status: product_status(enabled),
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
}

pub fn product_status(enabled: bool) -> ProductStatus {
    if enabled {
        ProductStatus::EnginePending
    } else {
        ProductStatus::Disabled
    }
}

pub fn runtime_status(key: &str, enabled: bool) -> &'static str {
    if !enabled {
        "disabled"
    } else if key == "hive-core" {
        "online"
    } else {
        "degraded"
    }
}

pub fn contract_drift_count(key: &str, enabled: bool) -> usize {
    if enabled && key != "hive-core" {
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
        assert_eq!(signal_hive.module_path, "crate::products::signal_hive");
    }
}

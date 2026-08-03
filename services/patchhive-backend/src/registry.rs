use anyhow::{Context, Result};
use serde::Deserialize;

use crate::models::{
    CapabilityMetadata, ProductHealthContract, ProductResponse, ProductStatus, RouteClaim,
    SafetyBoundary,
};

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
    pub capabilities: Vec<CapabilityMetadata>,
    #[serde(default)]
    pub safety: SafetyBoundary,
    pub health: ProductHealthContract,
    #[serde(default)]
    pub routes: Vec<RouteClaim>,
    pub display: ProductDisplay,
    #[serde(default)]
    pub smoke: patchhive_product_core::smoke_manifest::ProductSmokeManifest,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProductDisplay {
    pub order: usize,
    pub icon: String,
    pub lane: String,
    pub description: String,
    pub repository: String,
    pub frontend_url: String,
    pub api_url: String,
}

fn default_route_prefix() -> String {
    String::new()
}

impl ProductRegistry {
    pub fn load() -> Result<Self> {
        let mut products = Vec::with_capacity(crate::PRODUCT_MANIFEST_SOURCES.len());
        for (index, source) in crate::PRODUCT_MANIFEST_SOURCES.iter().enumerate() {
            let mut product = toml::from_str::<ProductManifest>(source).with_context(|| {
                format!("could not parse product manifest at inventory index {index}")
            })?;
            if product.route_prefix.is_empty() {
                product.route_prefix = format!("/api/products/{}", product.key);
            }
            product.validate()?;
            products.push(product);
        }
        let mut keys = std::collections::HashSet::with_capacity(products.len());
        let mut orders = std::collections::HashSet::with_capacity(products.len());
        for product in &products {
            anyhow::ensure!(
                keys.insert(product.key.as_str()),
                "duplicate product key '{}' in registry",
                product.key
            );
            anyhow::ensure!(
                orders.insert(product.display.order),
                "duplicate product display order {} in registry",
                product.display.order
            );
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
    fn validate(&self) -> Result<()> {
        anyhow::ensure!(!self.key.trim().is_empty(), "product key must not be empty");
        anyhow::ensure!(
            !self.name.trim().is_empty(),
            "product '{}' has no name",
            self.key
        );
        anyhow::ensure!(
            !self.display.icon.trim().is_empty(),
            "product '{}' has no display icon",
            self.key
        );
        anyhow::ensure!(
            !self.display.lane.trim().is_empty(),
            "product '{}' has no display lane",
            self.key
        );
        anyhow::ensure!(
            !self.display.description.trim().is_empty(),
            "product '{}' has no display description",
            self.key
        );
        anyhow::ensure!(
            !self.display.repository.trim().is_empty(),
            "product '{}' has no repository",
            self.key
        );
        anyhow::ensure!(
            !self.display.frontend_url.trim().is_empty(),
            "product '{}' has no frontend URL",
            self.key
        );
        anyhow::ensure!(
            !self.display.api_url.trim().is_empty(),
            "product '{}' has no API URL",
            self.key
        );
        self.smoke.validate(&self.key)?;
        Ok(())
    }

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
            route_prefix: self.route_prefix.clone(),
            capabilities: self
                .capabilities
                .iter()
                .map(|capability| capability.id.clone())
                .collect(),
            capability_metadata: self.capabilities.clone(),
            safety: self.safety.clone(),
            health: self.health.clone(),
            routes: self.routes.clone(),
        }
    }

    fn sort_index(&self) -> usize {
        self.display.order
    }
}

pub fn product_status(enabled: bool) -> ProductStatus {
    if !enabled {
        ProductStatus::Disabled
    } else {
        ProductStatus::Online
    }
}

#[cfg(test)]
mod tests {
    use patchhive_product_core::smoke_manifest::SmokeTier;

    use super::ProductRegistry;

    #[test]
    fn manifests_load_with_routes_and_capabilities() {
        let registry = ProductRegistry::load().expect("registry manifests should parse");
        let signal_hive = registry
            .find("signal-hive")
            .expect("SignalHive manifest should exist");

        assert_eq!(registry.products().len(), 12);
        assert!(registry
            .products()
            .iter()
            .all(|product| !product.health.endpoint.is_empty()));
        assert!(!signal_hive.capabilities.is_empty());
        assert!(!signal_hive.routes.is_empty());
        assert!(signal_hive.safety.read_only);
        assert!(signal_hive.smoke.participates_in(SmokeTier::FirstStack));
        assert!(signal_hive.smoke.participates_in(SmokeTier::ReadOnlyFleet));
        assert_eq!(
            signal_hive
                .smoke
                .action
                .as_ref()
                .expect("SignalHive smoke action should be declared")
                .id,
            "smoke_check"
        );
        assert_eq!(
            signal_hive.health.endpoint,
            "/api/products/signal-hive/health"
        );
        assert_eq!(signal_hive.health.timeout_ms, 2_000);
        assert!(signal_hive
            .routes
            .iter()
            .any(|route| route.path.ends_with("/health")));
        assert_eq!(signal_hive.module_path, "signal_hive");
    }
}

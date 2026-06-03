use chrono::Utc;

use crate::{
    config::Config,
    models::{LauncherStatus, RunSummary, SetupResponse, SuiteEvent},
    registry,
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: Config,
    started_at: chrono::DateTime<Utc>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            started_at: Utc::now(),
        }
    }

    pub fn product_enabled(&self, key: &str) -> bool {
        self.config.product_selection.enables(key)
    }

    pub fn enabled_product_count(&self) -> usize {
        registry::PRODUCTS
            .iter()
            .filter(|product| self.product_enabled(product.key))
            .count()
    }

    pub fn runs(&self) -> Vec<RunSummary> {
        Vec::new()
    }

    pub fn first_stack_status(&self, actions: Vec<String>) -> SetupResponse {
        SetupResponse {
            suite_bootstrap_configured: false,
            launcher: LauncherStatus {
                available: false,
                status: "not-configured",
                message: "Launcher authority still lives in the existing HiveCore backend during this migration step.",
            },
            products: registry::PRODUCTS
                .iter()
                .map(|product| product.to_setup_product(self.product_enabled(product.key)))
                .collect(),
            actions,
        }
    }

    pub fn events(&self) -> Vec<SuiteEvent> {
        vec![SuiteEvent {
            id: "evt-backend-started",
            kind: "backend.started",
            message: "patchhive-backend started",
            created_at: self.started_at,
        }]
    }
}

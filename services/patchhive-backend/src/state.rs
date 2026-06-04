use anyhow::Result;
use chrono::Utc;

use crate::{
    config::Config,
    db::SharedDb,
    models::{LauncherStatus, RunSummary, SetupResponse, SuiteEvent},
    registry::ProductRegistry,
};

#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub registry: ProductRegistry,
    pub http: reqwest::Client,
    db: SharedDb,
    started_at: chrono::DateTime<Utc>,
}

impl AppState {
    pub fn new(config: Config) -> Result<Self> {
        let db = SharedDb::open(&config.db_path)?;
        let registry = ProductRegistry::load()?;
        let started_at = Utc::now();
        let state = Self {
            config,
            registry,
            http: reqwest::Client::new(),
            db,
            started_at,
        };

        state.db.record_event(
            &format!("evt-backend-started-{}", started_at.timestamp_millis()),
            "backend.started",
            "patchhive-backend started",
            started_at,
        );

        Ok(state)
    }

    pub fn product_enabled(&self, key: &str) -> bool {
        self.config.product_selection.enables(key)
    }

    pub fn enabled_product_count(&self) -> usize {
        self.registry
            .products()
            .iter()
            .filter(|product| self.product_enabled(product.key.as_str()))
            .count()
    }

    pub fn db_ok(&self) -> bool {
        self.db.ping()
    }

    pub fn product_override_count(&self) -> usize {
        self.db.product_override_count()
    }

    pub fn runs(&self) -> Vec<RunSummary> {
        self.db.runs()
    }

    pub fn first_stack_status(&self, actions: Vec<String>) -> SetupResponse {
        SetupResponse {
            suite_bootstrap_configured: false,
            launcher: LauncherStatus {
                available: false,
                status: "not-configured",
                message: "Launcher authority still lives in the existing HiveCore backend during this migration step.",
            },
            products: self
                .registry
                .products()
                .iter()
                .map(|product| product.to_setup_product(self.product_enabled(product.key.as_str())))
                .collect(),
            actions,
        }
    }

    pub fn events(&self) -> Vec<SuiteEvent> {
        let events = self.db.events();
        if events.is_empty() {
            vec![SuiteEvent {
                id: "evt-backend-started".to_string(),
                kind: "backend.started".to_string(),
                message: "patchhive-backend started".to_string(),
                created_at: self.started_at,
            }]
        } else {
            events
        }
    }
}

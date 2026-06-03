use crate::models::{
    AuthStatusResponse, MigrationStage, ProductHealthResponse, ProductResponse, ProductStatus,
    RuntimeProduct, SetupProduct,
};

#[derive(Clone, Debug)]
pub struct ProductDefinition {
    pub key: &'static str,
    pub code: &'static str,
    pub name: &'static str,
    pub role: &'static str,
    pub capabilities: &'static [&'static str],
}

pub const PRODUCTS: &[ProductDefinition] = &[
    ProductDefinition {
        key: "hive-core",
        code: "HC",
        name: "HiveCore",
        role: "control plane and suite cockpit",
        capabilities: &["suite-status", "product-registry", "run-index"],
    },
    ProductDefinition {
        key: "signal-hive",
        code: "SH",
        name: "SignalHive",
        role: "maintenance signal reconnaissance",
        capabilities: &["repo-discovery", "signal-scan", "read-only"],
    },
    ProductDefinition {
        key: "review-bee",
        code: "RB",
        name: "ReviewBee",
        role: "PR review feedback checklist",
        capabilities: &["pr-review-read", "checklist"],
    },
    ProductDefinition {
        key: "trust-gate",
        code: "TG",
        name: "TrustGate",
        role: "diff policy and risk review",
        capabilities: &["diff-review", "policy-rules", "risk-decision"],
    },
    ProductDefinition {
        key: "repo-memory",
        code: "RM",
        name: "RepoMemory",
        role: "durable repo memory and prompt packs",
        capabilities: &["memory-ingest", "prompt-pack", "failguard"],
    },
    ProductDefinition {
        key: "merge-keeper",
        code: "MK",
        name: "MergeKeeper",
        role: "merge readiness assessment",
        capabilities: &["pr-readiness", "checks", "review-state"],
    },
    ProductDefinition {
        key: "flake-sting",
        code: "FS",
        name: "FlakeSting",
        role: "CI flake detection",
        capabilities: &["actions-history", "flake-signal", "read-only"],
    },
    ProductDefinition {
        key: "dep-triage",
        code: "DT",
        name: "DepTriage",
        role: "dependency update triage",
        capabilities: &["dependency-prs", "dependabot-alerts", "read-only"],
    },
    ProductDefinition {
        key: "vuln-triage",
        code: "VT",
        name: "VulnTriage",
        role: "security finding triage",
        capabilities: &["code-scanning", "dependabot-alerts", "read-only"],
    },
    ProductDefinition {
        key: "refactor-scout",
        code: "RS",
        name: "RefactorScout",
        role: "conservative refactor discovery",
        capabilities: &["local-scan", "refactor-candidates", "read-only"],
    },
    ProductDefinition {
        key: "release-sentry",
        code: "RSY",
        name: "ReleaseSentry",
        role: "release readiness evidence",
        capabilities: &["release-health", "tag-read", "read-only"],
    },
    ProductDefinition {
        key: "repo-reaper",
        code: "RR",
        name: "RepoReaper",
        role: "autonomous patch and PR execution",
        capabilities: &["issue-hunt", "patch-generation", "pull-request"],
    },
];

impl ProductDefinition {
    pub fn to_response(&self, enabled: bool) -> ProductResponse {
        ProductResponse {
            key: self.key,
            slug: self.key,
            name: self.name,
            title: self.name,
            role: self.role,
            enabled,
            status: product_status(enabled),
            migration_stage: MigrationStage::NotStarted,
            route_prefix: format!("/api/products/{}", self.key),
            capabilities: self.capabilities,
        }
    }

    pub fn to_setup_product(&self, enabled: bool) -> SetupProduct {
        SetupProduct {
            runtime: RuntimeProduct {
                slug: self.key,
                icon: self.code,
                title: self.name,
                role: self.role,
                status: runtime_status(self.key, enabled),
                api_url: format!("/api/products/{}", self.key),
                enabled,
                service_token_configured: false,
                legacy_api_key_configured: false,
                contract_drift_count: contract_drift_count(self.key, enabled),
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
            key: self.key,
            name: self.name,
            enabled,
            status: product_status(enabled),
            migration_stage: MigrationStage::NotStarted,
            message: if enabled {
                "Product is enabled in patchhive-backend, but its engine has not been migrated yet."
            } else {
                "Product is disabled by PATCHHIVE_PRODUCTS."
            },
        }
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

pub fn find_product(key: &str) -> Option<&'static ProductDefinition> {
    PRODUCTS.iter().find(|product| product.key == key)
}

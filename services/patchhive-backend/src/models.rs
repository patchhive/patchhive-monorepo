use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct HealthResponse {
    pub service: &'static str,
    pub status: &'static str,
    pub version: &'static str,
    pub mode: &'static str,
    pub enabled_products: usize,
    pub db_ok: bool,
    pub product_override_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuthStatusResponse {
    pub auth_enabled: bool,
    pub bootstrap_required: bool,
    pub service_auth_enabled: bool,
    pub suite_bootstrap_enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionResponse {
    pub service: &'static str,
    pub authenticated: bool,
    pub auth_configured: bool,
    pub mode: &'static str,
    pub enabled_products: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductResponse {
    pub key: String,
    pub slug: String,
    pub name: String,
    pub title: String,
    pub code: String,
    pub role: String,
    pub module_path: String,
    pub enabled: bool,
    pub status: ProductStatus,
    pub route_prefix: String,
    pub capabilities: Vec<String>,
    pub capability_metadata: Vec<CapabilityMetadata>,
    pub safety: SafetyBoundary,
    pub health: ProductHealthContract,
    pub routes: Vec<RouteClaim>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProductStatus {
    Disabled,
    Online,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RouteClaim {
    pub method: String,
    pub path: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProductHealthContract {
    pub endpoint: String,
    pub timeout_ms: u64,
    pub healthy_status: u16,
}

impl Default for ProductHealthContract {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            timeout_ms: 2_000,
            healthy_status: 200,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CapabilityMetadata {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub mutating: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SafetyBoundary {
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub writes_external_state: bool,
    #[serde(default)]
    pub mutates_repositories: bool,
    #[serde(default)]
    pub opens_pull_requests: bool,
    #[serde(default)]
    pub requires_operator_approval: bool,
    #[serde(default)]
    pub credential_scopes: Vec<String>,
    #[serde(default)]
    pub evidence_required: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunSummary {
    pub id: String,
    pub product_key: String,
    pub status: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SuiteEvent {
    pub id: String,
    pub kind: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ErrorResponse {
    pub error: &'static str,
    pub message: String,
}

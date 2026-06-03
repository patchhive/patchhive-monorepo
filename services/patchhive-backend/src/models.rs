use chrono::{DateTime, Utc};
use serde::Serialize;

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
    pub key: &'static str,
    pub slug: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub role: &'static str,
    pub enabled: bool,
    pub status: ProductStatus,
    pub migration_stage: MigrationStage,
    pub route_prefix: String,
    pub capabilities: &'static [&'static str],
}

#[derive(Clone, Debug, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "kebab-case")]
pub enum ProductStatus {
    Disabled,
    GatewayPending,
    EnginePending,
}

#[derive(Clone, Debug, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationStage {
    NotStarted,
    GatewayReady,
    Integrated,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProductHealthResponse {
    pub key: &'static str,
    pub name: &'static str,
    pub enabled: bool,
    pub status: ProductStatus,
    pub migration_stage: MigrationStage,
    pub message: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct SetupResponse {
    pub suite_bootstrap_configured: bool,
    pub launcher: LauncherStatus,
    pub products: Vec<SetupProduct>,
    pub actions: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LauncherStatus {
    pub available: bool,
    pub status: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct SetupProduct {
    pub runtime: RuntimeProduct,
    pub pairing_ready: bool,
    pub auth_status: AuthStatusResponse,
    pub auth_status_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeProduct {
    pub slug: &'static str,
    pub icon: &'static str,
    pub title: &'static str,
    pub role: &'static str,
    pub status: &'static str,
    pub api_url: String,
    pub enabled: bool,
    pub service_token_configured: bool,
    pub legacy_api_key_configured: bool,
    pub contract_drift_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunSummary {
    pub id: &'static str,
    pub product_key: &'static str,
    pub status: &'static str,
    pub message: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct SuiteEvent {
    pub id: &'static str,
    pub kind: &'static str,
    pub message: &'static str,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ErrorResponse {
    pub error: &'static str,
    pub message: String,
}

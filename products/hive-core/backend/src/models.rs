use chrono::Utc;
use patchhive_product_core::contract;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub const PRODUCT_SLUG: &str = "hive-core";
pub const PRODUCT_TITLE: &str = "HiveCore";
pub const PRODUCT_TAGLINE: &str = "Control the PatchHive suite from one clear surface.";
pub const PRODUCT_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Serialize)]
pub struct ApiMeta {
    pub product: &'static str,
    pub version: &'static str,
    pub request_id: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiEnvelope<T> {
    pub status: &'static str,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub meta: ApiMeta,
}

impl ApiMeta {
    pub fn new() -> Self {
        Self {
            product: PRODUCT_SLUG,
            version: PRODUCT_VERSION,
            request_id: format!("req_{}", Uuid::now_v7()),
            timestamp: now_rfc3339(),
        }
    }
}

pub fn ok<T>(data: T) -> ApiEnvelope<T> {
    ApiEnvelope {
        status: "ok",
        data: Some(data),
        error: None,
        meta: ApiMeta::new(),
    }
}

pub fn error(
    code: impl Into<String>,
    message: impl Into<String>,
    retryable: bool,
) -> ApiEnvelope<Value> {
    ApiEnvelope {
        status: "error",
        data: None,
        error: Some(ApiError {
            code: code.into(),
            message: message.into(),
            retryable,
            details: json!({}),
        }),
        meta: ApiMeta::new(),
    }
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuiteSettings {
    pub operator_label: String,
    pub mission: String,
    pub default_topics: String,
    pub default_languages: String,
    pub repo_allowlist: String,
    pub repo_denylist: String,
    pub opt_out_notes: String,
    pub preferred_launch_product: String,
    pub notes: String,
    pub updated_at: String,
}

impl Default for SuiteSettings {
    fn default() -> Self {
        Self {
            operator_label: "PatchHive operator".into(),
            mission:
                "Visibility first. Trust and memory second. Autonomous action after that foundation is earned."
                    .into(),
            default_topics: "developer tooling, ci reliability, maintenance backlog".into(),
            default_languages: "rust,typescript,python".into(),
            repo_allowlist: String::new(),
            repo_denylist: String::new(),
            opt_out_notes: "Respect project opt-outs and keep autonomous discovery bounded.".into(),
            preferred_launch_product: "signal-hive".into(),
            notes: "HiveCore stores suite defaults here first. Product-level adoption comes next."
                .into(),
            updated_at: now_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductOverride {
    pub slug: String,
    pub frontend_url: String,
    pub api_url: String,
    pub api_key: String,
    pub enabled: bool,
    pub notes: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSettingsItem {
    pub slug: String,
    pub title: String,
    pub icon: String,
    pub lane: String,
    pub role: String,
    pub repo: String,
    pub default_frontend_url: String,
    pub default_api_url: String,
    pub override_frontend_url: String,
    pub override_api_url: String,
    pub api_key_configured: bool,
    pub enabled: bool,
    pub notes: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductHealthSnapshot {
    pub status: String,
    pub reachable: bool,
    pub version: String,
    pub capabilities_ok: bool,
    pub action_count: u32,
    pub config_errors: u32,
    pub startup_errors: u32,
    pub startup_warns: u32,
    pub startup_infos: u32,
    pub db_ok: Option<bool>,
    pub checked_at: String,
    pub error: String,
}

impl Default for ProductHealthSnapshot {
    fn default() -> Self {
        Self {
            status: "unknown".into(),
            reachable: false,
            version: String::new(),
            capabilities_ok: false,
            action_count: 0,
            config_errors: 0,
            startup_errors: 0,
            startup_warns: 0,
            startup_infos: 0,
            db_ok: None,
            checked_at: now_rfc3339(),
            error: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRuntimeItem {
    pub slug: String,
    pub title: String,
    pub icon: String,
    pub lane: String,
    pub role: String,
    pub repo: String,
    pub enabled: bool,
    pub frontend_url: String,
    pub api_url: String,
    pub api_key_configured: bool,
    pub notes: String,
    pub status: String,
    pub health: ProductHealthSnapshot,
    pub actions: Vec<contract::ProductAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OverviewSummary {
    pub total_products: u32,
    pub enabled_products: u32,
    pub online_products: u32,
    pub degraded_products: u32,
    pub offline_products: u32,
    pub disabled_products: u32,
    pub unconfigured_products: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewResponse {
    pub product: &'static str,
    pub tagline: &'static str,
    pub suite_settings: SuiteSettings,
    pub summary: OverviewSummary,
    pub products: Vec<ProductRuntimeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsResponse {
    pub product: &'static str,
    pub tagline: &'static str,
    pub suite_settings: SuiteSettings,
    pub products: Vec<ProductSettingsItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveSettingsRequest {
    pub suite_settings: SuiteSettingsInput,
    pub products: Vec<ProductOverrideInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SuiteSettingsInput {
    pub operator_label: String,
    pub mission: String,
    pub default_topics: String,
    pub default_languages: String,
    pub repo_allowlist: String,
    pub repo_denylist: String,
    pub opt_out_notes: String,
    pub preferred_launch_product: String,
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductOverrideInput {
    pub slug: String,
    pub frontend_url: String,
    pub api_url: String,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductActionEvent {
    pub id: String,
    pub product_slug: String,
    pub action_id: String,
    pub action_label: String,
    pub method: String,
    pub path: String,
    pub target_url: String,
    pub status: String,
    pub remote_status: Option<u16>,
    pub request_json: Value,
    pub response_json: Value,
    pub error: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchActionResponse {
    pub event: ProductActionEvent,
    pub started_run: bool,
}

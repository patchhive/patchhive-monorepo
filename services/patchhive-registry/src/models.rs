use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryMode {
    Disabled,
    Anonymous,
    NamedPrivate,
    PublicDemo,
}

impl Default for RegistryMode {
    fn default() -> Self {
        Self::Anonymous
    }
}

impl RegistryMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Anonymous => "anonymous",
            Self::NamedPrivate => "named-private",
            Self::PublicDemo => "public-demo",
        }
    }

    pub fn is_public(&self) -> bool {
        matches!(self, Self::PublicDemo)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RegisterInstallRequest {
    #[serde(default)]
    pub install_mode: RegistryMode,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub public_slug: Option<String>,
    #[serde(default)]
    pub hivecore_version: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RegisterInstallResponse {
    pub install_id: String,
    pub registry_token: String,
    pub install_mode: RegistryMode,
    pub public_slug: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistrySnapshot {
    pub schema_version: String,
    pub install_mode: RegistryMode,
    pub install_id: String,
    #[serde(default)]
    pub public_slug: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    pub generated_at: String,
    #[serde(default)]
    pub stale_after_seconds: Option<u64>,
    pub hivecore: HiveCoreSnapshot,
    pub fleet: FleetSnapshot,
    #[serde(default)]
    pub products: Vec<ProductSnapshot>,
    pub smoke: SmokeSnapshot,
    #[serde(default)]
    pub privacy: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HiveCoreSnapshot {
    pub version: String,
    pub status: String,
    pub launcher_available: bool,
    pub suite_bootstrap_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FleetSnapshot {
    pub products_total: i64,
    pub products_online: i64,
    pub products_degraded: i64,
    pub products_blocked: i64,
    pub products_paired: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductSnapshot {
    pub slug: String,
    #[serde(default)]
    pub name: Option<String>,
    pub version: String,
    pub status: String,
    #[serde(default)]
    pub capability_ids: Vec<String>,
    #[serde(default)]
    pub contract_version: Option<String>,
    #[serde(default)]
    pub image_tag: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmokeSnapshot {
    pub latest_tier: String,
    pub latest_status: String,
    pub passed: i64,
    pub warned: i64,
    pub failed: i64,
    pub skipped: i64,
    #[serde(default)]
    pub generated_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SmokeUpdateRequest {
    pub smoke: SmokeSnapshot,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicInstallSummary {
    pub install_id: String,
    pub public_slug: String,
    pub display_name: Option<String>,
    pub generated_at: String,
    pub last_heartbeat_at: String,
    pub hivecore_version: String,
    pub hivecore_status: String,
    pub products_total: i64,
    pub products_online: i64,
    pub products_degraded: i64,
    pub products_blocked: i64,
    pub latest_smoke_status: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct HealthResponse {
    pub service: &'static str,
    pub status: &'static str,
    pub version: &'static str,
    pub db_ok: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ErrorResponse {
    pub error: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct OkResponse {
    pub ok: bool,
}

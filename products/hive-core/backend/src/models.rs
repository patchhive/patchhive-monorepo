use chrono::Utc;
pub use patchhive_product_core::approvals::{
    ApprovalConsumptionOutcome, ApprovalEvent, ApprovalExpirableState, ApprovalOrigin,
    ApprovalReasonRequest, ApprovalRecord, ApprovalState, ApprovalSubject,
};
use patchhive_product_core::contract;
pub use patchhive_product_core::hivecore_policy::{
    PrBudgetLimitingLayer, PrBudgetReservation, PrBudgetUsage, PrReservationCommitRequest,
    PrReservationDecision, PrReservationDenial, PrReservationExpiration,
    PrReservationReleaseRequest, PrReservationRequest, PrReservationState, PrRunReleaseRequest,
};
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

impl Default for ApiMeta {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiMeta {
    /// Every envelope gets a fresh request id and timestamp, so this is not a
    /// constant value despite taking no arguments.
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
    pub service_token: String,
    pub legacy_api_key: String,
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
    pub auth_mode: String,
    pub machine_auth_configured: bool,
    pub service_token_configured: bool,
    pub legacy_api_key_configured: bool,
    pub enabled: bool,
    pub notes: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductHealthStatus {
    Online,
    Degraded,
    Offline,
    Disabled,
    Unconfigured,
    Unknown,
}

impl ProductHealthStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Degraded => "degraded",
            Self::Offline => "offline",
            Self::Disabled => "disabled",
            Self::Unconfigured => "unconfigured",
            Self::Unknown => "unknown",
        }
    }

    pub const fn is_reachable(self) -> bool {
        matches!(self, Self::Online | Self::Degraded)
    }
}

impl std::fmt::Display for ProductHealthStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Evidence from an attempted observation.
///
/// This type intentionally has no `Default`. An endpoint that returned an empty
/// collection, an endpoint that failed, an endpoint that was never queried, and an
/// endpoint that does not apply are four different facts for an operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Observation<T> {
    Observed { value: T },
    Failed { reason: String },
    NotObserved { reason: String },
    NotApplicable { reason: String },
}

impl<T> Observation<T> {
    pub fn observed(value: T) -> Self {
        Self::Observed { value }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self::Failed {
            reason: reason.into(),
        }
    }

    pub fn not_observed(reason: impl Into<String>) -> Self {
        Self::NotObserved {
            reason: reason.into(),
        }
    }

    pub fn not_applicable(reason: impl Into<String>) -> Self {
        Self::NotApplicable {
            reason: reason.into(),
        }
    }

    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Observed { value } => Some(value),
            Self::Failed { .. } | Self::NotObserved { .. } | Self::NotApplicable { .. } => None,
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Observed { .. } => None,
            Self::Failed { reason }
            | Self::NotObserved { reason }
            | Self::NotApplicable { reason } => Some(reason),
        }
    }

    pub const fn is_observed(&self) -> bool {
        matches!(self, Self::Observed { .. })
    }

    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthEndpointEvidence {
    pub reported_status: Observation<String>,
    pub latency_ms: u64,
    pub config_errors: Observation<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupChecksEvidence {
    pub errors: u32,
    pub warnings: u32,
    pub infos: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitiesEvidence {
    pub action_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunsEvidence {
    pub run_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductHealthSnapshot {
    pub status: ProductHealthStatus,
    pub health_endpoint: Observation<HealthEndpointEvidence>,
    pub version: Observation<String>,
    pub database_ok: Observation<bool>,
    pub startup_checks: Observation<StartupChecksEvidence>,
    pub capabilities: Observation<CapabilitiesEvidence>,
    pub runs: Observation<RunsEvidence>,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductContractCheck {
    pub id: String,
    pub label: String,
    pub path: String,
    pub ok: bool,
    pub status: String,
    pub error: String,
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
    pub auth_mode: String,
    pub machine_auth_configured: bool,
    pub service_token_configured: bool,
    pub legacy_api_key_configured: bool,
    #[serde(default = "default_product_auth_observation")]
    pub auth_status: Observation<crate::pipeline::ProductAuthStatusBody>,
    pub notes: String,
    pub status: String,
    pub health: ProductHealthSnapshot,
    pub hivecore: Option<contract::HiveCoreLifecycleSupport>,
    pub actions: Vec<contract::ProductAction>,
    pub links: Vec<contract::ProductLink>,
    pub contract_checks: Vec<ProductContractCheck>,
    pub contract_drift_count: u32,
    pub run_detail_template: String,
    pub recent_runs: Vec<contract::ProductRunSummary>,
}

fn default_product_auth_observation() -> Observation<crate::pipeline::ProductAuthStatusBody> {
    Observation::not_observed("The stored snapshot predates product auth observations.")
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
    pub unknown_products: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SuiteSnapshotCycleState {
    Running {
        started_at: String,
    },
    Succeeded {
        started_at: String,
        completed_at: String,
        product_count: u32,
    },
    Failed {
        started_at: String,
        failed_at: String,
        reason: String,
    },
    Unknown {
        raw_state: String,
        raw_evidence: serde_json::Value,
    },
}

impl SuiteSnapshotCycleState {
    pub const fn kind(&self) -> &str {
        match self {
            Self::Running { .. } => "running",
            Self::Succeeded { .. } => "succeeded",
            Self::Failed { .. } => "failed",
            Self::Unknown { .. } => "unknown",
        }
    }

    pub fn from_storage(raw_state: String, raw_evidence: serde_json::Value) -> Self {
        match serde_json::from_value::<Self>(raw_evidence.clone()) {
            Ok(value) if value.kind() == raw_state => value,
            _ => Self::Unknown {
                raw_state,
                raw_evidence,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteSnapshotCycle {
    pub id: String,
    pub lifecycle: SuiteSnapshotCycleState,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PublicOptOutLifecycle {
    Active {
        asserted_at: String,
    },
    Revoked {
        revoked_at: String,
    },
    Unknown {
        raw_state: String,
        raw_evidence: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicOptOutAssertion {
    pub repository: String,
    pub actor_login: String,
    pub reason: String,
    pub lifecycle: PublicOptOutLifecycle,
    pub verified_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublicOptOutFeed {
    pub schema_version: String,
    pub generated_at: String,
    pub assertions: Vec<PublicOptOutAssertion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PublicOptOutSyncState {
    NotConfigured {
        checked_at: String,
    },
    Running {
        started_at: String,
    },
    Succeeded {
        started_at: String,
        completed_at: String,
        feed_generated_at: String,
        active: u32,
        revoked: u32,
    },
    Failed {
        started_at: String,
        failed_at: String,
        reason: String,
    },
    Unknown {
        raw_state: String,
        raw_evidence: serde_json::Value,
    },
}

impl PublicOptOutSyncState {
    pub const fn kind(&self) -> &str {
        match self {
            Self::NotConfigured { .. } => "not_configured",
            Self::Running { .. } => "running",
            Self::Succeeded { .. } => "succeeded",
            Self::Failed { .. } => "failed",
            Self::Unknown { .. } => "unknown",
        }
    }

    pub fn from_storage(raw_state: String, raw_evidence: serde_json::Value) -> Self {
        match serde_json::from_value::<Self>(raw_evidence.clone()) {
            Ok(value) if value.kind() == raw_state => value,
            _ => Self::Unknown {
                raw_state,
                raw_evidence,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewResponse {
    pub product: &'static str,
    pub tagline: &'static str,
    pub suite_settings: SuiteSettings,
    pub snapshot: Observation<SuiteSnapshotCycle>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RepositoryPolicy {
    pub repository: String,
    pub trusted: bool,
    /// Operator denial. Distinct from `public_opt_out`, which the operator may see
    /// but must not clear.
    pub operator_excluded: bool,
    /// Present when an allowlist is configured; an empty allowlist is not deny-all.
    pub allowlisted: bool,
    /// A verified opt-out from the public patchhive.dev flow. Read-only here: the
    /// repository owner asked to be left alone, and no operator edit revokes that.
    pub public_opt_out: bool,
    /// Where the entry came from — operator, migration, opt-out service.
    pub source: String,
    pub notes: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryPoliciesResponse {
    pub policies: Vec<RepositoryPolicy>,
    pub public_opt_out_sync: Observation<PublicOptOutSyncState>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SaveRepositoryPoliciesRequest {
    pub policies: Vec<RepositoryPolicyInput>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RepositoryPolicyInput {
    pub repository: String,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default)]
    pub operator_excluded: bool,
    #[serde(default)]
    pub allowlisted: bool,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepositoryPolicyDecisionRequest {
    pub repository: String,
    pub product: String,
    pub operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepositoryPolicyDecision {
    pub repository: String,
    pub product: String,
    pub operation: String,
    pub decision: String,
    pub reason: String,
    pub trusted: bool,
    pub operator_excluded: bool,
    pub public_opt_out_checked: bool,
    pub public_opted_out: bool,
    /// Every precedence step the shared evaluator walked, in order. A denial is
    /// evidence a run records, so "why was this repository skipped" stays
    /// answerable after the fact.
    #[serde(default)]
    pub chain: Vec<String>,
    pub policy_version: String,
    pub evaluated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductPrBudget {
    pub product: String,
    pub limit: u32,
    pub used: u32,
    pub remaining: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrReconciliationFailure {
    pub reservation_id: String,
    pub pr_url: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PrReconciliationState {
    NotConfigured {
        checked_at: String,
    },
    Running {
        started_at: String,
    },
    Succeeded {
        started_at: String,
        completed_at: String,
        checked: u32,
        open: u32,
        released: u32,
    },
    Failed {
        started_at: String,
        failed_at: String,
        checked: u32,
        open: u32,
        released: u32,
        reason: String,
        failures: Vec<PrReconciliationFailure>,
    },
    Unknown {
        raw_state: String,
        raw_evidence: serde_json::Value,
    },
}

impl PrReconciliationState {
    pub const fn kind(&self) -> &str {
        match self {
            Self::NotConfigured { .. } => "not_configured",
            Self::Running { .. } => "running",
            Self::Succeeded { .. } => "succeeded",
            Self::Failed { .. } => "failed",
            Self::Unknown { .. } => "unknown",
        }
    }

    pub fn from_storage(raw_state: String, raw_evidence: serde_json::Value) -> Self {
        match serde_json::from_value::<Self>(raw_evidence.clone()) {
            Ok(value) if value.kind() == raw_state => value,
            _ => Self::Unknown {
                raw_state,
                raw_evidence,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrBudgetStatusResponse {
    pub suite_limit: u32,
    pub suite_used: u32,
    pub suite_remaining: u32,
    pub products: Vec<ProductPrBudget>,
    pub reservations: Vec<PrBudgetReservation>,
    pub reconciliation: Observation<PrReconciliationState>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SavePrBudgetRequest {
    pub suite_limit: u32,
    pub products: Vec<ProductPrBudgetInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductPrBudgetInput {
    pub product: String,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRunsSnapshotResponse {
    pub slug: String,
    pub title: String,
    pub api_url: String,
    pub auth_mode: String,
    pub machine_auth_configured: bool,
    pub service_token_configured: bool,
    pub legacy_api_key_configured: bool,
    pub checked_at: String,
    pub runs: Observation<Vec<contract::ProductRunSummary>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRunDetailResponse {
    pub slug: String,
    pub title: String,
    pub api_url: String,
    pub auth_mode: String,
    pub machine_auth_configured: bool,
    pub service_token_configured: bool,
    pub legacy_api_key_configured: bool,
    pub checked_at: String,
    pub detail_path: String,
    pub detail_ok: bool,
    pub remote_status: Option<u16>,
    pub error: String,
    pub detail: Value,
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
    #[serde(default)]
    pub service_token: Option<String>,
    #[serde(default, alias = "api_key")]
    pub legacy_api_key: Option<String>,
    pub enabled: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProvisionServiceTokenRequest {
    #[serde(default)]
    pub operator_api_key: Option<String>,
    #[serde(default)]
    pub api_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionServiceTokenResponse {
    pub product: ProductSettingsItem,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupLauncherStatus {
    pub available: bool,
    pub message: String,
    pub repo_root: String,
    pub docker_available: bool,
    pub docker_compose_available: bool,
    #[serde(default)]
    pub image_mode: String,
    #[serde(default)]
    pub image_tag: String,
    #[serde(default)]
    pub image_pull_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupLauncherProductStatus {
    pub slug: String,
    pub title: String,
    pub product_dir: String,
    pub compose_file: String,
    pub compose_exists: bool,
    pub env_file: String,
    pub env_exists: bool,
    pub env_example_exists: bool,
    pub suite_bootstrap_configured: bool,
    pub frontend_port: u16,
    pub api_port: u16,
    #[serde(default)]
    pub image_mode: String,
    #[serde(default)]
    pub image_status: String,
    #[serde(default)]
    pub image_tag: String,
    #[serde(default)]
    pub image_pull_policy: String,
    #[serde(default)]
    pub image_source: String,
    #[serde(default)]
    pub image_ready: bool,
    #[serde(default)]
    pub compose_declares_images: bool,
    #[serde(default)]
    pub backend_image_ref: String,
    #[serde(default)]
    pub frontend_image_ref: String,
    pub frontend_port_open: bool,
    pub api_port_open: bool,
    pub compose_running: bool,
    #[serde(default)]
    pub first_stack: bool,
    #[serde(default)]
    pub start_ready: bool,
    #[serde(default)]
    pub start_blockers: Vec<String>,
    #[serde(default)]
    pub preflight_status: String,
    pub status: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SetupCredentialRequirement {
    pub key: String,
    pub label: String,
    pub kind: String,
    pub profile: String,
    pub required: bool,
    pub redact: bool,
    pub configured: bool,
    pub placeholder: bool,
    pub status: String,
    pub message: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SetupProductCredentialRequirements {
    pub slug: String,
    pub title: String,
    pub env_file: String,
    pub env_exists: bool,
    pub requirements: Vec<SetupCredentialRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupProductStatus {
    pub runtime: ProductRuntimeItem,
    pub auth_status: Option<crate::pipeline::ProductAuthStatusBody>,
    pub auth_status_error: String,
    pub pairing_ready: bool,
    pub launcher: Option<SetupLauncherProductStatus>,
    pub credentials: Vec<SetupCredentialRequirement>,
    /// Whether `credentials` is an answer or an absence.
    ///
    /// Requirements come from the launcher, which owns the `.env` files. When the
    /// launcher is unreachable the list is empty because HiveCore could not ask —
    /// which is not the same as "this product needs nothing", and rendering the two
    /// identically tells an operator their setup is complete when it is unknown.
    #[serde(default)]
    pub credentials_known: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetLaunchMode {
    StartReady,
    StartAll,
    Unknown,
}

impl FleetLaunchMode {
    pub const fn as_str(&self) -> &str {
        match self {
            Self::StartReady => "start_ready",
            Self::StartAll => "start_all",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_storage(raw_mode: &str) -> Self {
        match raw_mode {
            "start_ready" => Self::StartReady,
            "start_all" => Self::StartAll,
            _ => Self::Unknown,
        }
    }

    pub const fn label(&self) -> &str {
        match self {
            Self::StartReady => "ready fleet",
            Self::StartAll => "full fleet",
            Self::Unknown => "unknown fleet",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetLaunchPhase {
    Observe,
    Preflight,
    Launch,
    Health,
    Pair,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FleetLaunchStepState {
    Queued {
        phase: FleetLaunchPhase,
    },
    Running {
        phase: FleetLaunchPhase,
        started_at: String,
    },
    Ready {
        finished_at: String,
    },
    Attention {
        finished_at: String,
        reason: String,
    },
    Failed {
        finished_at: String,
        reason: String,
    },
    Skipped {
        finished_at: String,
        reason: String,
    },
    Blocked {
        finished_at: String,
        reason: String,
    },
    Unknown {
        raw_state: String,
        raw_evidence: serde_json::Value,
    },
}

impl FleetLaunchStepState {
    pub const fn kind(&self) -> &str {
        match self {
            Self::Queued { .. } => "queued",
            Self::Running { .. } => "running",
            Self::Ready { .. } => "ready",
            Self::Attention { .. } => "attention",
            Self::Failed { .. } => "failed",
            Self::Skipped { .. } => "skipped",
            Self::Blocked { .. } => "blocked",
            Self::Unknown { .. } => "unknown",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Queued { .. } | Self::Running { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupFleetLaunchStep {
    pub slug: String,
    pub title: String,
    pub lifecycle: FleetLaunchStepState,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FleetLaunchJobState {
    Queued {
        queued_at: String,
        lease_expires_at: String,
    },
    Running {
        started_at: String,
        lease_expires_at: String,
    },
    Succeeded {
        finished_at: String,
        ready: u32,
        skipped: u32,
    },
    NeedsAttention {
        finished_at: String,
        ready: u32,
        attention: u32,
        failed: u32,
        skipped: u32,
    },
    Failed {
        finished_at: String,
        failed: u32,
        skipped: u32,
    },
    Blocked {
        finished_at: String,
        blocked: u32,
    },
    NoOp {
        finished_at: String,
        skipped: u32,
    },
    Unknown {
        raw_state: String,
        raw_evidence: serde_json::Value,
    },
}

impl FleetLaunchJobState {
    pub const fn kind(&self) -> &str {
        match self {
            Self::Queued { .. } => "queued",
            Self::Running { .. } => "running",
            Self::Succeeded { .. } => "succeeded",
            Self::NeedsAttention { .. } => "needs_attention",
            Self::Failed { .. } => "failed",
            Self::Blocked { .. } => "blocked",
            Self::NoOp { .. } => "no_op",
            Self::Unknown { .. } => "unknown",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Queued { .. } | Self::Running { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupFleetLaunchJob {
    pub id: String,
    pub mode: FleetLaunchMode,
    pub lifecycle: FleetLaunchJobState,
    pub summary: String,
    pub created_at: String,
    pub updated_at: String,
    pub requested_products: Vec<String>,
    pub started_products: Vec<String>,
    pub skipped_products: Vec<String>,
    pub actions: Vec<String>,
    pub steps: Vec<SetupFleetLaunchStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstStackSetupResponse {
    pub stack_id: String,
    pub launcher: SetupLauncherStatus,
    /// False when the launcher could not be asked what each product requires.
    #[serde(default)]
    pub requirements_known: bool,
    /// Why requirements are unknown, when they are.
    #[serde(default)]
    pub requirements_error: String,
    pub suite_bootstrap_authority: SuiteBootstrapAuthorityState,
    pub latest_smoke: Option<FirstStackSmokeRun>,
    pub latest_fleet_launch: Observation<SetupFleetLaunchJob>,
    pub fleet_launch_history: Observation<Vec<SetupFleetLaunchJob>>,
    pub actions: Vec<String>,
    pub products: Vec<SetupProductStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SuiteBootstrapAuthorityState {
    Ready {
        source: SuiteBootstrapAuthoritySource,
        established_at: Option<String>,
    },
    NotConfigured {
        reason: String,
    },
    Invalid {
        source: SuiteBootstrapAuthoritySource,
        reason: String,
    },
    Unknown {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuiteBootstrapAuthoritySource {
    Environment,
    PersistedEncrypted,
}

impl SuiteBootstrapAuthorityState {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::NotConfigured { reason }
            | Self::Invalid { reason, .. }
            | Self::Unknown { reason } => Some(reason),
            Self::Ready { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstStackSmokeStep {
    pub slug: String,
    pub title: String,
    pub check: String,
    pub status: String,
    pub message: String,
    pub remote_status: Option<u16>,
    pub evidence: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstStackSmokeRun {
    pub id: String,
    #[serde(default)]
    pub tier: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: String,
    pub summary: String,
    pub steps: Vec<FirstStackSmokeStep>,
}

/// One observed health probe.
///
/// The deck used to draw latency sparklines from a seeded pseudo-random generator fed
/// by a constant in its own source. These are measurements: what a probe actually took
/// and whether it succeeded. Uptime is derived from the same samples, so the number and
/// the graph can never disagree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeSample {
    pub observed_at: String,
    pub latency_ms: u64,
    pub healthy: bool,
}

/// One check in a product runbook.
///
/// A runbook step is an observation HiveCore actually made. There is no step for
/// something it cannot do — no "restart the worker pool", no "failover the feed".
/// Those are host operations belonging to the launcher, and a control plane that
/// claims to have performed them while sleeping for 700ms is worse than one that
/// offers nothing: it writes a confident audit line about work that never happened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookStep {
    pub id: String,
    pub label: String,
    /// ok | warn | fail | skipped
    pub status: String,
    pub message: String,
    pub remote_status: Option<u16>,
    /// What the check actually saw, for the operator who wants to disagree with it.
    pub evidence: Value,
}

/// A recorded diagnostic pass over one product.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookRun {
    pub id: String,
    pub product_slug: String,
    pub product_title: String,
    /// ok | degraded | failed
    pub status: String,
    pub started_at: String,
    pub finished_at: String,
    pub summary: String,
    pub steps: Vec<RunbookStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupProductLogsResponse {
    pub slug: String,
    pub title: String,
    pub logs: String,
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

#[cfg(test)]
mod evidence_tests {
    use super::{Observation, PrReconciliationState, PublicOptOutSyncState};
    use serde_json::json;

    #[test]
    fn empty_observation_is_not_a_failed_or_missing_observation() {
        let observed = serde_json::to_value(Observation::observed(Vec::<String>::new()))
            .expect("observed evidence should serialize");
        let failed = serde_json::to_value(Observation::<Vec<String>>::failed("database error"))
            .expect("failed evidence should serialize");
        let missing =
            serde_json::to_value(Observation::<Vec<String>>::not_observed("not requested"))
                .expect("missing evidence should serialize");

        assert_eq!(observed, json!({"state": "observed", "value": []}));
        assert_eq!(
            failed,
            json!({"state": "failed", "reason": "database error"})
        );
        assert_eq!(
            missing,
            json!({"state": "not_observed", "reason": "not requested"})
        );
    }

    #[test]
    fn contradictory_opt_out_sync_storage_is_unknown() {
        let evidence = json!({
            "state": "succeeded",
            "started_at": "2026-08-02T00:00:00Z",
            "completed_at": "2026-08-02T00:00:01Z",
            "feed_generated_at": "2026-08-02T00:00:00Z",
            "active": 1,
            "revoked": 0
        });

        let decoded = PublicOptOutSyncState::from_storage("failed".into(), evidence.clone());

        assert!(matches!(
            decoded,
            PublicOptOutSyncState::Unknown {
                raw_state,
                raw_evidence
            } if raw_state == "failed" && raw_evidence == evidence
        ));
    }

    #[test]
    fn contradictory_pr_reconciliation_storage_is_unknown() {
        let evidence = json!({
            "state": "succeeded",
            "started_at": "2026-08-02T00:00:00Z",
            "completed_at": "2026-08-02T00:00:01Z",
            "checked": 1,
            "open": 1,
            "released": 0
        });

        let decoded = PrReconciliationState::from_storage("running".into(), evidence.clone());

        assert!(matches!(
            decoded,
            PrReconciliationState::Unknown {
                raw_state,
                raw_evidence
            } if raw_state == "running" && raw_evidence == evidence
        ));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum DispatchActionResponse {
    Dispatched {
        event: Box<ProductActionEvent>,
        started_run: bool,
    },
    ApprovalRequired {
        approval: Box<ApprovalRecord>,
    },
}

/// One step of a suite run: a single product action, dispatched in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteRunStep {
    pub product: String,
    pub action: String,
    /// The payload actually dispatched, after any target substitution. Recorded so a
    /// step is answerable later: "what did this run with" cannot be reconstructed
    /// from the composed input once a fan-out has rewritten a field.
    #[serde(default)]
    pub payload: Value,
    /// The target this step was expanded for, when it came from an earlier step's
    /// output. Empty for ordinary steps.
    #[serde(default)]
    pub target: String,
    /// queued | dispatched | pending_approval | failed | skipped
    pub status: String,
    pub message: String,
    pub remote_status: Option<u16>,
    /// The dispatch event this step produced, so a step is traceable to its evidence.
    pub event_id: String,
    /// The pending approval this step produced instead of dispatching.
    #[serde(default)]
    pub approval_id: String,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteRun {
    pub id: String,
    pub name: String,
    /// running | awaiting_approval | completed | failed | halted
    pub status: String,
    pub started_at: String,
    pub finished_at: String,
    pub summary: String,
    pub steps: Vec<SuiteRunStep>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SuiteRunStepInput {
    pub product: String,
    pub action: String,
    #[serde(default)]
    pub payload: Value,
    /// Run this step once per target produced by an earlier step.
    #[serde(default)]
    pub targets: Option<SuiteRunTargets>,
    /// A fail-closed expression over prior stage results.
    #[serde(default)]
    pub gate: Option<String>,
}

/// An explicit reference from one step to an earlier step's output.
///
/// Explicit on purpose. HiveCore could try to infer that a scan produced repositories
/// and that the next step wants them, but inference here is a guess about what an
/// operator meant, applied to actions that reach real repositories. The operator names
/// the step, the path, and the field; HiveCore resolves exactly that and nothing else.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SuiteRunTargets {
    /// 1-based index of an earlier step in the same run.
    pub from_step: usize,
    /// Dot path into that step's response body, resolving to an array.
    /// Empty means the body itself is the array.
    #[serde(default)]
    pub path: String,
    /// Field to read from each array element. Empty means the element is the value.
    #[serde(default)]
    pub field: String,
    /// Payload field to set on each expanded dispatch.
    pub assign_to: String,
    /// Ceiling on fan-out. Clamped server-side — a client-supplied cap is a
    /// suggestion, never the limit, or the limit is whatever the caller says it is.
    #[serde(default)]
    pub max_targets: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartSuiteRunRequest {
    #[serde(default)]
    pub name: String,
    pub steps: Vec<SuiteRunStepInput>,
    /// Keep going after a step fails. Off by default: a suite run that continues
    /// past a failed gate is how a partial result gets mistaken for a clean one.
    #[serde(default)]
    pub continue_on_failure: bool,
}

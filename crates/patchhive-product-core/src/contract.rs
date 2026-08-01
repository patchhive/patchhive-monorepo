use std::collections::HashMap;

use serde::{
    de::Error as _, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::Value;

pub const CONTRACT_SCHEMA_VERSION: &str = "patchhive.product.contract.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductCapabilities {
    pub schema_version: String,
    pub product_slug: String,
    pub display_name: String,
    pub version: String,
    pub standalone: bool,
    pub hivecore: HiveCoreLifecycleSupport,
    pub routes: ProductContractRoutes,
    #[serde(default)]
    pub operating_modes: ProductOperatingModes,
    pub actions: Vec<ProductAction>,
    pub links: Vec<ProductLink>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductOperatingModes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<RunTriggerMode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_selection: Vec<TargetSelectionMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunTriggerMode {
    Operator,
    Schedule,
    Webhook,
    Orchestration,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetSelectionMode {
    #[default]
    Direct,
    Discovery,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HiveCoreLifecycleSupport {
    pub can_launch: bool,
    pub can_start_runs: bool,
    pub can_list_runs: bool,
    pub can_read_run_detail: bool,
    pub can_apply_settings: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductContractRoutes {
    pub health: String,
    pub startup_checks: String,
    pub capabilities: String,
    pub runs: String,
    pub run_detail_template: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings_apply: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionEffect {
    ReadOnly,
    WritesLocalState,
    WritesExternalState,
    MutatesRepository { opens_pull_request: bool },
}

impl ActionEffect {
    pub fn is_read_only(self) -> bool {
        matches!(self, Self::ReadOnly)
    }

    pub fn writes_local_state(self) -> bool {
        matches!(self, Self::WritesLocalState)
    }

    pub fn writes_external_state(self) -> bool {
        matches!(self, Self::WritesExternalState)
    }

    pub fn mutates_repository(self) -> bool {
        matches!(self, Self::MutatesRepository { .. })
    }

    pub fn is_mutating(self) -> bool {
        !self.is_read_only()
    }

    pub fn opens_pull_request(self) -> bool {
        matches!(
            self,
            Self::MutatesRepository {
                opens_pull_request: true
            }
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy {
    Automatic,
    OperatorRequired,
}

impl ApprovalPolicy {
    pub fn requires_operator(self) -> bool {
        matches!(self, Self::OperatorRequired)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionSafety {
    pub effect: ActionEffect,
    pub approval: ApprovalPolicy,
}

impl ActionSafety {
    pub const fn automatic(effect: ActionEffect) -> Self {
        Self {
            effect,
            approval: ApprovalPolicy::Automatic,
        }
    }

    pub const fn operator_required(effect: ActionEffect) -> Self {
        Self {
            effect,
            approval: ApprovalPolicy::OperatorRequired,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductAction {
    pub id: String,
    pub label: String,
    pub method: String,
    pub path: String,
    pub description: String,
    pub starts_run: bool,
    pub destructive: bool,
    pub effect: ActionEffect,
    pub approval: ApprovalPolicy,
    pub scheduleable: bool,
    pub required_scopes: Vec<String>,
    pub credential_requirements: Vec<String>,
    pub operating_modes: Option<ProductOperatingModes>,
}

#[derive(Deserialize)]
struct ProductActionWire {
    id: String,
    label: String,
    method: String,
    path: String,
    description: String,
    starts_run: bool,
    #[serde(default)]
    destructive: bool,
    effect: Option<ActionEffect>,
    approval: Option<ApprovalPolicy>,
    read_only: Option<bool>,
    mutating: Option<bool>,
    requires_approval: Option<bool>,
    opens_pr: Option<bool>,
    #[serde(default)]
    scheduleable: bool,
    #[serde(default)]
    required_scopes: Vec<String>,
    #[serde(default)]
    credential_requirements: Vec<String>,
    #[serde(default)]
    operating_modes: Option<ProductOperatingModes>,
}

impl Serialize for ProductAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ProductAction", 17)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("label", &self.label)?;
        state.serialize_field("method", &self.method)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("description", &self.description)?;
        state.serialize_field("starts_run", &self.starts_run)?;
        state.serialize_field("destructive", &self.destructive)?;
        state.serialize_field("effect", &self.effect)?;
        state.serialize_field("approval", &self.approval)?;
        state.serialize_field("read_only", &self.effect.is_read_only())?;
        state.serialize_field("mutating", &self.effect.is_mutating())?;
        state.serialize_field("requires_approval", &self.approval.requires_operator())?;
        if self.scheduleable {
            state.serialize_field("scheduleable", &true)?;
        }
        state.serialize_field("opens_pr", &self.effect.opens_pull_request())?;
        state.serialize_field("required_scopes", &self.required_scopes)?;
        if !self.credential_requirements.is_empty() {
            state.serialize_field("credential_requirements", &self.credential_requirements)?;
        }
        if let Some(operating_modes) = &self.operating_modes {
            state.serialize_field("operating_modes", operating_modes)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for ProductAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProductActionWire::deserialize(deserializer)?;
        let explicit_effect = wire.effect.is_some();
        let effect = match wire.effect {
            Some(effect) => effect,
            None => match (wire.read_only, wire.mutating) {
                (Some(true), None | Some(false)) => ActionEffect::ReadOnly,
                (None | Some(false), Some(true)) if wire.opens_pr == Some(true) => {
                    ActionEffect::MutatesRepository {
                        opens_pull_request: true,
                    }
                }
                (None | Some(false), Some(true)) => ActionEffect::WritesExternalState,
                _ => {
                    return Err(D::Error::custom(
                        "action effect is required; legacy read_only/mutating fields must declare exactly one true value",
                    ));
                }
            },
        };

        if explicit_effect {
            validate_legacy_bool::<D::Error>("read_only", wire.read_only, effect.is_read_only())?;
            validate_legacy_bool::<D::Error>("mutating", wire.mutating, effect.is_mutating())?;
            validate_legacy_bool::<D::Error>(
                "opens_pr",
                wire.opens_pr,
                effect.opens_pull_request(),
            )?;
        }

        let approval = match wire.approval {
            Some(approval) => {
                validate_legacy_bool::<D::Error>(
                    "requires_approval",
                    wire.requires_approval,
                    approval.requires_operator(),
                )?;
                approval
            }
            None if !explicit_effect => {
                if wire.requires_approval.unwrap_or(false) {
                    ApprovalPolicy::OperatorRequired
                } else {
                    ApprovalPolicy::Automatic
                }
            }
            None => {
                return Err(D::Error::custom(
                    "action approval policy is required when effect uses the explicit contract",
                ));
            }
        };

        Ok(Self {
            id: wire.id,
            label: wire.label,
            method: wire.method,
            path: wire.path,
            description: wire.description,
            starts_run: wire.starts_run,
            destructive: wire.destructive,
            effect,
            approval,
            scheduleable: wire.scheduleable,
            required_scopes: wire.required_scopes,
            credential_requirements: wire.credential_requirements,
            operating_modes: wire.operating_modes,
        })
    }
}

fn validate_legacy_bool<E>(field: &str, actual: Option<bool>, expected: bool) -> Result<(), E>
where
    E: serde::de::Error,
{
    if actual.is_some_and(|actual| actual != expected) {
        return Err(E::custom(format!(
            "legacy {field} contradicts the explicit action contract"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductLink {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductRunsResponse {
    pub schema_version: String,
    pub product_slug: String,
    pub runs: Vec<ProductRunSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductRunSummary {
    pub id: String,
    #[serde(default)]
    pub lifecycle_status: RunLifecycleStatus,
    pub status: String,
    pub title: String,
    pub summary: String,
    pub created_at: String,
    pub updated_at: String,
    pub detail_path: String,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductRunEventsResponse {
    pub schema_version: String,
    pub product_slug: String,
    pub run_id: String,
    pub events: Vec<ProductRunEvent>,
}

/// A presentation page over a complete, product-owned evidence collection.
///
/// Products must persist the complete collection before using this type. A
/// page is an API/view boundary, never the product's storage boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetainedEvidencePage<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
    pub has_more: bool,
}

impl<T: Clone> RetainedEvidencePage<T> {
    pub fn from_retained(items: &[T], offset: u64, limit: u64) -> Self {
        let total = items.len() as u64;
        let offset = offset.min(total);
        let limit = limit.max(1);
        let end = offset.saturating_add(limit).min(total);

        Self {
            items: items[offset as usize..end as usize].to_vec(),
            total,
            offset,
            limit,
            has_more: end < total,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductRunEvent {
    pub id: String,
    pub run_id: String,
    #[serde(default)]
    pub product_slug: String,
    #[serde(default)]
    pub sequence: u64,
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub level: RunEventLevel,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub actor: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<ProductRunArtifact>,
    #[serde(default)]
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductRunArtifact {
    pub kind: String,
    pub label: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub external_id: String,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DispatchActionInput {
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub path_params: HashMap<String, String>,
    #[serde(default)]
    pub query: HashMap<String, String>,
}

impl Default for DispatchActionInput {
    fn default() -> Self {
        Self {
            payload: Value::Null,
            path_params: HashMap::new(),
            query: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuiteScheduleRecord {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub product: String,
    pub action_id: String,
    pub cadence: String,
    pub cron: String,
    pub timezone: String,
    pub enabled: bool,
    pub target_scope: Value,
    pub approval_policy: String,
    pub next_run_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default)]
    pub dispatch: DispatchActionInput,
}

impl SuiteScheduleRecord {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        product: impl Into<String>,
        action_id: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: CONTRACT_SCHEMA_VERSION.into(),
            id: id.into(),
            name: name.into(),
            product: product.into(),
            action_id: action_id.into(),
            cadence: "manual".into(),
            cron: String::new(),
            timezone: "UTC".into(),
            enabled: false,
            target_scope: Value::Null,
            approval_policy: "read_only_auto".into(),
            next_run_at: String::new(),
            last_run_id: None,
            last_run_at: None,
            last_status: "idle".into(),
            last_error: None,
            dispatch: DispatchActionInput::default(),
        }
    }
}

impl ProductAction {
    pub fn is_read_only(&self) -> bool {
        self.effect.is_read_only()
    }

    pub fn is_mutating(&self) -> bool {
        self.effect.is_mutating()
    }

    pub fn requires_approval(&self) -> bool {
        self.approval.requires_operator()
    }

    pub fn opens_pull_request(&self) -> bool {
        self.effect.opens_pull_request()
    }

    pub fn scheduleable(mut self, value: bool) -> Self {
        self.scheduleable = value;
        if let Some(modes) = &mut self.operating_modes {
            if value {
                push_unique(&mut modes.triggers, RunTriggerMode::Schedule);
            } else {
                modes
                    .triggers
                    .retain(|mode| *mode != RunTriggerMode::Schedule);
            }
        }
        self
    }

    pub fn trigger_modes<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = RunTriggerMode>,
    {
        if let Some(modes) = &mut self.operating_modes {
            modes.triggers.clear();
            for value in values {
                push_unique(&mut modes.triggers, value);
            }
            self.scheduleable = modes.triggers.contains(&RunTriggerMode::Schedule);
        }
        self
    }

    pub fn target_selection_modes<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = TargetSelectionMode>,
    {
        if let Some(modes) = &mut self.operating_modes {
            modes.target_selection.clear();
            for value in values {
                push_unique(&mut modes.target_selection, value);
            }
        }
        self
    }

    pub fn credential_requirements<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.credential_requirements = values.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunLifecycleStatus {
    Standby,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Held,
    Skipped,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunEventLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
    Success,
}

impl RunEventLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Success => "success",
        }
    }
}

impl RunLifecycleStatus {
    pub fn from_status(value: &str) -> Self {
        let normalized = value.trim().to_ascii_lowercase().replace([' ', '-'], "_");
        match normalized.as_str() {
            "" => Self::Unknown,
            "standby" | "idle" | "ready" => Self::Standby,
            "queued" | "pending" | "scheduled" => Self::Queued,
            "running" | "active" | "working" | "in_progress" | "processing" | "dispatching" => {
                Self::Running
            }
            "completed" | "complete" | "done" | "saved" | "success" | "succeeded" => {
                Self::Completed
            }
            "failed" | "failure" | "error" => Self::Failed,
            "cancelled" | "canceled" => Self::Cancelled,
            "held" | "hold" => Self::Held,
            "skipped" | "skip" => Self::Skipped,
            _ => Self::Unknown,
        }
    }

    pub fn from_runtime_status(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace([' ', '-'], "_");
        match normalized.as_str() {
            "queued" | "pending" | "scheduled" => Some(Self::Queued),
            "running" | "active" | "working" | "in_progress" | "processing" | "dispatching" => {
                Some(Self::Running)
            }
            "completed" | "complete" | "done" | "saved" | "success" | "succeeded" | "partial" => {
                Some(Self::Completed)
            }
            "failed" | "failure" | "error" | "crashed" | "timeout" | "timed_out" => {
                Some(Self::Failed)
            }
            "cancelled" | "canceled" => Some(Self::Cancelled),
            "held" | "hold" | "blocked" => Some(Self::Held),
            "skipped" | "skip" => Some(Self::Skipped),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standby => "standby",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Held => "held",
            Self::Skipped => "skipped",
            Self::Unknown => "unknown",
        }
    }
}

impl ProductRunEvent {
    pub fn new(
        id: impl Into<String>,
        run_id: impl Into<String>,
        phase: impl Into<String>,
        level: RunEventLevel,
        message: impl Into<String>,
        created_at: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            run_id: run_id.into(),
            product_slug: String::new(),
            sequence: 0,
            phase: phase.into(),
            level,
            message: message.into(),
            source: String::new(),
            actor: String::new(),
            created_at: created_at.into(),
            artifact: None,
            raw: Value::Null,
        }
    }

    pub fn product_slug(mut self, value: impl Into<String>) -> Self {
        self.product_slug = value.into();
        self
    }

    pub fn sequence(mut self, value: u64) -> Self {
        self.sequence = value;
        self
    }

    pub fn source(mut self, value: impl Into<String>) -> Self {
        self.source = value.into();
        self
    }

    pub fn actor(mut self, value: impl Into<String>) -> Self {
        self.actor = value.into();
        self
    }

    pub fn artifact(mut self, value: ProductRunArtifact) -> Self {
        self.artifact = Some(value);
        self
    }

    pub fn raw(mut self, value: Value) -> Self {
        self.raw = value;
        self
    }
}

impl ProductRunArtifact {
    pub fn new(kind: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            label: label.into(),
            url: String::new(),
            external_id: String::new(),
            metadata: Value::Null,
        }
    }

    pub fn url(mut self, value: impl Into<String>) -> Self {
        self.url = value.into();
        self
    }

    pub fn external_id(mut self, value: impl Into<String>) -> Self {
        self.external_id = value.into();
        self
    }

    pub fn metadata(mut self, value: Value) -> Self {
        self.metadata = value;
        self
    }
}

pub fn capabilities(
    product_slug: impl Into<String>,
    display_name: impl Into<String>,
    actions: Vec<ProductAction>,
    links: Vec<ProductLink>,
) -> ProductCapabilities {
    let can_start_runs = actions.iter().any(|action| action.starts_run);
    let operating_modes = aggregate_operating_modes(&actions);
    ProductCapabilities {
        schema_version: CONTRACT_SCHEMA_VERSION.into(),
        product_slug: product_slug.into(),
        display_name: display_name.into(),
        version: "0.1.0".into(),
        standalone: true,
        hivecore: HiveCoreLifecycleSupport {
            can_launch: true,
            can_start_runs,
            can_list_runs: true,
            can_read_run_detail: true,
            can_apply_settings: false,
        },
        routes: ProductContractRoutes {
            health: "/health".into(),
            startup_checks: "/startup/checks".into(),
            capabilities: "/capabilities".into(),
            runs: "/runs".into(),
            run_detail_template: "/runs/{id}".into(),
            settings_apply: None,
        },
        operating_modes,
        actions,
        links,
    }
}

pub fn action(
    id: impl Into<String>,
    label: impl Into<String>,
    method: impl Into<String>,
    path: impl Into<String>,
    description: impl Into<String>,
    starts_run: bool,
    safety: ActionSafety,
) -> ProductAction {
    let operating_modes = starts_run.then(|| ProductOperatingModes {
        triggers: vec![RunTriggerMode::Operator, RunTriggerMode::Orchestration],
        target_selection: vec![TargetSelectionMode::Direct],
    });
    ProductAction {
        id: id.into(),
        label: label.into(),
        method: method.into(),
        path: path.into(),
        description: description.into(),
        starts_run,
        destructive: false,
        effect: safety.effect,
        approval: safety.approval,
        scheduleable: false,
        required_scopes: vec![crate::auth::SERVICE_SCOPE_ACTIONS_DISPATCH.into()],
        credential_requirements: vec![],
        operating_modes,
    }
}

fn aggregate_operating_modes(actions: &[ProductAction]) -> ProductOperatingModes {
    let mut aggregate = ProductOperatingModes::default();
    for modes in actions
        .iter()
        .filter_map(|action| action.operating_modes.as_ref())
    {
        for trigger in &modes.triggers {
            push_unique(&mut aggregate.triggers, trigger.clone());
        }
        for selection in &modes.target_selection {
            push_unique(&mut aggregate.target_selection, *selection);
        }
    }
    aggregate
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

pub fn link(
    id: impl Into<String>,
    label: impl Into<String>,
    path: impl Into<String>,
) -> ProductLink {
    ProductLink {
        id: id.into(),
        label: label.into(),
        path: path.into(),
    }
}

pub fn parse_dispatch_input(raw: Value) -> DispatchActionInput {
    let Some(object) = raw.as_object() else {
        return DispatchActionInput {
            payload: raw,
            ..DispatchActionInput::default()
        };
    };

    let has_wrapper_keys = object.contains_key("payload")
        || object.contains_key("path_params")
        || object.contains_key("query");
    if !has_wrapper_keys {
        return DispatchActionInput {
            payload: raw,
            ..DispatchActionInput::default()
        };
    }

    DispatchActionInput {
        payload: object.get("payload").cloned().unwrap_or(Value::Null),
        path_params: string_map_from_value(object.get("path_params")),
        query: string_map_from_value(object.get("query")),
    }
}

pub fn cadence_from_hours(hours: u32) -> String {
    match hours.max(1) {
        1 => "hourly".into(),
        24 => "daily".into(),
        168 => "weekly".into(),
        value => format!("every_{value}h"),
    }
}

pub fn interval_cron_label(hours: u32) -> String {
    format!("interval:{}h", hours.max(1))
}

pub fn runs_from_history<T: Serialize>(
    product_slug: impl Into<String>,
    history_items: Vec<T>,
) -> ProductRunsResponse {
    let product_slug = product_slug.into();
    let runs = history_items
        .into_iter()
        .filter_map(|item| serde_json::to_value(item).ok())
        .map(|raw| run_summary_from_value(&raw))
        .collect();

    ProductRunsResponse {
        schema_version: CONTRACT_SCHEMA_VERSION.into(),
        product_slug,
        runs,
    }
}

pub fn runs_from_values(
    product_slug: impl Into<String>,
    history_items: Vec<Value>,
) -> ProductRunsResponse {
    let product_slug = product_slug.into();
    let runs = history_items
        .into_iter()
        .map(|raw| run_summary_from_value(&raw))
        .collect();

    ProductRunsResponse {
        schema_version: CONTRACT_SCHEMA_VERSION.into(),
        product_slug,
        runs,
    }
}

pub fn run_events_response(
    product_slug: impl Into<String>,
    run_id: impl Into<String>,
    events: Vec<ProductRunEvent>,
) -> ProductRunEventsResponse {
    ProductRunEventsResponse {
        schema_version: CONTRACT_SCHEMA_VERSION.into(),
        product_slug: product_slug.into(),
        run_id: run_id.into(),
        events,
    }
}

fn run_summary_from_value(raw: &Value) -> ProductRunSummary {
    let id = first_string(raw, &["id", "run_id", "scan_id", "review_id"])
        .unwrap_or_else(|| "unknown".into());
    let lifecycle_status = first_string(
        raw,
        &[
            "lifecycle_status",
            "lifecycle",
            "run_status",
            "execution_status",
        ],
    )
    .map(|value| RunLifecycleStatus::from_status(&value))
    .filter(|status| *status != RunLifecycleStatus::Unknown)
    .or_else(|| {
        first_string(raw, &["status"])
            .and_then(|value| RunLifecycleStatus::from_runtime_status(&value))
    })
    .unwrap_or(RunLifecycleStatus::Completed);
    let status = first_string(raw, &["status", "recommendation", "readiness"])
        .unwrap_or_else(|| "completed".into());
    let created_at =
        first_string(raw, &["created_at", "started_at", "opened_at"]).unwrap_or_default();
    let updated_at =
        first_string(raw, &["updated_at", "finished_at", "last_checked"]).unwrap_or_default();
    let title = first_string(
        raw,
        &[
            "title",
            "repo",
            "repo_name",
            "repository",
            "target_repo",
            "pr_url",
            "top_repo",
        ],
    )
    .unwrap_or_else(|| id.clone());
    let summary = first_string(raw, &["summary", "message", "decision", "reason"])
        .or_else(|| numeric_summary(raw))
        .unwrap_or_default();

    ProductRunSummary {
        id: id.clone(),
        lifecycle_status,
        status,
        title,
        summary,
        created_at,
        updated_at,
        detail_path: format!("/runs/{id}"),
        raw: raw.clone(),
    }
}

fn first_string(raw: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        raw.get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn string_map_from_value(value: Option<&Value>) -> HashMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        value
                            .as_str()
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| value.to_string()),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn numeric_summary(raw: &Value) -> Option<String> {
    let object = raw.as_object()?;
    let parts = object
        .iter()
        .filter_map(|(key, value)| {
            if matches!(
                key.as_str(),
                "id" | "created_at" | "started_at" | "updated_at"
            ) {
                return None;
            }
            value
                .as_u64()
                .map(|number| format!("{} {}", number, key.replace('_', " ")))
        })
        .take(3)
        .collect::<Vec<_>>();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        action, cadence_from_hours, capabilities, interval_cron_label, parse_dispatch_input,
        run_events_response, runs_from_values, ActionEffect, ActionSafety, ApprovalPolicy,
        ProductRunArtifact, ProductRunEvent, RetainedEvidencePage, RunEventLevel,
        RunLifecycleStatus, RunTriggerMode, SuiteScheduleRecord, TargetSelectionMode,
    };
    use serde_json::json;

    #[test]
    fn capabilities_report_run_support_from_actions() {
        let caps = capabilities(
            "signal-hive",
            "SignalHive",
            vec![action(
                "scan",
                "Run scan",
                "POST",
                "/scan",
                "Scan repos",
                true,
                ActionSafety::automatic(ActionEffect::WritesLocalState),
            )],
            vec![],
        );

        assert_eq!(caps.schema_version, "patchhive.product.contract.v1");
        assert!(caps.standalone);
        assert!(caps.hivecore.can_start_runs);
        assert_eq!(caps.routes.runs, "/runs");
        assert_eq!(
            caps.actions[0].required_scopes,
            vec![crate::auth::SERVICE_SCOPE_ACTIONS_DISPATCH.to_string()]
        );
        assert_eq!(caps.actions[0].effect, ActionEffect::WritesLocalState);
        assert_eq!(caps.actions[0].approval, ApprovalPolicy::Automatic);
        assert!(caps.actions[0].credential_requirements.is_empty());
        assert_eq!(
            caps.operating_modes.triggers,
            vec![RunTriggerMode::Operator, RunTriggerMode::Orchestration]
        );
        assert_eq!(
            caps.operating_modes.target_selection,
            vec![TargetSelectionMode::Direct]
        );
    }

    #[test]
    fn action_wire_contract_emits_explicit_effect_and_approval() {
        let action = action(
            "run",
            "Run patch hunt",
            "POST",
            "/run",
            "Open a validated pull request.",
            true,
            ActionSafety::operator_required(ActionEffect::MutatesRepository {
                opens_pull_request: true,
            }),
        );

        let value = serde_json::to_value(action).expect("action should serialize");
        assert_eq!(value["effect"]["kind"], "mutates_repository");
        assert_eq!(value["effect"]["opens_pull_request"], true);
        assert_eq!(value["approval"], "operator_required");
        assert_eq!(value["read_only"], false);
        assert_eq!(value["mutating"], true);
        assert_eq!(value["requires_approval"], true);
        assert_eq!(value["opens_pr"], true);
    }

    #[test]
    fn action_wire_contract_rejects_missing_or_contradictory_effects() {
        let base = json!({
            "id": "scan",
            "label": "Scan",
            "method": "POST",
            "path": "/scan",
            "description": "Scan a repository.",
            "starts_run": true
        });
        assert!(serde_json::from_value::<super::ProductAction>(base.clone()).is_err());

        let mut contradictory = base;
        contradictory["effect"] = json!({"kind": "read_only"});
        contradictory["approval"] = json!("automatic");
        contradictory["mutating"] = json!(true);
        assert!(serde_json::from_value::<super::ProductAction>(contradictory).is_err());
    }

    #[test]
    fn action_wire_contract_accepts_only_unambiguous_legacy_effects() {
        let legacy = json!({
            "id": "scan",
            "label": "Scan",
            "method": "POST",
            "path": "/scan",
            "description": "Scan a repository.",
            "starts_run": true,
            "read_only": true
        });
        let decoded: super::ProductAction =
            serde_json::from_value(legacy).expect("unambiguous legacy action should decode");
        assert_eq!(decoded.effect, ActionEffect::ReadOnly);
        assert_eq!(decoded.approval, ApprovalPolicy::Automatic);

        let ambiguous = json!({
            "id": "scan",
            "label": "Scan",
            "method": "POST",
            "path": "/scan",
            "description": "Scan a repository.",
            "starts_run": true,
            "read_only": false,
            "mutating": false
        });
        assert!(serde_json::from_value::<super::ProductAction>(ambiguous).is_err());
    }

    #[test]
    fn operating_modes_keep_trigger_and_target_selection_independent() {
        let caps = capabilities(
            "repo-reaper",
            "RepoReaper",
            vec![action(
                "run",
                "Run patch hunt",
                "POST",
                "/run",
                "Find and fix suitable work.",
                true,
                ActionSafety::operator_required(ActionEffect::MutatesRepository {
                    opens_pull_request: true,
                }),
            )
            .scheduleable(true)
            .target_selection_modes([TargetSelectionMode::Direct, TargetSelectionMode::Discovery])],
            vec![],
        );

        assert_eq!(
            caps.operating_modes.triggers,
            vec![
                RunTriggerMode::Operator,
                RunTriggerMode::Orchestration,
                RunTriggerMode::Schedule,
            ]
        );
        assert_eq!(
            caps.operating_modes.target_selection,
            vec![TargetSelectionMode::Direct, TargetSelectionMode::Discovery]
        );
        assert!(caps.actions[0].scheduleable);
    }

    #[test]
    fn operating_modes_are_backward_compatible_during_rolling_upgrades() {
        let caps = capabilities(
            "merge-keeper",
            "MergeKeeper",
            vec![action(
                "assess",
                "Assess PR",
                "POST",
                "/assess",
                "Assess one pull request.",
                true,
                ActionSafety::automatic(ActionEffect::WritesExternalState),
            )],
            vec![],
        );
        let mut legacy = serde_json::to_value(caps).expect("capabilities should serialize");
        legacy
            .as_object_mut()
            .expect("capabilities object")
            .remove("operating_modes");
        legacy["actions"][0]
            .as_object_mut()
            .expect("action object")
            .remove("operating_modes");

        let decoded: super::ProductCapabilities =
            serde_json::from_value(legacy).expect("legacy capabilities should deserialize");
        assert_eq!(
            decoded.operating_modes,
            super::ProductOperatingModes::default()
        );
        assert!(decoded.actions[0].operating_modes.is_none());
    }

    #[test]
    fn dispatch_input_accepts_wrapped_payloads() {
        let input = parse_dispatch_input(json!({
            "path_params": { "name": "daily" },
            "query": { "dry": true, "limit": 3 },
            "payload": { "repo": "patchhive/example" }
        }));

        assert_eq!(input.path_params["name"], "daily");
        assert_eq!(input.query["dry"], "true");
        assert_eq!(input.query["limit"], "3");
        assert_eq!(input.payload["repo"], "patchhive/example");
    }

    #[test]
    fn retained_evidence_pages_preserve_complete_collection_metadata() {
        let retained = vec!["one", "two", "three", "four", "five"];
        let first = RetainedEvidencePage::from_retained(&retained, 0, 2);
        let last = RetainedEvidencePage::from_retained(&retained, 4, 2);

        assert_eq!(first.items, vec!["one", "two"]);
        assert_eq!(first.total, 5);
        assert!(first.has_more);
        assert_eq!(last.items, vec!["five"]);
        assert_eq!(last.total, 5);
        assert!(!last.has_more);
        assert_eq!(retained.len(), 5);
    }

    #[test]
    fn dispatch_input_treats_plain_objects_as_payloads() {
        let input = parse_dispatch_input(json!({ "repo": "patchhive/example" }));

        assert_eq!(input.payload["repo"], "patchhive/example");
        assert!(input.path_params.is_empty());
        assert!(input.query.is_empty());
    }

    #[test]
    fn run_event_helpers_preserve_artifact_metadata() {
        let event = ProductRunEvent::new(
            "evt_1",
            "run_1",
            "submit",
            RunEventLevel::Success,
            "Opened draft pull request",
            "2026-07-04T20:00:00Z",
        )
        .product_slug("repo-reaper")
        .sequence(7)
        .source("gatekeeper")
        .actor("PatchHive Gatekeeper")
        .artifact(
            ProductRunArtifact::new("pull_request", "Draft PR")
                .url("https://github.com/patchhive/example/pull/1")
                .external_id("1")
                .metadata(json!({ "draft": true })),
        );

        let response = run_events_response("repo-reaper", "run_1", vec![event]);
        assert_eq!(response.schema_version, "patchhive.product.contract.v1");
        assert_eq!(response.events[0].sequence, 7);
        assert_eq!(response.events[0].level.as_str(), "success");
        assert_eq!(
            response.events[0].artifact.as_ref().unwrap().metadata["draft"],
            true
        );
    }

    #[test]
    fn run_event_deserializes_legacy_level_defaults() {
        let event: ProductRunEvent = serde_json::from_value(json!({
            "id": "evt_legacy",
            "run_id": "run_legacy",
            "created_at": "2026-07-04T20:00:00Z"
        }))
        .expect("legacy event should deserialize");

        assert_eq!(event.level, RunEventLevel::Info);
        assert_eq!(event.sequence, 0);
        assert_eq!(event.raw, serde_json::Value::Null);
    }

    #[test]
    fn suite_schedule_record_uses_contract_defaults() {
        let schedule = SuiteScheduleRecord::new(
            "sched_daily",
            "daily-maintenance-scan",
            "signal-hive",
            "scan",
        );

        assert_eq!(schedule.schema_version, "patchhive.product.contract.v1");
        assert_eq!(schedule.product, "signal-hive");
        assert_eq!(schedule.approval_policy, "read_only_auto");
        assert_eq!(schedule.last_status, "idle");
        assert_eq!(schedule.dispatch.payload, serde_json::Value::Null);
    }

    #[test]
    fn schedule_interval_helpers_describe_product_local_cadence() {
        assert_eq!(cadence_from_hours(1), "hourly");
        assert_eq!(cadence_from_hours(24), "daily");
        assert_eq!(cadence_from_hours(168), "weekly");
        assert_eq!(cadence_from_hours(6), "every_6h");
        assert_eq!(interval_cron_label(6), "interval:6h");
    }

    #[test]
    fn runs_from_history_values_normalizes_common_fields() {
        let runs = runs_from_values(
            "dep-triage",
            vec![json!({
                "id": "scan_123",
                "repo": "patchhive/example",
                "created_at": "2026-04-21T10:00:00Z",
                "summary": "2 updates need attention",
                "tracked_items": 7
            })],
        );

        assert_eq!(runs.product_slug, "dep-triage");
        assert_eq!(runs.runs[0].id, "scan_123");
        assert_eq!(runs.runs[0].status, "completed");
        assert_eq!(runs.runs[0].lifecycle_status, RunLifecycleStatus::Completed);
        assert_eq!(runs.runs[0].title, "patchhive/example");
        assert_eq!(runs.runs[0].detail_path, "/runs/scan_123");
    }

    #[test]
    fn run_lifecycle_status_preserves_product_decision_status() {
        let runs = runs_from_values(
            "merge-keeper",
            vec![json!({
                "id": "run_123",
                "repo": "patchhive/example",
                "readiness": "blocked",
                "run_status": "done",
                "created_at": "2026-04-21T10:00:00Z",
                "summary": "Checks are failing"
            })],
        );

        assert_eq!(runs.runs[0].status, "blocked");
        assert_eq!(runs.runs[0].lifecycle_status, RunLifecycleStatus::Completed);
    }

    #[test]
    fn run_lifecycle_status_infers_runtime_status_when_lifecycle_is_missing() {
        let runs = runs_from_values(
            "repo-reaper",
            vec![
                json!({
                    "id": "run_active",
                    "target_repo": "patchhive/example",
                    "status": "running",
                    "started_at": "2026-04-21T10:00:00Z"
                }),
                json!({
                    "id": "run_failed",
                    "target_repo": "patchhive/example",
                    "status": "crashed",
                    "started_at": "2026-04-21T10:00:00Z"
                }),
                json!({
                    "id": "run_partial",
                    "target_repo": "patchhive/example",
                    "status": "partial",
                    "started_at": "2026-04-21T10:00:00Z"
                }),
            ],
        );

        assert_eq!(runs.runs[0].lifecycle_status, RunLifecycleStatus::Running);
        assert_eq!(runs.runs[1].lifecycle_status, RunLifecycleStatus::Failed);
        assert_eq!(runs.runs[2].lifecycle_status, RunLifecycleStatus::Completed);
    }

    #[test]
    fn run_lifecycle_status_does_not_treat_ready_decisions_as_standby_runs() {
        let runs = runs_from_values(
            "merge-keeper",
            vec![json!({
                "id": "run_ready",
                "repo": "patchhive/example",
                "status": "ready",
                "created_at": "2026-04-21T10:00:00Z"
            })],
        );

        assert_eq!(runs.runs[0].status, "ready");
        assert_eq!(runs.runs[0].lifecycle_status, RunLifecycleStatus::Completed);
    }

    #[test]
    fn run_lifecycle_status_normalizes_common_runtime_words() {
        assert_eq!(
            RunLifecycleStatus::from_status("in-progress"),
            RunLifecycleStatus::Running
        );
        assert_eq!(
            RunLifecycleStatus::from_status("FAILED"),
            RunLifecycleStatus::Failed
        );
        assert_eq!(
            RunLifecycleStatus::from_status("saved"),
            RunLifecycleStatus::Completed
        );
    }

    #[test]
    fn run_summary_deserializes_legacy_json_without_lifecycle_status() {
        let summary: super::ProductRunSummary = serde_json::from_value(json!({
            "id": "run_legacy",
            "status": "completed",
            "title": "patchhive/example",
            "summary": "legacy response",
            "created_at": "2026-04-21T10:00:00Z",
            "updated_at": "",
            "detail_path": "/runs/run_legacy",
            "raw": {}
        }))
        .expect("legacy run summary without lifecycle status should parse");

        assert_eq!(summary.lifecycle_status, RunLifecycleStatus::Unknown);
    }
}

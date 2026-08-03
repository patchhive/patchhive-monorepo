use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::now_rfc3339;

/// The stable identity of one piece of maintenance work.
///
/// Product, action, mandate, and discovery source deliberately do not participate
/// in this identity. Two products finding the same work must converge on one row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkIdentity {
    pub kind: String,
    pub repository: String,
    pub subject_ref: String,
}

impl WorkIdentity {
    fn normalized(self) -> Result<Self, String> {
        let kind = required("kind", self.kind, 80)?.to_ascii_lowercase();
        if !kind
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        {
            return Err("kind must contain only letters, numbers, hyphens, or underscores".into());
        }
        let repository = required("repository", self.repository, 240)?.to_ascii_lowercase();
        let subject_ref = required("subject_ref", self.subject_ref, 500)?;
        let parts = repository.split('/').collect::<Vec<_>>();
        if parts.len() != 2
            || parts.iter().any(|part| part.is_empty())
            || repository.chars().any(char::is_whitespace)
        {
            return Err("repository must be a GitHub owner/repository name".into());
        }
        Ok(Self {
            kind,
            repository,
            subject_ref,
        })
    }

    pub fn fingerprint(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("work identity serialization cannot fail");
        format!("{:x}", Sha256::digest(bytes))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "origin", rename_all = "snake_case")]
pub enum WorkOrigin {
    Operator,
    ProductRun {
        product_slug: String,
        run_id: String,
    },
    SuiteRun {
        run_id: String,
    },
    ConductorTick {
        tick_id: String,
    },
}

impl WorkOrigin {
    fn normalized(self) -> Result<Self, String> {
        match self {
            Self::Operator => Ok(Self::Operator),
            Self::ProductRun {
                product_slug,
                run_id,
            } => Ok(Self::ProductRun {
                product_slug: required("origin product_slug", product_slug, 100)?,
                run_id: required("origin run_id", run_id, 200)?,
            }),
            Self::SuiteRun { run_id } => Ok(Self::SuiteRun {
                run_id: required("origin run_id", run_id, 200)?,
            }),
            Self::ConductorTick { tick_id } => Ok(Self::ConductorTick {
                tick_id: required("origin tick_id", tick_id, 200)?,
            }),
        }
    }
}

/// The dispatch HiveCore is proposing, not permission or an instruction to run it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProposedDispatch {
    pub product_slug: String,
    pub action_id: String,
    pub input: Value,
}

impl ProposedDispatch {
    fn normalized(self) -> Result<Self, String> {
        if !self.input.is_object() {
            return Err("proposed dispatch input must be a JSON object".into());
        }
        Ok(Self {
            product_slug: required("product_slug", self.product_slug, 100)?,
            action_id: required("action_id", self.action_id, 100)?,
            input: self.input,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProposeWorkRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mandate_id: Option<String>,
    pub identity: WorkIdentity,
    pub proposed_dispatch: ProposedDispatch,
    pub origin: WorkOrigin,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkProposal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mandate_id: Option<String>,
    pub identity: WorkIdentity,
    pub proposed_dispatch: ProposedDispatch,
    pub origin: WorkOrigin,
    pub rationale: String,
}

impl WorkProposal {
    pub fn from_request(request: ProposeWorkRequest) -> Result<Self, String> {
        let mandate_id = request
            .mandate_id
            .map(|value| required("mandate_id", value, 200))
            .transpose()?;
        Ok(Self {
            mandate_id,
            identity: request.identity.normalized()?,
            proposed_dispatch: request.proposed_dispatch.normalized()?,
            origin: request.origin.normalized()?,
            rationale: required("rationale", request.rationale, 2_000)?,
        })
    }
}

/// Durable work state. This intentionally starts small: transitions beyond
/// discovery are not implemented yet, so pretending to understand their stored
/// evidence would be worse than decoding them as unknown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WorkLifecycle {
    Discovered {
        discovered_at: String,
    },
    Unknown {
        raw_state: String,
        raw_evidence: Value,
    },
}

impl WorkLifecycle {
    pub const fn kind(&self) -> &str {
        match self {
            Self::Discovered { .. } => "discovered",
            Self::Unknown { .. } => "unknown",
        }
    }

    pub fn from_storage(raw_state: String, raw_evidence: Value) -> Self {
        let parsed = serde_json::from_value::<Self>(raw_evidence.clone());
        match parsed {
            Ok(value) if value.kind() == raw_state => value,
            _ => Self::Unknown {
                raw_state,
                raw_evidence,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkItem {
    pub id: String,
    pub fingerprint: String,
    pub proposal: WorkProposal,
    pub lifecycle: WorkLifecycle,
    pub attempts: u32,
    pub created_at: String,
    pub updated_at: String,
}

impl WorkItem {
    pub fn discovered(proposal: WorkProposal) -> Self {
        let now = now_rfc3339();
        Self {
            id: format!("work_{}", Uuid::now_v7()),
            fingerprint: proposal.identity.fingerprint(),
            proposal,
            lifecycle: WorkLifecycle::Discovered {
                discovered_at: now.clone(),
            },
            attempts: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ProposeWorkOutcome {
    Created { item: WorkItem },
    Deduplicated { item: WorkItem, observed_at: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MandateAutonomy {
    Observe,
    Propose,
    ActWithApproval,
    Act,
}

impl MandateAutonomy {
    /// The conductor has not earned dispatch authority yet. Requested autonomy is
    /// retained, while every executable plan is capped at propose.
    pub const fn effective_now(self) -> Self {
        match self {
            Self::Observe => Self::Observe,
            Self::Propose | Self::ActWithApproval | Self::Act => Self::Propose,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MandateScope {
    pub search_query: String,
    pub topics: Vec<String>,
    pub languages: Vec<String>,
    pub min_stars: u32,
    pub max_repositories: u32,
    pub issues_per_repository: u32,
    pub stale_days: u32,
}

impl MandateScope {
    fn normalized(self) -> Result<Self, String> {
        let search_query = self.search_query.trim().to_owned();
        if search_query.starts_with("repo:") {
            return Err(
                "mandate scope is autonomous discovery; use product direct targeting for one repository"
                    .into(),
            );
        }
        let topics = normalized_terms("topics", self.topics)?;
        let languages = normalized_terms("languages", self.languages)?;
        if search_query.is_empty() && topics.is_empty() && languages.is_empty() {
            return Err("mandate scope requires a search query, topic, or language".into());
        }
        if self.max_repositories == 0 || self.max_repositories > 25 {
            return Err("max_repositories must be between 1 and 25".into());
        }
        if !(5..=100).contains(&self.issues_per_repository) {
            return Err("issues_per_repository must be between 5 and 100".into());
        }
        if self.stale_days == 0 || self.stale_days > 730 {
            return Err("stale_days must be between 1 and 730".into());
        }
        if self.min_stars > 1_000_000 {
            return Err("min_stars must not exceed 1000000".into());
        }
        Ok(Self {
            search_query,
            topics,
            languages,
            min_stars: self.min_stars,
            max_repositories: self.max_repositories,
            issues_per_repository: self.issues_per_repository,
            stale_days: self.stale_days,
        })
    }

    fn signal_hive_input(&self) -> Value {
        serde_json::json!({
            "search_query": self.search_query,
            "topics": self.topics,
            "languages": self.languages,
            "min_stars": self.min_stars,
            "max_repos": self.max_repositories,
            "issues_per_repo": self.issues_per_repository,
            "stale_days": self.stale_days,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MandateLimits {
    pub pr_budget: u32,
    pub cost_budget_cents_per_day: u64,
    pub per_owner_open_prs: u32,
    pub cooldown_after_close_days: u32,
}

impl MandateLimits {
    fn validated(self) -> Result<Self, String> {
        if self.pr_budget > 100 {
            return Err("pr_budget must not exceed 100".into());
        }
        if self.cost_budget_cents_per_day > 1_000_000 {
            return Err("cost_budget_cents_per_day must not exceed 1000000".into());
        }
        if self.per_owner_open_prs == 0 || self.per_owner_open_prs > 20 {
            return Err("per_owner_open_prs must be between 1 and 20".into());
        }
        if self.cooldown_after_close_days > 365 {
            return Err("cooldown_after_close_days must not exceed 365".into());
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveMandateRequest {
    pub name: String,
    pub objective: String,
    pub scope: MandateScope,
    pub requested_autonomy: MandateAutonomy,
    pub limits: MandateLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateMandateRequest {
    pub expected_revision: u64,
    pub mandate: SaveMandateRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MandateConfig {
    pub name: String,
    pub objective: String,
    pub scope: MandateScope,
    pub requested_autonomy: MandateAutonomy,
    pub limits: MandateLimits,
}

impl MandateConfig {
    pub fn from_request(request: SaveMandateRequest) -> Result<Self, String> {
        Self {
            name: request.name,
            objective: request.objective,
            scope: request.scope,
            requested_autonomy: request.requested_autonomy,
            limits: request.limits,
        }
        .validated()
    }

    pub fn validated(self) -> Result<Self, String> {
        Ok(Self {
            name: required("name", self.name, 120)?,
            objective: required("objective", self.objective, 2_000)?,
            scope: self.scope.normalized()?,
            requested_autonomy: self.requested_autonomy,
            limits: self.limits.validated()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum MandateLifecycle {
    Active {
        activated_at: String,
    },
    Paused {
        paused_at: String,
        reason: String,
    },
    Archived {
        archived_at: String,
        reason: String,
    },
    Unknown {
        raw_state: String,
        raw_evidence: Value,
    },
}

impl MandateLifecycle {
    pub const fn kind(&self) -> &str {
        match self {
            Self::Active { .. } => "active",
            Self::Paused { .. } => "paused",
            Self::Archived { .. } => "archived",
            Self::Unknown { .. } => "unknown",
        }
    }

    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    pub fn from_storage(raw_state: String, raw_evidence: Value) -> Self {
        let parsed = serde_json::from_value::<Self>(raw_evidence.clone());
        match parsed {
            Ok(value) if value.kind() == raw_state => value,
            _ => Self::Unknown {
                raw_state,
                raw_evidence,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MandateRecord {
    pub id: String,
    pub config: MandateConfig,
    pub lifecycle: MandateLifecycle,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl MandateRecord {
    pub fn active(config: MandateConfig) -> Self {
        let now = now_rfc3339();
        Self {
            id: format!("mandate_{}", Uuid::now_v7()),
            config,
            lifecycle: MandateLifecycle::Active {
                activated_at: now.clone(),
            },
            revision: 1,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MandateReasonRequest {
    pub reason: String,
}

impl MandateReasonRequest {
    pub fn validated(self) -> Result<String, String> {
        required("reason", self.reason, 1_000)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum ConductorDecision {
    Deferred {
        mandate_id: String,
        reason: String,
    },
    ObservedOnly {
        mandate_id: String,
        requested_autonomy: MandateAutonomy,
        reason: String,
    },
    PlannedDiscovery {
        mandate_id: String,
        requested_autonomy: MandateAutonomy,
        effective_autonomy: MandateAutonomy,
        proposed_dispatch: ProposedDispatch,
        rationale: String,
    },
}

impl ConductorDecision {
    pub fn for_mandate(mandate: &MandateRecord) -> Self {
        if !mandate.lifecycle.is_active() {
            return Self::Deferred {
                mandate_id: mandate.id.clone(),
                reason: "Mandate lifecycle evidence is not an active state.".into(),
            };
        }
        if mandate.config.requested_autonomy == MandateAutonomy::Observe {
            return Self::ObservedOnly {
                mandate_id: mandate.id.clone(),
                requested_autonomy: MandateAutonomy::Observe,
                reason: "Observe autonomy records intent without proposing a product action."
                    .into(),
            };
        }
        Self::PlannedDiscovery {
            mandate_id: mandate.id.clone(),
            requested_autonomy: mandate.config.requested_autonomy,
            effective_autonomy: mandate.config.requested_autonomy.effective_now(),
            proposed_dispatch: ProposedDispatch {
                product_slug: "signal-hive".into(),
                action_id: "scan".into(),
                input: mandate.config.scope.signal_hive_input(),
            },
            rationale: format!(
                "Ask SignalHive to discover evidence for mandate: {}",
                mandate.config.objective
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConductorTickTrigger {
    Operator,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ConductorTickLifecycle {
    Running {
        started_at: String,
        lease_until: String,
    },
    Completed {
        started_at: String,
        finished_at: String,
        decisions: Vec<ConductorDecision>,
        remaining_active_mandates: u32,
    },
    Failed {
        started_at: String,
        failed_at: String,
        reason: String,
    },
    Unknown {
        raw_state: String,
        raw_evidence: Value,
    },
}

impl ConductorTickLifecycle {
    pub const fn kind(&self) -> &str {
        match self {
            Self::Running { .. } => "running",
            Self::Completed { .. } => "completed",
            Self::Failed { .. } => "failed",
            Self::Unknown { .. } => "unknown",
        }
    }

    pub fn from_storage(raw_state: String, raw_evidence: Value) -> Self {
        let parsed = serde_json::from_value::<Self>(raw_evidence.clone());
        match parsed {
            Ok(value) if value.kind() == raw_state => value,
            _ => Self::Unknown {
                raw_state,
                raw_evidence,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConductorTickRecord {
    pub id: String,
    pub trigger: ConductorTickTrigger,
    pub lifecycle: ConductorTickLifecycle,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RunConductorTickOutcome {
    Settled {
        tick: ConductorTickRecord,
    },
    Busy {
        active_tick_id: String,
        lease_until: String,
    },
}

fn normalized_terms(field: &str, values: Vec<String>) -> Result<Vec<String>, String> {
    let mut values = values
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    if values.len() > 25 {
        return Err(format!("{field} must contain at most 25 values"));
    }
    if values.iter().any(|value| value.len() > 80) {
        return Err(format!("each {field} value must be at most 80 characters"));
    }
    Ok(values)
}

static BACKGROUND_LOOP_STARTED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

pub fn start_background_loop() {
    if BACKGROUND_LOOP_STARTED.set(()).is_err() {
        return;
    }
    tokio::spawn(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(
                crate::db::conductor_interval_seconds(),
            ))
            .await;
            match crate::db::run_conductor_tick(ConductorTickTrigger::Background) {
                Ok(RunConductorTickOutcome::Settled { tick }) => {
                    tracing::debug!(tick_id = %tick.id, "proposal-only conductor tick settled");
                }
                Ok(RunConductorTickOutcome::Busy {
                    active_tick_id,
                    lease_until,
                }) => {
                    tracing::debug!(%active_tick_id, %lease_until, "conductor tick skipped because another writer holds the lease");
                }
                Err(error) => {
                    tracing::error!(%error, "background conductor tick failed");
                }
            }
        }
    });
}

fn required(field: &str, value: String, max_len: usize) -> Result<String, String> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(format!("{field} is required"));
    }
    if value.len() > max_len {
        return Err(format!("{field} must be at most {max_len} characters"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request(repository: &str, product: &str) -> ProposeWorkRequest {
        ProposeWorkRequest {
            mandate_id: None,
            identity: WorkIdentity {
                kind: " GitHub-Issue ".into(),
                repository: repository.into(),
                subject_ref: "issue:42".into(),
            },
            proposed_dispatch: ProposedDispatch {
                product_slug: product.into(),
                action_id: "analyze".into(),
                input: json!({"repository": repository}),
            },
            origin: WorkOrigin::Operator,
            rationale: "Worth assessing".into(),
        }
    }

    #[test]
    fn fingerprint_converges_across_case_and_proposed_products() {
        let first = WorkProposal::from_request(request("NousResearch/Hermes-Agent", "signal-hive"))
            .expect("valid proposal");
        let second =
            WorkProposal::from_request(request("nousresearch/hermes-agent", "repo-reaper"))
                .expect("valid proposal");
        assert_eq!(first.identity.fingerprint(), second.identity.fingerprint());
    }

    #[test]
    fn malformed_or_future_lifecycle_is_unknown() {
        let lifecycle = WorkLifecycle::from_storage(
            "ready".into(),
            json!({"state": "ready", "ready_at": "later"}),
        );
        assert!(matches!(lifecycle, WorkLifecycle::Unknown { .. }));
    }

    #[test]
    fn proposal_rejects_non_object_dispatch_input() {
        let mut value = request("owner/repo", "signal-hive");
        value.proposed_dispatch.input = json!(["not", "an", "object"]);
        assert_eq!(
            WorkProposal::from_request(value).expect_err("must reject array input"),
            "proposed dispatch input must be a JSON object"
        );
    }
}

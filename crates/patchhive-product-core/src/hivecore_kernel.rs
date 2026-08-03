//! Pure HiveCore authority types and fail-closed evaluators.
//!
//! Persistence and transport remain owned by HiveCore. This module is the kernel:
//! products and the conductor share one interpretation of pauses, resource evidence,
//! and smoke-earned autonomy instead of reconstructing authority from booleans.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentTopology {
    UnifiedInProcess,
    GatewayCompatibility,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum PauseTarget {
    Suite,
    Product { product_slug: String },
    Mandate { mandate_id: String },
    Repository { repository: String },
}

impl PauseTarget {
    pub fn storage_key(&self) -> String {
        match self {
            Self::Suite => "suite".into(),
            Self::Product { product_slug } => format!("product:{}", product_slug.trim()),
            Self::Mandate { mandate_id } => format!("mandate:{}", mandate_id.trim()),
            Self::Repository { repository } => {
                format!("repository:{}", repository.trim().to_ascii_lowercase())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DrainState {
    Draining {
        observed_in_flight: u32,
        checked_at: String,
    },
    Drained {
        drained_at: String,
    },
    Unknown {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PauseLifecycle {
    Running {
        resumed_at: String,
    },
    Paused {
        paused_at: String,
        reason: String,
        drain: DrainState,
    },
    Unknown {
        raw_state: String,
        raw_evidence: serde_json::Value,
    },
}

impl PauseLifecycle {
    pub const fn kind(&self) -> &str {
        match self {
            Self::Running { .. } => "running",
            Self::Paused { .. } => "paused",
            Self::Unknown { .. } => "unknown",
        }
    }

    pub const fn blocks_new_work(&self) -> bool {
        !matches!(self, Self::Running { .. })
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PauseRecord {
    pub target: PauseTarget,
    pub lifecycle: PauseLifecycle,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Evidence<T> {
    Observed { value: T, observed_at: String },
    Failed { reason: String },
    NotObserved { reason: String },
    NotApplicable { reason: String },
}

impl<T> Evidence<T> {
    pub fn observed_value(&self) -> Option<&T> {
        match self {
            Self::Observed { value, .. } => Some(value),
            Self::Failed { .. } | Self::NotObserved { .. } | Self::NotApplicable { .. } => None,
        }
    }

    pub fn unavailable_reason(&self) -> Option<&str> {
        match self {
            Self::Observed { .. } => None,
            Self::Failed { reason }
            | Self::NotObserved { reason }
            | Self::NotApplicable { reason } => Some(reason),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GithubRateEvidence {
    pub limit: u32,
    pub remaining: u32,
    pub reset_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiSpendEvidence {
    pub daily_limit_cents: u64,
    pub spent_cents: u64,
    pub reserved_cents: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mandate_daily_limit_cents: Option<u64>,
    pub mandate_spent_cents: u64,
    pub mandate_reserved_cents: u64,
    pub day: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxEvidence {
    pub slots: u32,
    pub in_use: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnerPolitenessEvidence {
    pub owner: String,
    pub open_pull_requests: u32,
    pub limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdmissionEvidence {
    pub github_rate: Evidence<GithubRateEvidence>,
    pub ai_spend: Evidence<AiSpendEvidence>,
    pub sandbox: Evidence<SandboxEvidence>,
    pub owner_politeness: Evidence<OwnerPolitenessEvidence>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdmissionRequirements {
    pub github_rate: bool,
    pub ai_spend: bool,
    pub sandbox: bool,
    pub owner_politeness: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourcePolicy {
    pub github_min_remaining: u32,
    pub suite_ai_daily_limit_cents: u64,
    pub sandbox_slots: u32,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AiBudgetReservationState {
    Reserved {
        expires_at: String,
    },
    Committed {
        actual_cents: u64,
    },
    Released {
        reason: String,
    },
    Expired {
        expired_at: String,
    },
    Unknown {
        raw_state: String,
        raw_evidence: serde_json::Value,
    },
}

impl AiBudgetReservationState {
    pub const fn kind(&self) -> &str {
        match self {
            Self::Reserved { .. } => "reserved",
            Self::Committed { .. } => "committed",
            Self::Released { .. } => "released",
            Self::Expired { .. } => "expired",
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiBudgetReservation {
    pub id: String,
    pub work_item_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mandate_id: Option<String>,
    pub reserved_cents: u64,
    pub lifecycle: AiBudgetReservationState,
    pub day: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SandboxLeaseState {
    Claimed {
        expires_at: String,
    },
    Released {
        reason: String,
    },
    Expired {
        expired_at: String,
    },
    Unknown {
        raw_state: String,
        raw_evidence: serde_json::Value,
    },
}

impl SandboxLeaseState {
    pub const fn kind(&self) -> &str {
        match self {
            Self::Claimed { .. } => "claimed",
            Self::Released { .. } => "released",
            Self::Expired { .. } => "expired",
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxLease {
    pub id: String,
    pub work_item_id: String,
    pub lifecycle: SandboxLeaseState,
    pub created_at: String,
    pub updated_at: String,
}

impl ResourcePolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.github_min_remaining > 5_000 {
            return Err("github_min_remaining must not exceed 5000".into());
        }
        if self.suite_ai_daily_limit_cents > 100_000_000 {
            return Err("suite_ai_daily_limit_cents must not exceed 100000000".into());
        }
        if self.sandbox_slots == 0 || self.sandbox_slots > 100 {
            return Err("sandbox_slots must be between 1 and 100".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionLimitingLayer {
    PauseAuthority,
    GithubRate,
    AiSpend,
    Sandbox,
    OwnerPoliteness,
    SmokeAuthority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdmissionDenial {
    pub limiting_layers: Vec<AdmissionLimitingLayer>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AdmissionDecision {
    Admitted { checked_at: String },
    Denied { denial: AdmissionDenial },
}

/// Fail closed on missing, failed, malformed, exhausted, or cooled-down evidence.
pub fn evaluate_resource_admission(
    evidence: &AdmissionEvidence,
    requirements: AdmissionRequirements,
    github_min_remaining: u32,
    requested_ai_cents: u64,
    checked_at: String,
) -> AdmissionDecision {
    let mut limiting_layers = Vec::new();
    let mut reasons = Vec::new();

    if requirements.github_rate {
        match &evidence.github_rate {
            Evidence::Observed { value, .. }
                if value.remaining >= github_min_remaining && value.remaining <= value.limit => {}
            Evidence::Observed { value, .. } if value.remaining > value.limit => {
                limiting_layers.push(AdmissionLimitingLayer::GithubRate);
                reasons
                    .push("GitHub rate evidence is contradictory: remaining exceeds limit.".into());
            }
            Evidence::Observed { value, .. } => {
                limiting_layers.push(AdmissionLimitingLayer::GithubRate);
                reasons.push(format!(
                    "GitHub rate headroom is {} and the reserved floor is {}.",
                    value.remaining, github_min_remaining
                ));
            }
            unavailable => {
                limiting_layers.push(AdmissionLimitingLayer::GithubRate);
                reasons.push(format!(
                    "GitHub rate evidence is unavailable: {}",
                    unavailable
                        .unavailable_reason()
                        .unwrap_or("unknown evidence state")
                ));
            }
        }
    }

    if requirements.ai_spend {
        match &evidence.ai_spend {
            Evidence::Observed { value, .. } => {
                let suite_total = value
                    .spent_cents
                    .saturating_add(value.reserved_cents)
                    .saturating_add(requested_ai_cents);
                let mandate_denial = value.mandate_daily_limit_cents.and_then(|limit| {
                    let total = value
                        .mandate_spent_cents
                        .saturating_add(value.mandate_reserved_cents)
                        .saturating_add(requested_ai_cents);
                    (total > limit).then_some((total, limit))
                });
                if suite_total > value.daily_limit_cents || mandate_denial.is_some() {
                    limiting_layers.push(AdmissionLimitingLayer::AiSpend);
                }
                if suite_total > value.daily_limit_cents {
                    reasons.push(format!(
                        "AI daily capacity cannot admit {requested_ai_cents} cents: the resulting {suite_total} cents exceeds the {}-cent limit.",
                        value.daily_limit_cents
                    ));
                }
                if let Some((total, limit)) = mandate_denial {
                    reasons.push(format!(
                        "Mandate AI capacity cannot admit {requested_ai_cents} cents: the resulting {total} cents exceeds the {limit}-cent limit."
                    ));
                }
            }
            unavailable => {
                limiting_layers.push(AdmissionLimitingLayer::AiSpend);
                reasons.push(format!(
                    "AI spend evidence is unavailable: {}",
                    unavailable
                        .unavailable_reason()
                        .unwrap_or("unknown evidence state")
                ));
            }
        }
    }

    if requirements.sandbox {
        match &evidence.sandbox {
            Evidence::Observed { value, .. } if value.in_use < value.slots => {}
            Evidence::Observed { value, .. } => {
                limiting_layers.push(AdmissionLimitingLayer::Sandbox);
                reasons.push(format!(
                    "Sandbox capacity is exhausted: {} of {} slots are in use.",
                    value.in_use, value.slots
                ));
            }
            unavailable => {
                limiting_layers.push(AdmissionLimitingLayer::Sandbox);
                reasons.push(format!(
                    "Sandbox evidence is unavailable: {}",
                    unavailable
                        .unavailable_reason()
                        .unwrap_or("unknown evidence state")
                ));
            }
        }
    }

    if requirements.owner_politeness {
        match &evidence.owner_politeness {
            Evidence::Observed { value, .. }
                if value.cooldown_until.is_none() && value.open_pull_requests < value.limit => {}
            Evidence::Observed { value, .. } => {
                limiting_layers.push(AdmissionLimitingLayer::OwnerPoliteness);
                reasons.push(match &value.cooldown_until {
                    Some(until) => format!(
                        "Repository owner {} is cooling down until {until}.",
                        value.owner
                    ),
                    None => format!(
                        "Repository owner {} has {} open PatchHive PRs and the limit is {}.",
                        value.owner, value.open_pull_requests, value.limit
                    ),
                });
            }
            unavailable => {
                limiting_layers.push(AdmissionLimitingLayer::OwnerPoliteness);
                reasons.push(format!(
                    "Owner-politeness evidence is unavailable: {}",
                    unavailable
                        .unavailable_reason()
                        .unwrap_or("unknown evidence state")
                ));
            }
        }
    }

    if limiting_layers.is_empty() {
        AdmissionDecision::Admitted { checked_at }
    } else {
        AdmissionDecision::Denied {
            denial: AdmissionDenial {
                limiting_layers,
                reasons,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    Observe,
    Propose,
    ActWithApproval,
    Act,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkOutcomeKind {
    Merged,
    ClosedUnmerged,
    StaleIgnored,
    Completed,
    Failed,
    Unknown,
}

impl WorkOutcomeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Merged => "merged",
            Self::ClosedUnmerged => "closed_unmerged",
            Self::StaleIgnored => "stale_ignored",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReputationSummary {
    pub shipped: u32,
    pub merged: u32,
    pub closed_unmerged: u32,
    pub stale_ignored: u32,
    pub rolling_decisions: u32,
    pub rolling_rejections: u32,
    pub slowdown_active: bool,
    pub evaluated_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum SmokeTier {
    FirstStack,
    ReadOnlyFleet,
    WriteDryRun,
    ReleaseGate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmokeProof {
    pub run_id: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SmokeAuthority {
    pub first_stack: Evidence<SmokeProof>,
    pub read_only_fleet: Evidence<SmokeProof>,
    pub write_dry_run: Evidence<SmokeProof>,
    pub release_gate: Evidence<SmokeProof>,
}

impl SmokeAuthority {
    pub fn tier_passed(&self, tier: SmokeTier) -> bool {
        match tier {
            SmokeTier::FirstStack => self.first_stack.observed_value().is_some(),
            SmokeTier::ReadOnlyFleet => self.read_only_fleet.observed_value().is_some(),
            SmokeTier::WriteDryRun => self.write_dry_run.observed_value().is_some(),
            SmokeTier::ReleaseGate => self.release_gate.observed_value().is_some(),
        }
    }

    pub fn earned_autonomy(&self) -> AutonomyLevel {
        if !self.tier_passed(SmokeTier::FirstStack) {
            return AutonomyLevel::Observe;
        }
        if !self.tier_passed(SmokeTier::ReadOnlyFleet) {
            return AutonomyLevel::Observe;
        }
        if !self.tier_passed(SmokeTier::WriteDryRun) {
            return AutonomyLevel::Propose;
        }
        if !self.tier_passed(SmokeTier::ReleaseGate) {
            return AutonomyLevel::ActWithApproval;
        }
        AutonomyLevel::Act
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutonomyDecision {
    pub requested: AutonomyLevel,
    pub earned: AutonomyLevel,
    pub effective: AutonomyLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demotion_reason: Option<String>,
}

pub fn evaluate_autonomy(requested: AutonomyLevel, smoke: &SmokeAuthority) -> AutonomyDecision {
    let earned = smoke.earned_autonomy();
    let effective = requested.min(earned);
    AutonomyDecision {
        requested,
        earned,
        effective,
        demotion_reason: (effective < requested).then(|| {
            format!(
                "Requested {requested:?} autonomy was demoted to {effective:?} by durable smoke evidence."
            )
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed<T>(value: T) -> Evidence<T> {
        Evidence::Observed {
            value,
            observed_at: "2026-08-03T00:00:00Z".into(),
        }
    }

    #[test]
    fn resource_admission_fails_closed_when_any_evidence_is_missing() {
        let decision = evaluate_resource_admission(
            &AdmissionEvidence {
                github_rate: Evidence::NotObserved {
                    reason: "no rate response".into(),
                },
                ai_spend: observed(AiSpendEvidence {
                    daily_limit_cents: 100,
                    spent_cents: 1,
                    reserved_cents: 0,
                    mandate_daily_limit_cents: Some(50),
                    mandate_spent_cents: 1,
                    mandate_reserved_cents: 0,
                    day: "2026-08-03".into(),
                }),
                sandbox: observed(SandboxEvidence {
                    slots: 2,
                    in_use: 0,
                }),
                owner_politeness: observed(OwnerPolitenessEvidence {
                    owner: "patchhive".into(),
                    open_pull_requests: 0,
                    limit: 1,
                    cooldown_until: None,
                }),
            },
            AdmissionRequirements {
                github_rate: true,
                ai_spend: true,
                sandbox: true,
                owner_politeness: true,
            },
            100,
            1,
            "2026-08-03T00:00:00Z".into(),
        );
        assert!(matches!(
            decision,
            AdmissionDecision::Denied {
                denial: AdmissionDenial { limiting_layers, .. }
            } if limiting_layers == vec![AdmissionLimitingLayer::GithubRate]
        ));
    }

    #[test]
    fn ai_admission_reports_only_the_exhausted_budget_layer() {
        let decision = evaluate_resource_admission(
            &AdmissionEvidence {
                github_rate: Evidence::NotApplicable {
                    reason: "not required".into(),
                },
                ai_spend: observed(AiSpendEvidence {
                    daily_limit_cents: 1_000,
                    spent_cents: 100,
                    reserved_cents: 100,
                    mandate_daily_limit_cents: Some(250),
                    mandate_spent_cents: 100,
                    mandate_reserved_cents: 100,
                    day: "2026-08-03".into(),
                }),
                sandbox: Evidence::NotApplicable {
                    reason: "not required".into(),
                },
                owner_politeness: Evidence::NotApplicable {
                    reason: "not required".into(),
                },
            },
            AdmissionRequirements {
                github_rate: false,
                ai_spend: true,
                sandbox: false,
                owner_politeness: false,
            },
            0,
            51,
            "2026-08-03T00:00:00Z".into(),
        );
        let AdmissionDecision::Denied { denial } = decision else {
            panic!("mandate budget must deny the request");
        };
        assert_eq!(
            denial.limiting_layers,
            vec![AdmissionLimitingLayer::AiSpend]
        );
        assert_eq!(denial.reasons.len(), 1);
        assert!(denial.reasons[0].starts_with("Mandate AI capacity"));
    }

    #[test]
    fn ai_admission_allows_an_exact_remaining_capacity_match() {
        let decision = evaluate_resource_admission(
            &AdmissionEvidence {
                github_rate: Evidence::NotApplicable {
                    reason: "not required".into(),
                },
                ai_spend: observed(AiSpendEvidence {
                    daily_limit_cents: 300,
                    spent_cents: 100,
                    reserved_cents: 100,
                    mandate_daily_limit_cents: Some(300),
                    mandate_spent_cents: 100,
                    mandate_reserved_cents: 100,
                    day: "2026-08-03".into(),
                }),
                sandbox: Evidence::NotApplicable {
                    reason: "not required".into(),
                },
                owner_politeness: Evidence::NotApplicable {
                    reason: "not required".into(),
                },
            },
            AdmissionRequirements {
                github_rate: false,
                ai_spend: true,
                sandbox: false,
                owner_politeness: false,
            },
            0,
            100,
            "2026-08-03T00:00:00Z".into(),
        );
        assert!(matches!(decision, AdmissionDecision::Admitted { .. }));
    }

    #[test]
    fn smoke_authority_demotes_act_to_the_highest_proven_level() {
        let unavailable = || Evidence::NotObserved {
            reason: "tier has not passed".into(),
        };
        let smoke = SmokeAuthority {
            first_stack: observed(SmokeProof {
                run_id: "first".into(),
                finished_at: "2026-08-03T00:00:00Z".into(),
            }),
            read_only_fleet: observed(SmokeProof {
                run_id: "read".into(),
                finished_at: "2026-08-03T00:00:00Z".into(),
            }),
            write_dry_run: unavailable(),
            release_gate: unavailable(),
        };
        let decision = evaluate_autonomy(AutonomyLevel::Act, &smoke);
        assert_eq!(decision.effective, AutonomyLevel::Propose);
        assert!(decision.demotion_reason.is_some());
    }

    #[test]
    fn unknown_pause_state_blocks_new_work() {
        assert!(PauseLifecycle::Unknown {
            raw_state: "future".into(),
            raw_evidence: serde_json::json!({}),
        }
        .blocks_new_work());
    }
}

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

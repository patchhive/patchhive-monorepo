use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::contract::{ActionEffect, DispatchActionInput, ProductAction};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "origin", rename_all = "snake_case")]
pub enum ApprovalOrigin {
    OperatorDispatch,
    SuiteRun { run_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalSubject {
    pub fingerprint: String,
    pub product: String,
    pub action_id: String,
    pub action_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub input_hash: String,
    pub effect: ActionEffect,
    pub required_scopes: Vec<String>,
    pub origin: ApprovalOrigin,
}

impl ApprovalSubject {
    pub fn for_dispatch(
        product: impl Into<String>,
        action: &ProductAction,
        dispatch: &DispatchActionInput,
        repository: Option<String>,
        run_id: Option<String>,
        origin: ApprovalOrigin,
    ) -> Self {
        let product = product.into();
        let input_hash = dispatch_input_hash(dispatch);
        let mut required_scopes = action.required_scopes.clone();
        required_scopes.sort();
        required_scopes.dedup();
        let material = ApprovalSubjectMaterial {
            product: &product,
            action_id: &action.id,
            action_label: &action.label,
            repository: repository.as_deref(),
            run_id: run_id.as_deref(),
            input_hash: &input_hash,
            effect: action.effect,
            required_scopes: &required_scopes,
            origin: &origin,
        };
        let fingerprint = hash_serializable(&material);
        Self {
            fingerprint,
            product,
            action_id: action.id.clone(),
            action_label: action.label.clone(),
            repository,
            run_id,
            input_hash,
            effect: action.effect,
            required_scopes,
            origin,
        }
    }
}

#[derive(Serialize)]
struct ApprovalSubjectMaterial<'a> {
    product: &'a str,
    action_id: &'a str,
    action_label: &'a str,
    repository: Option<&'a str>,
    run_id: Option<&'a str>,
    input_hash: &'a str,
    effect: ActionEffect,
    required_scopes: &'a [String],
    origin: &'a ApprovalOrigin,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalExpirableState {
    Pending,
    Granted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ApprovalConsumptionOutcome {
    Accepted {
        remote_status: u16,
    },
    Rejected {
        #[serde(skip_serializing_if = "Option::is_none")]
        remote_status: Option<u16>,
        reason: String,
    },
    Uncertain {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ApprovalState {
    Pending {
        expires_at: String,
    },
    Granted {
        granted_at: String,
        expires_at: String,
    },
    Denied {
        denied_at: String,
        reason: String,
    },
    Revoked {
        revoked_at: String,
        reason: String,
    },
    Consuming {
        claimed_at: String,
    },
    Consumed {
        claimed_at: String,
        consumed_at: String,
        event_id: String,
        outcome: ApprovalConsumptionOutcome,
    },
    Expired {
        expired_at: String,
        previous: ApprovalExpirableState,
    },
    Unknown {
        raw_state: String,
        raw_evidence: Value,
    },
}

impl ApprovalState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending { .. } => "pending",
            Self::Granted { .. } => "granted",
            Self::Denied { .. } => "denied",
            Self::Revoked { .. } => "revoked",
            Self::Consuming { .. } => "consuming",
            Self::Consumed { .. } => "consumed",
            Self::Expired { .. } => "expired",
            Self::Unknown { .. } => "unknown",
        }
    }

    pub fn expires_at(&self) -> Option<&str> {
        match self {
            Self::Pending { expires_at } | Self::Granted { expires_at, .. } => Some(expires_at),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApprovalRecord {
    pub id: String,
    pub subject: ApprovalSubject,
    pub dispatch: DispatchActionInput,
    pub lifecycle: ApprovalState,
    pub created_at: String,
    pub updated_at: String,
    pub history: Vec<ApprovalEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalEvent {
    pub id: i64,
    pub approval_id: String,
    pub event: String,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalReasonRequest {
    pub reason: String,
}

pub fn dispatch_input_hash(dispatch: &DispatchActionInput) -> String {
    let canonical = Value::Object(
        [
            (
                "path_params".to_string(),
                canonical_string_map(&dispatch.path_params),
            ),
            ("payload".to_string(), canonical_json(&dispatch.payload)),
            ("query".to_string(), canonical_string_map(&dispatch.query)),
        ]
        .into_iter()
        .collect(),
    );
    hash_bytes(&serde_json::to_vec(&canonical).expect("canonical JSON values always serialize"))
}

fn canonical_string_map(values: &std::collections::HashMap<String, String>) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect(),
    )
}

/// Recursively sort JSON object keys so hashes describe semantic JSON rather
/// than the insertion order chosen by one serializer or caller.
pub fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| (key.clone(), canonical_json(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn hash_serializable(value: &impl Serialize) -> String {
    hash_bytes(&serde_json::to_vec(value).expect("approval subject material always serializes"))
}

fn hash_bytes(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::{dispatch_input_hash, ApprovalOrigin, ApprovalSubject};
    use crate::contract::{self, ActionEffect, ActionSafety, DispatchActionInput};

    fn action() -> contract::ProductAction {
        contract::action(
            "run",
            "Run patch hunt",
            "POST",
            "/run",
            "Open validated pull requests.",
            true,
            ActionSafety::operator_required(ActionEffect::MutatesRepository {
                opens_pull_request: true,
            }),
        )
        .credential_requirements(["runs:write", "pull_requests:write"])
    }

    #[test]
    fn input_hash_is_stable_across_map_and_object_order() {
        let mut first_query = HashMap::new();
        first_query.insert("page".into(), "1".into());
        first_query.insert("sort".into(), "new".into());
        let mut second_query = HashMap::new();
        second_query.insert("sort".into(), "new".into());
        second_query.insert("page".into(), "1".into());
        let first = DispatchActionInput {
            payload: json!({"repo": "patchhive/example", "limits": {"issues": 3, "repos": 1}}),
            path_params: HashMap::new(),
            query: first_query,
        };
        let second = DispatchActionInput {
            payload: json!({"limits": {"repos": 1, "issues": 3}, "repo": "patchhive/example"}),
            path_params: HashMap::new(),
            query: second_query,
        };

        assert_eq!(dispatch_input_hash(&first), dispatch_input_hash(&second));
    }

    #[test]
    fn altered_dispatch_or_origin_changes_the_approval_subject() {
        let first = contract::parse_dispatch_input(json!({"repo": "patchhive/example"}));
        let second = contract::parse_dispatch_input(json!({"repo": "patchhive/other"}));
        let operator = ApprovalSubject::for_dispatch(
            "repo-reaper",
            &action(),
            &first,
            Some("patchhive/example".into()),
            None,
            ApprovalOrigin::OperatorDispatch,
        );
        let altered = ApprovalSubject::for_dispatch(
            "repo-reaper",
            &action(),
            &second,
            Some("patchhive/other".into()),
            None,
            ApprovalOrigin::OperatorDispatch,
        );
        let suite = ApprovalSubject::for_dispatch(
            "repo-reaper",
            &action(),
            &first,
            Some("patchhive/example".into()),
            Some("srun_1".into()),
            ApprovalOrigin::SuiteRun {
                run_id: "srun_1".into(),
            },
        );

        assert_ne!(operator.fingerprint, altered.fingerprint);
        assert_ne!(operator.fingerprint, suite.fingerprint);
    }
}

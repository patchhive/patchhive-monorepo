use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SmokeTier {
    FirstStack,
    ReadOnlyFleet,
    WriteDryRun,
    ReleaseGate,
}

impl SmokeTier {
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "first-stack" => Some(Self::FirstStack),
            "read-only-fleet" => Some(Self::ReadOnlyFleet),
            "write-dry-run" => Some(Self::WriteDryRun),
            "release-gate" => Some(Self::ReleaseGate),
            _ => None,
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::FirstStack => "first-stack",
            Self::ReadOnlyFleet => "read-only-fleet",
            Self::WriteDryRun => "write-dry-run",
            Self::ReleaseGate => "release-gate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmokeActionManifest {
    pub id: String,
    #[serde(default)]
    pub payload: Value,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StartupCheckIdentity {
    pub code: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProductSmokeManifest {
    #[serde(default)]
    pub tiers: Vec<SmokeTier>,
    #[serde(default)]
    pub action: Option<SmokeActionManifest>,
    #[serde(default)]
    pub acknowledged_startup: Vec<StartupCheckIdentity>,
}

impl ProductSmokeManifest {
    pub fn validate(&self, product: &str) -> Result<()> {
        let mut tiers = HashSet::new();
        for tier in &self.tiers {
            anyhow::ensure!(
                tiers.insert(*tier),
                "product '{product}' declares duplicate smoke tier '{}'",
                tier.slug()
            );
        }
        let dispatches_action = self
            .tiers
            .iter()
            .any(|tier| !matches!(tier, SmokeTier::ReadOnlyFleet));
        anyhow::ensure!(
            !dispatches_action || self.action.is_some(),
            "product '{product}' smoke tiers require an action declaration"
        );
        anyhow::ensure!(
            self.action.is_none() || !self.tiers.is_empty(),
            "product '{product}' declares a smoke action without a tier"
        );
        if let Some(action) = &self.action {
            anyhow::ensure!(
                !action.id.trim().is_empty(),
                "product '{product}' smoke action ID is empty"
            );
            anyhow::ensure!(
                (1..=300).contains(&action.timeout_seconds),
                "product '{product}' smoke timeout must be between 1 and 300 seconds"
            );
        }
        let mut acknowledged = HashSet::new();
        for identity in &self.acknowledged_startup {
            anyhow::ensure!(
                !identity.code.trim().is_empty() && !identity.status.trim().is_empty(),
                "product '{product}' has an incomplete acknowledged startup identity"
            );
            anyhow::ensure!(
                acknowledged.insert(identity),
                "product '{product}' repeats acknowledged startup identity '{}:{}'",
                identity.code,
                identity.status
            );
        }
        Ok(())
    }

    pub fn participates_in(&self, tier: SmokeTier) -> bool {
        self.tiers.contains(&tier)
    }

    pub fn acknowledges(&self, code: Option<&str>, status: Option<&str>) -> bool {
        self.acknowledged_startup.iter().any(|identity| {
            code == Some(identity.code.as_str()) && status == Some(identity.status.as_str())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ProductSmokeManifest, SmokeActionManifest, SmokeTier, StartupCheckIdentity};
    use serde_json::json;

    #[test]
    fn action_tiers_require_a_typed_action() {
        let manifest = ProductSmokeManifest {
            tiers: vec![SmokeTier::FirstStack],
            ..ProductSmokeManifest::default()
        };
        assert!(manifest.validate("signal-hive").is_err());
    }

    #[test]
    fn identities_match_code_and_status_not_message_text() {
        let manifest = ProductSmokeManifest {
            tiers: vec![SmokeTier::WriteDryRun],
            action: Some(SmokeActionManifest {
                id: "dry_run".into(),
                payload: json!({}),
                timeout_seconds: 45,
            }),
            acknowledged_startup: vec![StartupCheckIdentity {
                code: "webhook_secret".into(),
                status: "missing".into(),
            }],
        };
        manifest.validate("repo-reaper").unwrap();
        assert!(manifest.acknowledges(Some("webhook_secret"), Some("missing")));
        assert!(!manifest.acknowledges(Some("webhook_secret"), Some("invalid")));
        assert!(!manifest.acknowledges(None, None));
    }
}

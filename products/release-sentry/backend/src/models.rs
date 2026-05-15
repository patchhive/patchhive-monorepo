use serde::{Deserialize, Serialize};

fn default_run_limit() -> u32 {
    20
}

fn default_changelog_path() -> String {
    "CHANGELOG.md".into()
}

fn default_blocker_labels() -> Vec<String> {
    vec![
        "release-blocker".into(),
        "blocker".into(),
        "critical".into(),
        "regression".into(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCheckRequest {
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub target_version: String,
    #[serde(default)]
    pub target_tag: String,
    #[serde(default = "default_changelog_path")]
    pub changelog_path: String,
    #[serde(default = "default_run_limit")]
    pub workflow_run_limit: u32,
    #[serde(default = "default_blocker_labels")]
    pub blocker_labels: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseReadinessMetrics {
    #[serde(default)]
    pub checks: u32,
    #[serde(default)]
    pub passed: u32,
    #[serde(default)]
    pub warned: u32,
    #[serde(default)]
    pub blocked: u32,
    #[serde(default)]
    pub workflow_runs: u32,
    #[serde(default)]
    pub workflow_successes: u32,
    #[serde(default)]
    pub workflow_failures: u32,
    #[serde(default)]
    pub workflow_pending: u32,
    #[serde(default)]
    pub release_blockers: u32,
    #[serde(default)]
    pub tags_seen: u32,
    #[serde(default)]
    pub releases_seen: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseEvidenceLink {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseCheck {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub links: Vec<ReleaseEvidenceLink>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseReadinessResult {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub target_version: String,
    #[serde(default)]
    pub target_tag: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub decision: String,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub metrics: ReleaseReadinessMetrics,
    #[serde(default)]
    pub checks: Vec<ReleaseCheck>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub target_version: String,
    #[serde(default)]
    pub target_tag: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub decision: String,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverviewCounts {
    #[serde(default)]
    pub runs: u32,
    #[serde(default)]
    pub repos: u32,
    #[serde(default)]
    pub ready: u32,
    #[serde(default)]
    pub watch: u32,
    #[serde(default)]
    pub hold: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverviewPayload {
    #[serde(default)]
    pub product: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub counts: OverviewCounts,
    #[serde(default)]
    pub recent_runs: Vec<HistoryItem>,
}

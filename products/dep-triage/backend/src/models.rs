use serde::{Deserialize, Serialize};

fn default_pr_limit() -> u32 {
    25
}

fn default_include_alerts() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanRequest {
    #[serde(default)]
    pub repo: String,
    #[serde(default = "default_pr_limit")]
    pub pr_limit: u32,
    #[serde(default = "default_include_alerts")]
    pub include_alerts: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyPullRef {
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub source_tool: String,
    #[serde(default)]
    pub from_version: String,
    #[serde(default)]
    pub to_version: String,
    #[serde(default)]
    pub update_kind: String,
    #[serde(default)]
    pub manifest_paths: Vec<String>,
    #[serde(default)]
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyAlertRef {
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub package_name: String,
    #[serde(default)]
    pub ecosystem: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub vulnerable_version_range: String,
    #[serde(default)]
    pub first_patched_version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyTriageItem {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub package_name: String,
    #[serde(default)]
    pub ecosystem: String,
    #[serde(default)]
    pub recommendation: String,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub update_kind: String,
    #[serde(default)]
    pub runtime_impact: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub manifests: Vec<String>,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub stale_days: u32,
    #[serde(default)]
    pub pull_requests: Vec<DependencyPullRef>,
    #[serde(default)]
    pub alerts: Vec<DependencyAlertRef>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriageMetrics {
    #[serde(default)]
    pub scanned_pull_requests: u32,
    #[serde(default)]
    pub dependency_pull_requests: u32,
    #[serde(default)]
    pub open_alerts: u32,
    #[serde(default)]
    pub tracked_items: u32,
    #[serde(default)]
    pub update_now: u32,
    #[serde(default)]
    pub watch: u32,
    #[serde(default)]
    pub ignore_for_now: u32,
    #[serde(default)]
    pub runtime_updates: u32,
    #[serde(default)]
    pub major_updates: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriageScanResult {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub metrics: TriageMetrics,
    #[serde(default)]
    pub items: Vec<DependencyTriageItem>,
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
    pub summary: String,
    #[serde(default)]
    pub tracked_items: u32,
    #[serde(default)]
    pub update_now: u32,
    #[serde(default)]
    pub watch: u32,
    #[serde(default)]
    pub ignore_for_now: u32,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverviewCounts {
    #[serde(default)]
    pub scans: u32,
    #[serde(default)]
    pub repos: u32,
    #[serde(default)]
    pub tracked_items: u32,
    #[serde(default)]
    pub update_now: u32,
    #[serde(default)]
    pub watch: u32,
    #[serde(default)]
    pub ignore_for_now: u32,
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
    pub recent_scans: Vec<HistoryItem>,
}

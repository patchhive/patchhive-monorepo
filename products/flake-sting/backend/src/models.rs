use serde::{Deserialize, Serialize};

fn default_lookback_runs() -> u32 {
    25
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanRequest {
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub workflow_name: String,
    #[serde(default = "default_lookback_runs")]
    pub lookback_runs: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlakeMetrics {
    #[serde(default)]
    pub workflow_runs: u32,
    #[serde(default)]
    pub completed_runs: u32,
    #[serde(default)]
    pub successful_runs: u32,
    #[serde(default)]
    pub failed_runs: u32,
    #[serde(default)]
    pub rerun_like_runs: u32,
    #[serde(default)]
    pub flaky_signals: u32,
    #[serde(default)]
    pub quarantine_candidates: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlakeSignal {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub workflow_name: String,
    #[serde(default)]
    pub job_name: String,
    #[serde(default)]
    pub step_name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub failure_count: u32,
    #[serde(default)]
    pub success_count: u32,
    #[serde(default)]
    pub rerun_hits: u32,
    #[serde(default)]
    pub environment_hints: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlakeScanResult {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub workflow_name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub metrics: FlakeMetrics,
    #[serde(default)]
    pub signals: Vec<FlakeSignal>,
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
    pub workflow_name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub flaky_signals: u32,
    #[serde(default)]
    pub quarantine_candidates: u32,
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
    pub flaky_signals: u32,
    #[serde(default)]
    pub quarantine_candidates: u32,
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

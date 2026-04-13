use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
pub struct FlakeTrend {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub compared_to_scan_id: String,
    #[serde(default)]
    pub compared_to_created_at: String,
    #[serde(default)]
    pub flaky_signal_delta: i32,
    #[serde(default)]
    pub quarantine_delta: i32,
    #[serde(default)]
    pub rerun_delta: i32,
    #[serde(default)]
    pub new_signal_count: u32,
    #[serde(default)]
    pub cleared_signal_count: u32,
    #[serde(default)]
    pub new_signals: Vec<String>,
    #[serde(default)]
    pub cleared_signals: Vec<String>,
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
    #[serde(default)]
    pub trend: Option<FlakeTrend>,
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
    #[serde(default)]
    pub trend: Option<FlakeTrend>,
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

fn signal_display(signal: &FlakeSignal) -> String {
    if signal.step_name.trim().is_empty() {
        format!("{} · {}", signal.workflow_name, signal.job_name)
    } else {
        format!(
            "{} · {} / {}",
            signal.workflow_name, signal.job_name, signal.step_name
        )
    }
}

pub fn compute_trend(
    current: &FlakeScanResult,
    previous: Option<&FlakeScanResult>,
) -> Option<FlakeTrend> {
    let previous = previous?;

    let current_labels = current
        .signals
        .iter()
        .map(|signal| (signal.key.clone(), signal_display(signal)))
        .collect::<BTreeMap<_, _>>();
    let previous_labels = previous
        .signals
        .iter()
        .map(|signal| (signal.key.clone(), signal_display(signal)))
        .collect::<BTreeMap<_, _>>();

    let new_signals = current_labels
        .iter()
        .filter(|(key, _)| !previous_labels.contains_key(*key))
        .map(|(_, label)| label.clone())
        .take(4)
        .collect::<Vec<_>>();
    let cleared_signals = previous_labels
        .iter()
        .filter(|(key, _)| !current_labels.contains_key(*key))
        .map(|(_, label)| label.clone())
        .take(4)
        .collect::<Vec<_>>();

    let flaky_signal_delta =
        current.metrics.flaky_signals as i32 - previous.metrics.flaky_signals as i32;
    let quarantine_delta = current.metrics.quarantine_candidates as i32
        - previous.metrics.quarantine_candidates as i32;
    let rerun_delta =
        current.metrics.rerun_like_runs as i32 - previous.metrics.rerun_like_runs as i32;

    let new_signal_count = current
        .signals
        .iter()
        .filter(|signal| !previous_labels.contains_key(&signal.key))
        .count() as u32;
    let cleared_signal_count = previous
        .signals
        .iter()
        .filter(|signal| !current_labels.contains_key(&signal.key))
        .count() as u32;

    let status = if flaky_signal_delta == 0
        && quarantine_delta == 0
        && rerun_delta == 0
        && new_signal_count == 0
        && cleared_signal_count == 0
    {
        "steady"
    } else if quarantine_delta > 0
        || flaky_signal_delta > 0
        || new_signal_count > cleared_signal_count
    {
        "rising"
    } else if quarantine_delta < 0
        || flaky_signal_delta < 0
        || cleared_signal_count > new_signal_count
    {
        "improving"
    } else {
        "shifted"
    };

    Some(FlakeTrend {
        status: status.into(),
        compared_to_scan_id: previous.id.clone(),
        compared_to_created_at: previous.created_at.clone(),
        flaky_signal_delta,
        quarantine_delta,
        rerun_delta,
        new_signal_count,
        cleared_signal_count,
        new_signals,
        cleared_signals,
    })
}

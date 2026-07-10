use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanRequest {
    #[serde(default)]
    pub repo: String,
    #[serde(default = "default_true")]
    pub include_code_scanning: bool,
    #[serde(default = "default_true")]
    pub include_dependency_alerts: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VulnerabilityFinding {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub recommendation: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub owner_hint: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub package_name: String,
    #[serde(default)]
    pub ecosystem: String,
    #[serde(default)]
    pub reachability: String,
    #[serde(default)]
    pub next_action: String,
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub identifiers: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VulnMetrics {
    #[serde(default)]
    pub code_scanning_alerts: u32,
    #[serde(default)]
    pub dependency_alerts: u32,
    #[serde(default)]
    pub tracked_findings: u32,
    #[serde(default)]
    pub fix_now: u32,
    #[serde(default)]
    pub plan_next: u32,
    #[serde(default)]
    pub watch: u32,
    #[serde(default, alias = "runtime_exposed")]
    pub runtime_scoped: u32,
    #[serde(default)]
    pub owner_scoped: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VulnScanResult {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub metrics: VulnMetrics,
    #[serde(default)]
    pub findings: Vec<VulnerabilityFinding>,
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
    pub tracked_findings: u32,
    #[serde(default)]
    pub fix_now: u32,
    #[serde(default)]
    pub plan_next: u32,
    #[serde(default)]
    pub watch: u32,
    #[serde(default)]
    pub code_scanning_alerts: u32,
    #[serde(default)]
    pub dependency_alerts: u32,
    #[serde(default, alias = "runtime_exposed")]
    pub runtime_scoped: u32,
    #[serde(default)]
    pub owner_scoped: u32,
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
    pub tracked_findings: u32,
    #[serde(default)]
    pub fix_now: u32,
    #[serde(default)]
    pub plan_next: u32,
    #[serde(default)]
    pub watch: u32,
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

#[cfg(test)]
mod tests {
    use super::VulnMetrics;

    #[test]
    fn runtime_scope_reads_legacy_scans_and_uses_the_precise_contract_name() {
        let metrics: VulnMetrics = serde_json::from_str(r#"{"runtime_exposed":44}"#)
            .expect("legacy metrics should deserialize");
        assert_eq!(metrics.runtime_scoped, 44);

        let json = serde_json::to_value(metrics).expect("metrics should serialize");
        assert_eq!(json["runtime_scoped"], 44);
        assert!(json.get("runtime_exposed").is_none());
    }
}

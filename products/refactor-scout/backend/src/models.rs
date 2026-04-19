use serde::{Deserialize, Serialize};

fn default_max_files() -> u32 {
    250
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRequest {
    #[serde(default)]
    pub repo_path: String,
    #[serde(default = "default_max_files")]
    pub max_files: u32,
}

impl Default for ScanRequest {
    fn default() -> Self {
        Self {
            repo_path: String::new(),
            max_files: default_max_files(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanMetrics {
    #[serde(default)]
    pub files_scanned: u32,
    #[serde(default)]
    pub files_skipped: u32,
    #[serde(default)]
    pub opportunities: u32,
    #[serde(default)]
    pub high_safety: u32,
    #[serde(default)]
    pub medium_safety: u32,
    #[serde(default)]
    pub large_file_count: u32,
    #[serde(default)]
    pub long_function_count: u32,
    #[serde(default)]
    pub repeated_literal_count: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefactorOpportunity {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub safety: String,
    #[serde(default)]
    pub effort: String,
    #[serde(default)]
    pub line_start: u32,
    #[serde(default)]
    pub line_end: u32,
    #[serde(default)]
    pub suggestion: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefactorScanResult {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub repo_path: String,
    #[serde(default)]
    pub repo_name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub metrics: ScanMetrics,
    #[serde(default)]
    pub opportunities: Vec<RefactorOpportunity>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub repo_path: String,
    #[serde(default)]
    pub repo_name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub opportunities: u32,
    #[serde(default)]
    pub high_safety: u32,
    #[serde(default)]
    pub medium_safety: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverviewCounts {
    #[serde(default)]
    pub scans: u32,
    #[serde(default)]
    pub repos: u32,
    #[serde(default)]
    pub opportunities: u32,
    #[serde(default)]
    pub high_safety: u32,
    #[serde(default)]
    pub large_file_count: u32,
    #[serde(default)]
    pub long_function_count: u32,
    #[serde(default)]
    pub repeated_literal_count: u32,
    #[serde(default)]
    pub last_repo: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverviewPayload {
    #[serde(default)]
    pub product: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub scan_count: u32,
    #[serde(default)]
    pub repo_count: u32,
    #[serde(default)]
    pub opportunity_count: u32,
    #[serde(default)]
    pub high_safety_count: u32,
    #[serde(default)]
    pub large_file_count: u32,
    #[serde(default)]
    pub long_function_count: u32,
    #[serde(default)]
    pub repeated_literal_count: u32,
    #[serde(default)]
    pub last_repo: String,
    #[serde(default)]
    pub allowed_roots: Vec<String>,
    #[serde(default)]
    pub remote_fs_enabled: bool,
}

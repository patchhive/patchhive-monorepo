use patchhive_product_core::contract::TargetSelectionMode;
use serde::{Deserialize, Serialize};

fn default_max_files() -> u32 {
    250
}

fn default_min_stars() -> u32 {
    25
}

fn default_cooldown_days() -> u32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryScope {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default = "default_min_stars")]
    pub min_stars: u32,
    #[serde(default = "default_cooldown_days")]
    pub cooldown_days: u32,
}

impl Default for DiscoveryScope {
    fn default() -> Self {
        Self {
            query: String::new(),
            topics: Vec::new(),
            languages: vec!["rust".into(), "typescript".into(), "python".into()],
            min_stars: default_min_stars(),
            cooldown_days: default_cooldown_days(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRequest {
    #[serde(default)]
    pub repo_path: String,
    #[serde(default = "default_max_files")]
    pub max_files: u32,
    #[serde(default)]
    pub discovery: DiscoveryScope,
}

impl Default for ScanRequest {
    fn default() -> Self {
        Self {
            repo_path: String::new(),
            max_files: default_max_files(),
            discovery: DiscoveryScope::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanPreset {
    pub name: String,
    pub params: ScanRequest,
    pub target_selection_mode: TargetSelectionMode,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoListItem {
    pub repo: String,
    pub list_type: String,
    pub added_at: String,
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
    pub returned_opportunities: u32,
    #[serde(default)]
    pub opportunities_truncated: bool,
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
    #[serde(default = "default_trigger_type")]
    pub trigger_type: String,
    #[serde(default)]
    pub schedule_name: Option<String>,
    #[serde(default)]
    pub target_selection_mode: TargetSelectionMode,
}

fn default_trigger_type() -> String {
    "operator".into()
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
    #[serde(default = "default_trigger_type")]
    pub trigger_type: String,
    #[serde(default)]
    pub schedule_name: Option<String>,
    #[serde(default)]
    pub target_selection_mode: TargetSelectionMode,
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
    pub medium_safety: u32,
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
    pub medium_safety_count: u32,
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

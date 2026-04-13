use serde::{Deserialize, Serialize};

pub use patchhive_github_data::models::{
    GitHubIssue, GitHubPullFile, GitHubPullRequest, GitHubReview, GitHubReviewComment,
};

fn slug_component(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }

    out.trim_matches('-').to_string()
}

pub fn stable_memory_ref(repo: &str, kind: &str, title: &str) -> String {
    let repo = slug_component(repo);
    let kind = slug_component(kind);
    let title = slug_component(title);
    format!("{repo}__{kind}__{title}")
}

fn default_memory_disposition() -> String {
    "signal".into()
}

fn default_context_consumer() -> String {
    String::new()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestParams {
    pub repo: String,
    pub merged_pr_limit: u32,
    pub issue_limit: u32,
    pub since_days: u32,
}

impl Default for IngestParams {
    fn default() -> Self {
        Self {
            repo: String::new(),
            merged_pr_limit: 18,
            issue_limit: 24,
            since_days: 180,
        }
    }
}

impl IngestParams {
    pub fn normalized(&self) -> Self {
        Self {
            repo: self.repo.trim().to_string(),
            merged_pr_limit: self.merged_pr_limit.clamp(5, 40),
            issue_limit: self.issue_limit.clamp(5, 40),
            since_days: self.since_days.clamp(30, 730),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryEvidence {
    pub source_type: String,
    pub title: String,
    pub url: String,
    pub path: Option<String>,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryEntry {
    pub id: String,
    #[serde(default)]
    pub memory_ref: String,
    pub run_id: String,
    pub repo: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub prompt_line: String,
    pub confidence: f64,
    pub frequency: u32,
    #[serde(default = "default_memory_disposition")]
    pub disposition: String,
    #[serde(default)]
    pub pinned: bool,
    pub tags: Vec<String>,
    pub evidence: Vec<MemoryEvidence>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestSummary {
    pub merged_prs_analyzed: u32,
    pub review_feedback_items: u32,
    pub closed_issues_analyzed: u32,
    pub memories_created: u32,
    pub conventions: u32,
    pub failures: u32,
    pub hotspots: u32,
    pub top_memory: String,
}

impl IngestSummary {
    pub fn empty() -> Self {
        Self {
            merged_prs_analyzed: 0,
            review_feedback_items: 0,
            closed_issues_analyzed: 0,
            memories_created: 0,
            conventions: 0,
            failures: 0,
            hotspots: 0,
            top_memory: "No strong memory signals yet.".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRecord {
    pub id: String,
    pub repo: String,
    pub created_at: String,
    pub params: IngestParams,
    pub summary: IngestSummary,
    pub prompt_pack: String,
    pub entries: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub repo: String,
    pub created_at: String,
    pub memories_created: u32,
    pub conventions: u32,
    pub failures: u32,
    pub hotspots: u32,
    pub top_memory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDiffItem {
    pub memory_ref: String,
    pub kind: String,
    pub title: String,
    pub prompt_line: String,
    pub current_confidence: Option<f64>,
    pub previous_confidence: Option<f64>,
    pub current_frequency: Option<u32>,
    pub previous_frequency: Option<u32>,
    pub delta_confidence: f64,
    pub delta_frequency: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDiffSummary {
    pub new_entries: u32,
    pub strengthened_entries: u32,
    pub faded_entries: u32,
    pub retired_entries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDiffResponse {
    pub repo: String,
    pub run_id: String,
    pub previous_run_id: Option<String>,
    pub created_at: String,
    pub previous_created_at: Option<String>,
    pub summary: String,
    pub counts: RunDiffSummary,
    pub new_entries: Vec<RunDiffItem>,
    pub strengthened_entries: Vec<RunDiffItem>,
    pub faded_entries: Vec<RunDiffItem>,
    pub retired_entries: Vec<RunDiffItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownRepo {
    pub repo: String,
    pub last_ingested_at: String,
    pub run_count: u32,
    pub memory_count: u32,
    pub top_memory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewCounts {
    pub repos: u32,
    pub runs: u32,
    pub memories: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewPayload {
    pub product: String,
    pub tagline: String,
    pub counts: OverviewCounts,
    pub repos: Vec<KnownRepo>,
    pub featured_memories: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequest {
    pub repo: String,
    #[serde(default = "default_context_consumer")]
    pub consumer: String,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub task_summary: String,
    #[serde(default)]
    pub diff_summary: String,
    #[serde(default = "default_context_limit")]
    pub limit: u32,
}

fn default_context_limit() -> u32 {
    6
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: String,
    pub memory_ref: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub prompt_line: String,
    pub confidence: f64,
    pub frequency: u32,
    pub retrieval_score: f64,
    #[serde(default = "default_memory_disposition")]
    pub disposition: String,
    #[serde(default)]
    pub pinned: bool,
    pub matched_paths: Vec<String>,
    pub matched_terms: Vec<String>,
    pub tags: Vec<String>,
    pub evidence: Vec<MemoryEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResponse {
    pub repo: String,
    pub consumer: String,
    pub run_id: String,
    pub created_at: String,
    pub summary: String,
    pub prompt_lines: Vec<String>,
    pub entries: Vec<ContextEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCurationUpdate {
    pub repo: String,
    pub memory_ref: String,
    #[serde(default = "default_memory_disposition")]
    pub disposition: String,
    #[serde(default)]
    pub pinned: bool,
}

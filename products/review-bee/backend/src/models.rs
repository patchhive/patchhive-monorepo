use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub repo: String,
    pub pr_number: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChecklistEvidence {
    #[serde(default)]
    pub author_login: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub excerpt: String,
    #[serde(default)]
    pub resolved: bool,
    #[serde(default)]
    pub outdated: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChecklistItem {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub prompt_hint: String,
    #[serde(default)]
    pub path_hints: Vec<String>,
    #[serde(default)]
    pub commenter_logins: Vec<String>,
    #[serde(default)]
    pub open_threads: u32,
    #[serde(default)]
    pub resolved_threads: u32,
    #[serde(default)]
    pub outdated_threads: u32,
    #[serde(default)]
    pub comment_count: u32,
    #[serde(default)]
    pub evidence: Vec<ChecklistEvidence>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewMetrics {
    #[serde(default)]
    pub review_count: u32,
    #[serde(default)]
    pub requested_changes_reviews: u32,
    #[serde(default)]
    pub approval_reviews: u32,
    #[serde(default)]
    pub comment_reviews: u32,
    #[serde(default)]
    pub thread_count: u32,
    #[serde(default)]
    pub actionable_threads: u32,
    #[serde(default)]
    pub open_items: u32,
    #[serde(default)]
    pub resolved_items: u32,
    #[serde(default)]
    pub reviewer_count: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewResult {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub pr_number: i64,
    #[serde(default)]
    pub pr_title: String,
    #[serde(default)]
    pub pr_url: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub metrics: ReviewMetrics,
    #[serde(default)]
    pub reviewers: Vec<String>,
    #[serde(default)]
    pub prompt_suggestions: Vec<String>,
    #[serde(default)]
    pub checklist: Vec<ChecklistItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub pr_number: i64,
    #[serde(default)]
    pub pr_title: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub action_items: u32,
    #[serde(default)]
    pub open_items: u32,
    #[serde(default)]
    pub resolved_items: u32,
    #[serde(default)]
    pub reviewer_count: u32,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverviewCounts {
    #[serde(default)]
    pub reviews: u32,
    #[serde(default)]
    pub repos: u32,
    #[serde(default)]
    pub open_items: u32,
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
    pub recent_reviews: Vec<HistoryItem>,
}

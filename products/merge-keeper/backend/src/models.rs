use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssessmentRequest {
    pub repo: String,
    pub pr_number: i64,
    #[serde(default = "default_publish_report")]
    pub publish_report: bool,
    #[serde(default)]
    pub require_approval: Option<bool>,
}

fn default_publish_report() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MergeMetrics {
    #[serde(default)]
    pub approvals: u32,
    #[serde(default)]
    pub changes_requested: u32,
    #[serde(default)]
    pub comment_reviews: u32,
    #[serde(default)]
    pub reviewer_count: u32,
    #[serde(default)]
    pub review_threads: u32,
    #[serde(default)]
    pub open_review_threads: u32,
    #[serde(default)]
    pub actionable_open_threads: u32,
    #[serde(default)]
    pub successful_checks: u32,
    #[serde(default)]
    pub pending_checks: u32,
    #[serde(default)]
    pub failing_checks: u32,
    #[serde(default)]
    pub changed_files: u32,
    #[serde(default)]
    pub additions: u32,
    #[serde(default)]
    pub deletions: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MergeSignal {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewBeeContext {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub open_items: u32,
    #[serde(default)]
    pub actionable_threads: u32,
    #[serde(default)]
    pub top_items: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustGateContext {
    #[serde(default)]
    pub recommendation: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub risk_score: u32,
    #[serde(default)]
    pub blocked_findings: u32,
    #[serde(default)]
    pub warning_findings: u32,
    #[serde(default)]
    pub top_findings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoMemoryContextPreview {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub prompt_lines: Vec<String>,
    #[serde(default)]
    pub policy_entries: u32,
    #[serde(default)]
    pub pinned_entries: u32,
    #[serde(default)]
    pub top_entries: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewerState {
    #[serde(default)]
    pub login: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub submitted_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubAssessmentContext {
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub pr_number: i64,
    #[serde(default)]
    pub pr_title: String,
    #[serde(default)]
    pub pr_url: String,
    #[serde(default)]
    pub head_sha: String,
    #[serde(default)]
    pub head_repo: String,
    #[serde(default)]
    pub head_ref: String,
    #[serde(default)]
    pub base_ref: String,
    #[serde(default)]
    pub trigger: String,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub action: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubReportOutcome {
    #[serde(default)]
    pub attempted: bool,
    #[serde(default)]
    pub delivered: bool,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub details: Vec<String>,
    #[serde(default)]
    pub check_url: String,
    #[serde(default)]
    pub status_url: String,
    #[serde(default)]
    pub comment_url: String,
    #[serde(default)]
    pub comment_mode: String,
    #[serde(default)]
    pub report_markdown: String,
}

impl GitHubReportOutcome {
    pub fn is_complete_delivery(&self) -> bool {
        self.delivered
            && !self.comment_url.trim().is_empty()
            && (!self.check_url.trim().is_empty() || !self.status_url.trim().is_empty())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MergeAssessment {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub pr_number: i64,
    #[serde(default)]
    pub pr_title: String,
    #[serde(default)]
    pub pr_url: String,
    #[serde(default)]
    pub readiness: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default = "default_approval_required")]
    pub approval_required: bool,
    #[serde(default)]
    pub mergeable: String,
    #[serde(default)]
    pub mergeable_state: String,
    #[serde(default)]
    pub base_ref: String,
    #[serde(default)]
    pub head_ref: String,
    #[serde(default)]
    pub metrics: MergeMetrics,
    #[serde(default)]
    pub reviewer_states: Vec<ReviewerState>,
    #[serde(default)]
    pub blockers: Vec<MergeSignal>,
    #[serde(default)]
    pub warnings: Vec<MergeSignal>,
    #[serde(default)]
    pub review_bee: Option<ReviewBeeContext>,
    #[serde(default)]
    pub trust_gate: Option<TrustGateContext>,
    #[serde(default)]
    pub repo_memory: Option<RepoMemoryContextPreview>,
    #[serde(default)]
    pub github: Option<GitHubAssessmentContext>,
    #[serde(default)]
    pub github_report: Option<GitHubReportOutcome>,
}

fn default_approval_required() -> bool {
    true
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
    pub readiness: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub blockers_count: u32,
    #[serde(default)]
    pub warnings_count: u32,
    #[serde(default)]
    pub approvals_count: u32,
    #[serde(default)]
    pub failing_checks_count: u32,
    #[serde(default)]
    pub pending_checks_count: u32,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverviewCounts {
    #[serde(default)]
    pub runs: u32,
    #[serde(default)]
    pub repos: u32,
    #[serde(default)]
    pub ready_runs: u32,
    #[serde(default)]
    pub blocked_runs: u32,
    #[serde(default)]
    pub hold_runs: u32,
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

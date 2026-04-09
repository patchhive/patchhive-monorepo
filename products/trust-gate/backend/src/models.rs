use serde::{Deserialize, Serialize};

fn default_blocked_paths() -> Vec<String> {
    vec![
        ".github/workflows/".into(),
        "infra/".into(),
        "terraform/".into(),
        "migrations/".into(),
        "schema.sql".into(),
    ]
}

fn default_warn_paths() -> Vec<String> {
    vec![
        "auth/".into(),
        "permissions".into(),
        "billing".into(),
        "Dockerfile".into(),
        "docker-compose".into(),
    ]
}

fn default_require_test_for_paths() -> Vec<String> {
    vec![
        "src/".into(),
        "app/".into(),
        "lib/".into(),
        "server/".into(),
        "backend/".into(),
    ]
}

fn default_test_paths() -> Vec<String> {
    vec![
        "tests/".into(),
        "__tests__/".into(),
        ".test.".into(),
        ".spec.".into(),
    ]
}

fn default_suspicious_terms() -> Vec<String> {
    vec![
        "TODO".into(),
        "FIXME".into(),
        "skip ci".into(),
        "eval(".into(),
        "exec(".into(),
        "unsafe".into(),
        "curl | sh".into(),
        "rm -rf".into(),
        "password".into(),
        "secret".into(),
        "token".into(),
    ]
}

fn default_blocked_terms() -> Vec<String> {
    vec![
        "BEGIN PRIVATE KEY".into(),
        "PRIVATE KEY-----".into(),
        "ghp_".into(),
        "github_pat_".into(),
        "sk-".into(),
        "AKIA".into(),
    ]
}

fn default_max_files() -> u32 {
    12
}

fn default_max_additions() -> u32 {
    400
}

fn default_max_deletions() -> u32 {
    250
}

fn default_review_source_kind() -> String {
    "manual".into()
}

fn default_publish_status() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRuleSet {
    pub repo: String,
    #[serde(default = "default_blocked_paths")]
    pub blocked_paths: Vec<String>,
    #[serde(default = "default_warn_paths")]
    pub warn_paths: Vec<String>,
    #[serde(default = "default_require_test_for_paths")]
    pub require_test_for_paths: Vec<String>,
    #[serde(default = "default_test_paths")]
    pub test_paths: Vec<String>,
    #[serde(default = "default_suspicious_terms")]
    pub suspicious_terms: Vec<String>,
    #[serde(default = "default_blocked_terms")]
    pub blocked_terms: Vec<String>,
    #[serde(default = "default_max_files")]
    pub max_files: u32,
    #[serde(default = "default_max_additions")]
    pub max_additions: u32,
    #[serde(default = "default_max_deletions")]
    pub max_deletions: u32,
    #[serde(default)]
    pub notes: String,
}

impl Default for RepoRuleSet {
    fn default() -> Self {
        Self {
            repo: String::new(),
            blocked_paths: default_blocked_paths(),
            warn_paths: default_warn_paths(),
            require_test_for_paths: default_require_test_for_paths(),
            test_paths: default_test_paths(),
            suspicious_terms: default_suspicious_terms(),
            blocked_terms: default_blocked_terms(),
            max_files: default_max_files(),
            max_additions: default_max_additions(),
            max_deletions: default_max_deletions(),
            notes: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRuleSet {
    pub repo: String,
    pub rules: RepoRuleSet,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePack {
    pub id: String,
    pub label: String,
    pub description: String,
    pub rules: RepoRuleSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub repo: String,
    pub diff: String,
    #[serde(default)]
    pub ai_source: String,
    #[serde(default)]
    pub rules: Option<RepoRuleSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPrReviewRequest {
    pub repo: String,
    pub pr_number: i64,
    #[serde(default)]
    pub ai_source: String,
    #[serde(default)]
    pub rules: Option<RepoRuleSet>,
    #[serde(default = "default_publish_status")]
    pub publish_status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewMetricSummary {
    pub files_changed: u32,
    pub additions: u32,
    pub deletions: u32,
    pub tests_changed: u32,
    pub risky_files: u32,
    pub blocked_findings: u32,
    pub warning_findings: u32,
    #[serde(default)]
    pub generated_files: u32,
    #[serde(default)]
    pub source_files_changed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAssessment {
    pub path: String,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
    pub matched_rules: Vec<String>,
    pub summary: String,
    #[serde(default)]
    pub generated: bool,
    #[serde(default)]
    pub path_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    pub key: String,
    pub label: String,
    pub severity: String,
    pub detail: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubReviewContext {
    pub repo: String,
    #[serde(default)]
    pub head_repo: String,
    pub pr_number: i64,
    #[serde(default)]
    pub pr_title: String,
    #[serde(default)]
    pub pr_url: String,
    #[serde(default)]
    pub head_sha: String,
    #[serde(default)]
    pub head_ref: String,
    #[serde(default)]
    pub base_ref: String,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub trigger: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    pub id: String,
    pub created_at: String,
    pub repo: String,
    pub ai_source: String,
    pub recommendation: String,
    pub risk_score: u32,
    pub summary: String,
    pub metrics: ReviewMetricSummary,
    pub files: Vec<FileAssessment>,
    pub findings: Vec<ReviewFinding>,
    pub rules: RepoRuleSet,
    pub diff: String,
    #[serde(default = "default_review_source_kind")]
    pub source_kind: String,
    #[serde(default)]
    pub github: Option<GitHubReviewContext>,
    #[serde(default)]
    pub github_report: Option<GitHubReportOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewHistoryItem {
    pub id: String,
    pub created_at: String,
    pub repo: String,
    pub ai_source: String,
    pub recommendation: String,
    pub risk_score: u32,
    pub files_changed: u32,
    pub summary: String,
    #[serde(default = "default_review_source_kind")]
    pub source_kind: String,
    #[serde(default)]
    pub pr_number: Option<i64>,
}

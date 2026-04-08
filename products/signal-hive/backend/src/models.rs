use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanParams {
    pub search_query: String,
    pub topics: Vec<String>,
    pub languages: Vec<String>,
    pub min_stars: u32,
    pub max_repos: u32,
    pub issues_per_repo: u32,
    pub stale_days: u32,
}

impl Default for ScanParams {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            topics: Vec::new(),
            languages: vec!["rust".into()],
            min_stars: 25,
            max_repos: 8,
            issues_per_repo: 30,
            stale_days: 45,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSample {
    pub number: u32,
    pub title: String,
    pub url: String,
    pub updated_at: String,
    pub age_days: i64,
    pub comments: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCandidate {
    pub left_number: u32,
    pub right_number: u32,
    pub left_title: String,
    pub right_title: String,
    pub similarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreFactor {
    pub key: String,
    pub label: String,
    pub impact: f64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringBugCluster {
    pub label: String,
    pub issue_count: u32,
    pub shared_terms: Vec<String>,
    pub examples: Vec<IssueSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoSignal {
    pub full_name: String,
    pub repo_url: String,
    pub description: String,
    pub language: String,
    pub stars: u32,
    pub open_issues: u32,
    pub sampled_issues: u32,
    pub stale_issues: u32,
    pub unlabeled_issues: u32,
    pub stale_bug_issues: u32,
    pub stale_high_comment_issues: u32,
    pub duplicate_candidates: Vec<DuplicateCandidate>,
    pub recurring_bug_clusters: Vec<RecurringBugCluster>,
    pub todo_count: u32,
    pub fixme_count: u32,
    pub priority_score: f64,
    pub score_breakdown: Vec<ScoreFactor>,
    pub summary: String,
    pub signals: Vec<String>,
    pub issue_examples: Vec<IssueSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoListItem {
    pub repo: String,
    pub list_type: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub total_repos: u32,
    pub total_signals: u32,
    pub top_repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRecord {
    pub id: String,
    pub created_at: String,
    pub params: ScanParams,
    pub summary: ScanSummary,
    pub repos: Vec<RepoSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHistoryItem {
    pub id: String,
    pub created_at: String,
    pub search_query: String,
    pub topics: Vec<String>,
    pub languages: Vec<String>,
    pub max_repos: u32,
    pub total_repos: u32,
    pub total_signals: u32,
    pub top_repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanPreset {
    pub name: String,
    pub params: ScanParams,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchRepoOwner {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchRepo {
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stargazers_count: u32,
    pub open_issues_count: u32,
    pub owner: SearchRepoOwner,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchRepositoriesResponse {
    pub items: Vec<SearchRepo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubLabel {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubIssue {
    pub number: u32,
    pub title: String,
    pub html_url: String,
    pub updated_at: String,
    pub comments: u32,
    pub labels: Vec<GitHubLabel>,
    pub pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodeSearchResponse {
    pub total_count: u32,
}

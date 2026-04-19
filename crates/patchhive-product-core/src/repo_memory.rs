use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoMemoryContextRequest {
    pub repo: String,
    #[serde(default)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoMemoryContextEvidence {
    #[serde(default)]
    pub source_type: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub excerpt: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoMemoryContextEntry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub memory_ref: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub prompt_line: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub frequency: u32,
    #[serde(default)]
    pub retrieval_score: f64,
    #[serde(default)]
    pub disposition: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub matched_paths: Vec<String>,
    #[serde(default)]
    pub matched_terms: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<RepoMemoryContextEvidence>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoMemoryContextResponse {
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub consumer: String,
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub prompt_lines: Vec<String>,
    #[serde(default)]
    pub entries: Vec<RepoMemoryContextEntry>,
}

pub fn repo_memory_url() -> Option<String> {
    std::env::var("PATCHHIVE_REPO_MEMORY_URL")
        .ok()
        .or_else(|| std::env::var("REPO_MEMORY_URL").ok())
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

fn repo_memory_api_key() -> Option<String> {
    std::env::var("PATCHHIVE_REPO_MEMORY_API_KEY")
        .ok()
        .or_else(|| std::env::var("REPO_MEMORY_API_KEY").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub async fn fetch_repo_memory_context(
    client: &Client,
    request: &RepoMemoryContextRequest,
) -> Result<Option<RepoMemoryContextResponse>> {
    let Some(base_url) = repo_memory_url() else {
        return Ok(None);
    };

    let url = format!("{base_url}/context");
    let mut http = client.post(url).json(request);
    if let Some(api_key) = repo_memory_api_key() {
        http = http.header("X-API-Key", api_key);
    }

    let response = http
        .send()
        .await
        .context("RepoMemory context request failed")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "RepoMemory context request failed: {status} {body}"
        ));
    }

    let parsed = response
        .json::<RepoMemoryContextResponse>()
        .await
        .context("Could not decode RepoMemory context response")?;

    Ok(Some(parsed))
}

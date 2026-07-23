use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::{RwLock, Semaphore};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub role: String, // scout | judge | reaper | smith | gatekeeper
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub bot_token: Option<String>,
    pub bot_user: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub current_task: String,
    #[serde(default)]
    pub stats: AgentStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AgentStats {
    pub fixed: u32,
    pub skipped: u32,
    pub errors: u32,
    pub cost: f64,
}

pub type AgentMap = Arc<RwLock<HashMap<String, AgentConfig>>>;

#[derive(Clone)]
pub struct AppState {
    pub agents: AgentMap,
    pub run_active: Arc<AtomicBool>,
    pub watch_mode: Arc<AtomicBool>,
    pub process_worker_semaphore: Arc<Semaphore>,
    pub process_worker_limit: usize,
    pub http: Client,
}

impl AppState {
    pub fn new() -> Self {
        let process_worker_limit = std::env::var("REAPER_MAX_ACTIVE_WORKERS")
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(3)
            .clamp(1, 128);
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            run_active: Arc::new(AtomicBool::new(false)),
            watch_mode: Arc::new(AtomicBool::new(false)),
            process_worker_semaphore: Arc::new(Semaphore::new(process_worker_limit)),
            process_worker_limit,
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("HTTP client build failed"),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::AgentConfig;

    #[test]
    fn browser_created_agent_accepts_sparse_stats() {
        let agent: AgentConfig = serde_json::from_value(serde_json::json!({
            "id": "scout-test",
            "name": "PatchHive Scout",
            "role": "scout",
            "provider": "custom",
            "model": "openai/gpt-oss-20b:free",
            "base_url": "https://openrouter.ai/api/v1",
            "api_key": null,
            "bot_token": null,
            "bot_user": null,
            "status": "idle",
            "current_task": "",
            "stats": {}
        }))
        .expect("sparse browser stats should use typed defaults");

        assert_eq!(agent.stats.fixed, 0);
        assert_eq!(agent.stats.skipped, 0);
        assert_eq!(agent.stats.errors, 0);
        assert_eq!(agent.stats.cost, 0.0);
    }
}

use std::collections::HashMap;
use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub role: String, // scout | judge | reaper | smith | gatekeeper
    pub provider: String,
    pub model: String,
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
pub struct AgentStats {
    pub fixed: u32,
    pub skipped: u32,
    pub errors: u32,
    pub cost: f64,
}

pub type AgentMap = Arc<RwLock<HashMap<String, AgentConfig>>>;
pub type CooldownMap = Arc<RwLock<HashMap<String, std::time::Instant>>>;

#[derive(Clone)]
pub struct AppState {
    pub agents: AgentMap,
    pub cooldowns: CooldownMap,
    pub run_active: Arc<AtomicBool>,
    pub watch_mode: Arc<AtomicBool>,
    pub http: Client,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            cooldowns: Arc::new(RwLock::new(HashMap::new())),
            run_active: Arc::new(AtomicBool::new(false)),
            watch_mode: Arc::new(AtomicBool::new(false)),
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("HTTP client build failed"),
        }
    }
}

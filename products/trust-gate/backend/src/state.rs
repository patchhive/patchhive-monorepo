use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub http: Client,
}

impl AppState {
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent("trust-gate/0.1")
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { http }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

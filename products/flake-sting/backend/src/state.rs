#[derive(Clone)]
pub struct AppState {
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("FlakeSting by PatchHive")
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build reqwest client"),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

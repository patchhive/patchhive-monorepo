#[derive(Clone)]
pub struct AppState {
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("SignalHive by PatchHive")
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .expect("failed to build reqwest client");
        Self { http }
    }
}

use reqwest::Client;

#[derive(Clone)]
pub struct AppState {
    pub http: Client,
}

impl AppState {
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent("DepTriage by PatchHive")
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .expect("failed to build reqwest client");
        Self { http }
    }
}

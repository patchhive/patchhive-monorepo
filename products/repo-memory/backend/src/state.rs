use reqwest::Client;

#[derive(Clone)]
pub struct AppState {
    pub http: Client,
}

impl AppState {
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent("RepoMemory by PatchHive")
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to create reqwest client");

        Self { http }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

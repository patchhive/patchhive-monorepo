use reqwest::Client;

#[derive(Clone)]
pub struct AppState {
    pub http: Client,
}

impl AppState {
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent("RepoMemory by PatchHive")
            .build()
            .expect("failed to create reqwest client");

        Self { http }
    }
}

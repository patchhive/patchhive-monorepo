use reqwest::Client;

#[derive(Clone)]
pub struct AppState {
    pub http: Client,
}

impl AppState {
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent("trust-gate/0.1")
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { http }
    }
}

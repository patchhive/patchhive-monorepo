#[derive(Clone)]
pub struct AppState {
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }
}

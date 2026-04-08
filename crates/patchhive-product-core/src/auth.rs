use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct ApiKeyAuthConfig {
    pub hash_env_var: String,
    pub key_prefix: String,
    pub env_path: PathBuf,
    pub public_paths: Vec<String>,
    pub unauthorized_message: String,
}

impl ApiKeyAuthConfig {
    pub fn new(hash_env_var: impl Into<String>, key_prefix: impl Into<String>) -> Self {
        Self {
            hash_env_var: hash_env_var.into(),
            key_prefix: key_prefix.into(),
            env_path: PathBuf::from(".env"),
            public_paths: Vec::new(),
            unauthorized_message: "Unauthorized — provide X-API-Key header".into(),
        }
    }

    pub fn with_env_path(mut self, env_path: impl Into<PathBuf>) -> Self {
        self.env_path = env_path.into();
        self
    }

    pub fn with_public_paths<I, S>(mut self, public_paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.public_paths = public_paths.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_unauthorized_message(mut self, message: impl Into<String>) -> Self {
        self.unauthorized_message = message.into();
        self
    }
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn stored_hash(config: &ApiKeyAuthConfig) -> String {
    std::env::var(&config.hash_env_var).unwrap_or_default()
}

fn persist_hash(env_path: &Path, env_var: &str, hash: &str) {
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(env_path);

    let existing = fs::read_to_string(env_path).unwrap_or_default();
    let filtered = existing
        .lines()
        .filter(|line| !line.trim_start().starts_with(&format!("{env_var}=")))
        .collect::<Vec<_>>()
        .join("\n");

    let content = if filtered.trim().is_empty() {
        format!("{env_var}={hash}\n")
    } else {
        format!("{filtered}\n{env_var}={hash}\n")
    };

    let _ = fs::write(env_path, content);
}

fn request_token(headers: &HeaderMap) -> &str {
    headers
        .get("X-API-Key")
        .or_else(|| headers.get("Authorization"))
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim_start_matches("Bearer ").trim())
        .unwrap_or("")
}

pub fn auth_enabled(config: &ApiKeyAuthConfig) -> bool {
    !stored_hash(config).is_empty()
}

pub fn verify_token(config: &ApiKeyAuthConfig, token: &str) -> bool {
    let stored = stored_hash(config);
    if stored.is_empty() {
        return true;
    }

    let actual = hash_token(token).into_bytes();
    let expected = stored.into_bytes();
    if actual.len() != expected.len() {
        return false;
    }

    actual
        .iter()
        .zip(expected.iter())
        .fold(0u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

pub fn generate_and_save_key(config: &ApiKeyAuthConfig) -> String {
    let key = format!(
        "{}{}",
        config.key_prefix,
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let hash = hash_token(&key);
    std::env::set_var(&config.hash_env_var, &hash);
    persist_hash(&config.env_path, &config.hash_env_var, &hash);
    key
}

pub async fn auth_middleware(
    config: &ApiKeyAuthConfig,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    if !auth_enabled(config) {
        return next.run(request).await;
    }

    let path = request.uri().path();
    if config.public_paths.iter().any(|public| path == public) {
        return next.run(request).await;
    }

    let token = request_token(&headers);
    if token.is_empty() || !verify_token(config, token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": config.unauthorized_message })),
        )
            .into_response();
    }

    next.run(request).await
}

use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use anyhow::{Context, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock, RwLock},
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

fn runtime_hashes() -> &'static RwLock<HashMap<String, String>> {
    static RUNTIME_HASHES: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();
    RUNTIME_HASHES.get_or_init(|| RwLock::new(HashMap::new()))
}

fn warned_configs() -> &'static Mutex<HashSet<String>> {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    WARNED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn stored_hash(config: &ApiKeyAuthConfig) -> String {
    runtime_hashes()
        .read()
        .ok()
        .and_then(|hashes| hashes.get(&config.hash_env_var).cloned())
        .or_else(|| std::env::var(&config.hash_env_var).ok())
        .unwrap_or_default()
}

fn persist_hash(env_path: &Path, env_var: &str, hash: &str) -> Result<()> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(env_path)
        .with_context(|| format!("failed to open {}", env_path.display()))?;

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

    fs::write(env_path, content)
        .with_context(|| format!("failed to write {}", env_path.display()))?;
    Ok(())
}

fn request_token(headers: &HeaderMap) -> &str {
    headers
        .get("X-API-Key")
        .or_else(|| headers.get("Authorization"))
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.len() >= 7 && trimmed[..7].eq_ignore_ascii_case("bearer ") {
                trimmed[7..].trim()
            } else {
                trimmed
            }
        })
        .unwrap_or("")
}

pub fn auth_enabled(config: &ApiKeyAuthConfig) -> bool {
    !stored_hash(config).is_empty()
}

fn warn_auth_unconfigured(config: &ApiKeyAuthConfig) {
    let Ok(mut warned) = warned_configs().lock() else {
        tracing::warn!(
            "{} auth is not configured; protected endpoints are unavailable until /auth/generate-key succeeds",
            config.hash_env_var
        );
        return;
    };

    if warned.insert(config.hash_env_var.clone()) {
        tracing::warn!(
            "{} auth is not configured; protected endpoints are unavailable until /auth/generate-key succeeds",
            config.hash_env_var
        );
    }
}

pub fn verify_token(config: &ApiKeyAuthConfig, token: &str) -> bool {
    let stored = stored_hash(config);
    if stored.is_empty() {
        return false;
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

pub fn generate_and_save_key(config: &ApiKeyAuthConfig) -> Result<String> {
    let key = format!(
        "{}{}",
        config.key_prefix,
        uuid::Uuid::new_v4().to_string().replace('-', "")
    );
    let hash = hash_token(&key);

    runtime_hashes()
        .write()
        .map_err(|_| anyhow::anyhow!("failed to acquire runtime auth lock"))?
        .insert(config.hash_env_var.clone(), hash.clone());
    persist_hash(&config.env_path, &config.hash_env_var, &hash)?;
    Ok(key)
}

pub fn bootstrap_request_allowed(headers: &HeaderMap) -> bool {
    if matches!(
        std::env::var("PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP")
            .ok()
            .as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "on")
    ) {
        return true;
    }

    for header in ["origin", "referer"] {
        if let Some(value) = headers.get(header).and_then(|value| value.to_str().ok()) {
            if !local_endpoint(value) {
                return false;
            }
        }
    }

    if let Some(value) = headers.get("x-forwarded-for").and_then(|value| value.to_str().ok()) {
        let client = value.split(',').next().unwrap_or("").trim();
        if !matches!(client, "" | "127.0.0.1" | "::1" | "[::1]") {
            return false;
        }
    }

    headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .map(local_endpoint)
        .unwrap_or(false)
}

pub fn auth_status_payload(config: &ApiKeyAuthConfig) -> serde_json::Value {
    let configured = auth_enabled(config);
    json!({
        "auth_enabled": configured,
        "auth_configured": configured,
        "bootstrap_required": !configured,
        "auth_storage": "session",
    })
}

fn local_endpoint(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    let without_scheme = trimmed
        .split("://")
        .nth(1)
        .unwrap_or(trimmed)
        .split('/')
        .next()
        .unwrap_or(trimmed)
        .trim();

    let host = without_scheme
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(':')
        .next()
        .unwrap_or(without_scheme)
        .trim();

    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

pub async fn auth_middleware(
    config: &ApiKeyAuthConfig,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if config.public_paths.iter().any(|public| path == public) {
        return next.run(request).await;
    }

    if !auth_enabled(config) {
        warn_auth_unconfigured(config);
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "API key auth is not configured yet. Generate the first key from a localhost session before using protected endpoints."
            })),
        )
            .into_response();
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

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    fn test_config(env_var: &str, env_path: PathBuf) -> ApiKeyAuthConfig {
        ApiKeyAuthConfig::new(env_var, "ph_").with_env_path(env_path)
    }

    fn clear_runtime_hash(env_var: &str) {
        if let Ok(mut hashes) = runtime_hashes().write() {
            hashes.remove(env_var);
        }
    }

    #[test]
    fn generate_and_save_key_persists_hash_and_verifies_token() {
        let env_var = format!("PATCHHIVE_TEST_AUTH_{}", uuid::Uuid::new_v4().simple());
        let env_path = std::env::temp_dir().join(format!("{env_var}.env"));
        let config = test_config(&env_var, env_path.clone());

        let key = generate_and_save_key(&config).expect("key generation should succeed");
        let written = fs::read_to_string(&env_path).expect("env file should be written");

        assert!(key.starts_with("ph_"));
        assert!(written.contains(&format!("{}=", env_var)));
        assert!(verify_token(&config, &key));
        assert!(!verify_token(&config, "ph_wrong"));

        clear_runtime_hash(&env_var);
        let _ = fs::remove_file(env_path);
    }

    #[test]
    fn auth_status_payload_reports_bootstrap_when_unconfigured() {
        let env_var = format!("PATCHHIVE_TEST_AUTH_{}", uuid::Uuid::new_v4().simple());
        let config = test_config(&env_var, PathBuf::from("/tmp/unused.env"));
        clear_runtime_hash(&env_var);

        let payload = auth_status_payload(&config);
        assert_eq!(payload["auth_enabled"], false);
        assert_eq!(payload["bootstrap_required"], true);
        assert_eq!(payload["auth_storage"], "session");
    }

    #[test]
    fn request_token_prefers_api_key_and_accepts_case_insensitive_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("bearer test-token"));
        assert_eq!(request_token(&headers), "test-token");

        headers.insert("X-API-Key", HeaderValue::from_static("direct-key"));
        assert_eq!(request_token(&headers), "direct-key");
    }

    #[test]
    fn bootstrap_request_allows_local_only_by_default() {
        let mut local = HeaderMap::new();
        local.insert("host", HeaderValue::from_static("127.0.0.1:8000"));
        local.insert("origin", HeaderValue::from_static("http://localhost:5173"));
        assert!(bootstrap_request_allowed(&local));

        let mut remote = HeaderMap::new();
        remote.insert("host", HeaderValue::from_static("0.0.0.0:8000"));
        remote.insert("origin", HeaderValue::from_static("https://evil.example"));
        assert!(!bootstrap_request_allowed(&remote));
    }
}

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use dotenvy::from_path_iter;

fn env_path() -> PathBuf { PathBuf::from(".env") }

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn stored_hash() -> String {
    std::env::var("REAPER_API_KEY_HASH").unwrap_or_default()
}

pub fn auth_enabled() -> bool { !stored_hash().is_empty() }

pub fn verify_token(token: &str) -> bool {
    let stored = stored_hash();
    if stored.is_empty() { return true; }
    use hmac::Mac;
    // Constant-time compare
    let a = hash_token(token).into_bytes();
    let b = stored.into_bytes();
    if a.len() != b.len() { return false; }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn generate_and_save_key() -> String {
    use uuid::Uuid;
    let key = format!("rr-{}", Uuid::new_v4().to_string().replace('-', ""));
    let hash = hash_token(&key);
    std::env::set_var("REAPER_API_KEY_HASH", &hash);

    // Persist to .env
    let path = env_path();
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&path);
    let _ = std::fs::write(&path, format!(
        "{}\nREAPER_API_KEY_HASH={}\n",
        std::fs::read_to_string(&path).unwrap_or_default().lines()
            .filter(|l| !l.starts_with("REAPER_API_KEY_HASH"))
            .collect::<Vec<_>>().join("\n"),
        hash,
    ));
    key
}

const PUBLIC: &[&str] = &["/health", "/auth/login", "/auth/status", "/auth/generate-key", "/webhook/github"];

pub async fn auth_middleware(headers: HeaderMap, request: Request, next: Next) -> Response {
    if !auth_enabled() { return next.run(request).await; }

    let path = request.uri().path();
    if PUBLIC.iter().any(|p| path == *p) { return next.run(request).await; }

    let token = headers.get("X-API-Key")
        .or_else(|| headers.get("Authorization"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").trim())
        .unwrap_or("");

    if token.is_empty() || !verify_token(token) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized — provide X-API-Key header"}))).into_response();
    }

    next.run(request).await
}

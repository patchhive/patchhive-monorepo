use axum::{
    extract::Request,
    http::{
        header::{AUTHORIZATION, RETRY_AFTER},
        HeaderMap, HeaderValue, Method, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const DEFAULT_WINDOW_SECS: u64 = 60;
const DEFAULT_STANDARD_MAX: usize = 300;
const DEFAULT_SENSITIVE_MAX: usize = 30;

#[derive(Debug, Clone, Copy)]
struct RateLimitConfig {
    window: Duration,
    standard_max: usize,
    sensitive_max: usize,
}

impl RateLimitConfig {
    fn from_env() -> Self {
        Self {
            window: Duration::from_secs(env_u64(
                "PATCHHIVE_RATE_LIMIT_WINDOW_SECS",
                DEFAULT_WINDOW_SECS,
            )),
            standard_max: env_usize("PATCHHIVE_RATE_LIMIT_MAX", DEFAULT_STANDARD_MAX),
            sensitive_max: env_usize("PATCHHIVE_RATE_LIMIT_SENSITIVE_MAX", DEFAULT_SENSITIVE_MAX),
        }
    }

    fn max_for(self, sensitive: bool) -> usize {
        if sensitive {
            self.sensitive_max
        } else {
            self.standard_max
        }
    }
}

struct RateLimiter {
    config: RateLimitConfig,
    buckets: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl RateLimiter {
    fn from_env() -> Self {
        Self::new(RateLimitConfig::from_env())
    }

    fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    fn check(&self, key: String, sensitive: bool) -> Result<(), u64> {
        let limit = self.config.max_for(sensitive);
        let now = Instant::now();
        let window = self.config.window;

        let Ok(mut buckets) = self.buckets.lock() else {
            tracing::warn!("rate limiter lock poisoned; allowing request");
            return Ok(());
        };

        let bucket = buckets.entry(key).or_default();
        while bucket
            .front()
            .is_some_and(|seen| now.duration_since(*seen) >= window)
        {
            bucket.pop_front();
        }

        if bucket.len() >= limit {
            let retry_after = bucket
                .front()
                .map(|seen| {
                    window
                        .saturating_sub(now.duration_since(*seen))
                        .as_secs()
                        .max(1)
                })
                .unwrap_or(DEFAULT_WINDOW_SECS);
            return Err(retry_after);
        }

        bucket.push_back(now);
        // W3: periodic sweep of empty buckets to prevent unbounded memory growth.
        if buckets.len() % 100 == 0 {
            buckets.retain(|_, v| !v.is_empty());
        }
        Ok(())
    }
}

fn limiter() -> &'static RateLimiter {
    static LIMITER: OnceLock<RateLimiter> = OnceLock::new();
    LIMITER.get_or_init(RateLimiter::from_env)
}

pub async fn rate_limit_middleware(req: Request, next: Next) -> Response {
    let sensitive = is_sensitive_request(req.method(), req.uri().path());
    let identity = request_identity(req.headers());
    let bucket = if sensitive { "sensitive" } else { "standard" };
    let key = format!("{identity}:{bucket}");

    if let Err(retry_after_seconds) = limiter().check(key, sensitive) {
        tracing::warn!(
            path = req.uri().path(),
            method = %req.method(),
            retry_after_seconds,
            "PatchHive API rate limit exceeded"
        );
        return rate_limited_response(retry_after_seconds);
    }

    next.run(req).await
}

fn rate_limited_response(retry_after_seconds: u64) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "error": "rate_limited",
            "message": "Too many requests. Try again after the retry window.",
            "retry_after_seconds": retry_after_seconds,
        })),
    )
        .into_response();

    let retry_after = HeaderValue::from_str(&retry_after_seconds.to_string())
        .unwrap_or_else(|_| HeaderValue::from_static("60"));
    response.headers_mut().insert(RETRY_AFTER, retry_after);
    response
}

fn is_sensitive_request(method: &Method, path: &str) -> bool {
    path.starts_with("/auth/")
        || matches!(
            method,
            &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
        )
}

fn request_identity(headers: &HeaderMap) -> String {
    header_token(headers)
        .map(|token| format!("api:{}", hash_identity(&token)))
        .unwrap_or_else(|| {
            // W4: use IP-based identity for anonymous requests to avoid shared buckets.
            // Only trust x-forwarded-for when explicitly behind a trusted proxy.
            if let Some(ip) = client_ip(headers) {
                format!("anon:{}", hash_identity(&ip))
            } else {
                "anonymous".to_string()
            }
        })
}

/// Extract the client IP, respecting PATCHHIVE_TRUST_PROXY for x-forwarded-for.
fn client_ip(headers: &HeaderMap) -> Option<String> {
    if matches!(
        std::env::var("PATCHHIVE_TRUST_PROXY").ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "on")
    ) {
        if let Some(value) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
        {
            return Some(value.split(',').next()?.trim().to_string());
        }
    }
    None
}

fn header_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-api-key")
        .or_else(|| headers.get(AUTHORIZATION))
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim_start_matches("Bearer ").trim().to_string())
        .filter(|value| !value.is_empty())
}

fn hash_identity(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_and_mutating_requests_are_sensitive() {
        assert!(is_sensitive_request(&Method::GET, "/auth/status"));
        assert!(is_sensitive_request(&Method::POST, "/scan"));
        assert!(is_sensitive_request(&Method::DELETE, "/rules/example"));
        assert!(!is_sensitive_request(&Method::GET, "/health"));
        assert!(!is_sensitive_request(&Method::GET, "/runs"));
    }

    #[test]
    fn limiter_blocks_after_window_capacity() {
        let limiter = RateLimiter::new(RateLimitConfig {
            window: Duration::from_secs(60),
            standard_max: 2,
            sensitive_max: 1,
        });

        assert!(limiter.check("anonymous:standard".into(), false).is_ok());
        assert!(limiter.check("anonymous:standard".into(), false).is_ok());
        assert!(limiter.check("anonymous:standard".into(), false).is_err());

        assert!(limiter.check("anonymous:sensitive".into(), true).is_ok());
        let retry_after = limiter
            .check("anonymous:sensitive".into(), true)
            .expect_err("sensitive bucket should be full");
        assert!(retry_after >= 1);
    }

    #[test]
    fn api_key_identity_is_hashed() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("secret-key"));

        let identity = request_identity(&headers);

        assert!(identity.starts_with("api:"));
        assert!(!identity.contains("secret-key"));
    }
}

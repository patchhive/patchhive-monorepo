use axum::{
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use once_cell::sync::Lazy;
use patchhive_product_core::auth::{
    auth_enabled as core_auth_enabled, auth_middleware as core_auth_middleware,
    generate_and_save_key as core_generate_and_save_key, verify_token as core_verify_token,
    ApiKeyAuthConfig,
};

static AUTH_CONFIG: Lazy<ApiKeyAuthConfig> = Lazy::new(|| {
    ApiKeyAuthConfig::new("REAPER_API_KEY_HASH", "rr-").with_public_paths([
        "/health",
        "/auth/login",
        "/auth/status",
        "/auth/generate-key",
        "/webhook/github",
    ])
});

pub fn auth_enabled() -> bool {
    core_auth_enabled(&AUTH_CONFIG)
}

pub fn verify_token(token: &str) -> bool {
    core_verify_token(&AUTH_CONFIG, token)
}

pub fn generate_and_save_key() -> String {
    core_generate_and_save_key(&AUTH_CONFIG)
}

pub async fn auth_middleware(headers: HeaderMap, request: Request, next: Next) -> Response {
    core_auth_middleware(&AUTH_CONFIG, headers, request, next).await
}

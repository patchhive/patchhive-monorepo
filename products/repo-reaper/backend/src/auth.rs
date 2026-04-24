use anyhow::Result;
use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};
use once_cell::sync::Lazy;
use patchhive_product_core::auth::{
    auth_enabled as core_auth_enabled, auth_middleware as core_auth_middleware,
    auth_status_payload as core_auth_status_payload,
    bootstrap_request_allowed as core_bootstrap_request_allowed,
    generate_and_save_key as core_generate_and_save_key,
    generate_and_save_service_token as core_generate_and_save_service_token,
    rotate_and_save_service_token as core_rotate_and_save_service_token,
    service_auth_enabled as core_service_auth_enabled,
    service_token_generation_allowed as core_service_token_generation_allowed,
    service_token_rotation_allowed as core_service_token_rotation_allowed,
    verify_token as core_verify_token, ApiKeyAuthConfig,
};

static AUTH_CONFIG: Lazy<ApiKeyAuthConfig> = Lazy::new(|| {
    ApiKeyAuthConfig::new("REAPER_API_KEY_HASH", "rr-")
        .with_service_token("REAPER_SERVICE_TOKEN_HASH", "rr-svc-")
        .with_service_default_name("hivecore")
        .with_service_dispatch_paths(["/run", "/dry-run"])
        .with_unauthorized_message("Unauthorized — provide X-API-Key or X-PatchHive-Service-Token.")
        .with_public_paths([
            "/health",
            "/auth/login",
            "/auth/status",
            "/auth/generate-key",
            "/auth/generate-service-token",
            "/auth/rotate-service-token",
            "/startup/checks",
            "/capabilities",
            "/webhook/github",
        ])
});

pub fn auth_enabled() -> bool {
    core_auth_enabled(&AUTH_CONFIG)
}

pub fn verify_token(token: &str) -> bool {
    core_verify_token(&AUTH_CONFIG, token)
}

pub fn generate_and_save_key() -> Result<String> {
    core_generate_and_save_key(&AUTH_CONFIG)
}

pub fn service_auth_enabled() -> bool {
    core_service_auth_enabled(&AUTH_CONFIG)
}

pub fn generate_and_save_service_token() -> Result<String> {
    core_generate_and_save_service_token(&AUTH_CONFIG)
}

pub fn rotate_and_save_service_token() -> Result<String> {
    core_rotate_and_save_service_token(&AUTH_CONFIG)
}

pub fn auth_status_payload() -> serde_json::Value {
    core_auth_status_payload(&AUTH_CONFIG)
}

pub fn bootstrap_request_allowed(headers: &HeaderMap) -> bool {
    core_bootstrap_request_allowed(headers)
}

pub fn service_token_generation_allowed(headers: &HeaderMap) -> bool {
    core_service_token_generation_allowed(&AUTH_CONFIG, headers)
}

pub fn service_token_rotation_allowed(headers: &HeaderMap) -> bool {
    core_service_token_rotation_allowed(&AUTH_CONFIG, headers)
}

pub async fn auth_middleware(headers: HeaderMap, request: Request, next: Next) -> Response {
    core_auth_middleware(&AUTH_CONFIG, headers, request, next).await
}

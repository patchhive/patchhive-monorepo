patchhive_product_core::define_api_key_auth_module! {
    pub mod auth {
        patchhive_product_core::auth::ApiKeyAuthConfig::new("TRUST_API_KEY_HASH", "tg-")
            .with_service_token("TRUST_SERVICE_TOKEN_HASH", "tg-svc-")
            .with_service_default_name("hivecore")
            .with_service_dispatch_paths(["/review", "/review/github/pr", "/webhooks/github"])
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
                "/webhooks/github",
            ])
    }
}

mod db;
mod github;
mod models;
mod pipeline;
mod startup;
mod state;

use axum::{
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use patchhive_product_core::rate_limit::rate_limit_middleware;
use patchhive_product_core::startup::{
    cors_layer, count_errors, listen_addr, log_checks, StartupCheck,
};
use serde_json::json;
use tracing::info;

use crate::{
    auth::{
        auth_enabled, generate_and_save_key, generate_and_save_service_token,
        rotate_and_save_service_token, service_auth_enabled,
        service_token_generation_allowed_from_peer, service_token_rotation_allowed_from_peer,
        verify_token,
    },
    models::{report_template_variables, RepoRuleSet, ReportTemplateSet},
    state::AppState,
};

static STARTUP_CHECKS: OnceCell<Vec<StartupCheck>> = OnceCell::new();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let _ = dotenvy::dotenv();

    if let Err(err) = db::init_db() {
        eprintln!("DB init failed: {err}");
        std::process::exit(1);
    }

    let checks = startup::validate_config(&reqwest::Client::new()).await;
    log_checks(&checks);
    let _ = STARTUP_CHECKS.set(checks);

    let cors = cors_layer();

    let app = Router::new()
        .route("/auth/status", get(auth_status))
        .route("/auth/login", post(login))
        .route("/auth/generate-key", post(gen_key))
        .route("/auth/generate-service-token", post(gen_service_token))
        .route("/auth/rotate-service-token", post(rotate_service_token))
        .route("/health", get(health))
        .route("/startup/checks", get(startup_checks_route))
        .route("/capabilities", get(pipeline::capabilities))
        .route("/runs", get(pipeline::runs))
        .route("/runs/:id", get(pipeline::history_detail))
        .route("/rule-packs", get(pipeline::rule_packs))
        .route("/rules", get(list_rules).post(save_rules))
        .route("/rules/*repo", delete(delete_rules))
        .route("/templates", get(list_templates).post(save_templates))
        .route("/templates/*repo", delete(delete_templates))
        .route("/review", post(pipeline::review))
        .route("/review/github/pr", post(pipeline::review_github_pr))
        .route("/webhooks/github", post(pipeline::github_webhook))
        .route("/history", get(pipeline::history))
        .route("/history/:id", get(pipeline::history_detail))
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(cors)
        .with_state(AppState::new());

    let addr = listen_addr("TRUSTGATE_PORT", 8020);
    info!("🛡 TrustGate by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|err| panic!("failed to bind TrustGate to {addr}: {err}"));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap_or_else(|err| panic!("TrustGate server failed: {err}"));
}

async fn auth_status() -> Json<serde_json::Value> {
    Json(auth::auth_status_payload())
}

#[derive(serde::Deserialize)]
struct LoginBody {
    api_key: String,
}

async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth_enabled() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if !verify_token(&body.api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(
        json!({"ok": true, "auth_enabled": true, "auth_configured": true}),
    ))
}

async fn gen_key(
    headers: axum::http::HeaderMap,
    peer: Option<patchhive_product_core::auth::ClientConnectInfo>,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if auth_enabled() {
        return Err(patchhive_product_core::auth::auth_already_configured_error());
    }
    let peer_addr = patchhive_product_core::auth::peer_addr_from_connect_info(peer);
    if !auth::bootstrap_request_allowed_from_peer(&headers, peer_addr) {
        return Err(patchhive_product_core::auth::bootstrap_localhost_required_error());
    }
    let key = generate_and_save_key()
        .map_err(|err| patchhive_product_core::auth::key_generation_failed_error(&err))?;
    Ok(Json(
        json!({"api_key": key, "message": "Store this — it won't be shown again"}),
    ))
}

async fn gen_service_token(
    headers: axum::http::HeaderMap,
    peer: Option<patchhive_product_core::auth::ClientConnectInfo>,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if service_auth_enabled() {
        return Err(patchhive_product_core::auth::service_auth_already_configured_error());
    }
    let peer_addr = patchhive_product_core::auth::peer_addr_from_connect_info(peer);
    if !service_token_generation_allowed_from_peer(&headers, peer_addr) {
        return Err(patchhive_product_core::auth::service_token_generation_forbidden_error());
    }
    let token = generate_and_save_service_token()
        .map_err(|err| patchhive_product_core::auth::service_token_generation_failed_error(&err))?;
    Ok(Json(json!({
        "service_token": token,
        "message": "Store this for HiveCore or other PatchHive service callers — it won't be shown again"
    })))
}

async fn rotate_service_token(
    headers: axum::http::HeaderMap,
    peer: Option<patchhive_product_core::auth::ClientConnectInfo>,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if !service_auth_enabled() {
        return Err(patchhive_product_core::auth::service_auth_not_configured_error());
    }
    let peer_addr = patchhive_product_core::auth::peer_addr_from_connect_info(peer);
    if !service_token_rotation_allowed_from_peer(&headers, peer_addr) {
        return Err(patchhive_product_core::auth::service_token_rotation_forbidden_error());
    }
    let token = rotate_and_save_service_token()
        .map_err(|err| patchhive_product_core::auth::service_token_rotation_failed_error(&err))?;
    Ok(Json(json!({
        "service_token": token,
        "message": "Store this replacement service token for HiveCore or other PatchHive service callers — it won't be shown again"
    })))
}

async fn health() -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();
    let reviews = db::list_reviews().unwrap_or_default();
    let github_verified = STARTUP_CHECKS
        .get()
        .map(|checks| patchhive_product_core::github_permissions::github_token_verified(checks))
        .unwrap_or(false);

    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "TrustGate by PatchHive",
        "review_count": db::review_count(),
        "rules_count": db::rule_count(),
        "template_count": db::template_count(),
        "repo_count": pipeline::unique_repos(&reviews),
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "mode": "review-first",
        "github_ready": github_verified,
        "github": {
            "token_configured": github::github_token_configured(),
            "token_verified": github_verified,
            "webhook_secret_configured": github::webhook_secret_configured(),
            "public_url_configured": std::env::var("TRUSTGATE_PUBLIC_URL")
                .ok()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false),
        }
    }))
}

async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

async fn list_rules() -> Json<serde_json::Value> {
    Json(json!({
        "rules": db::list_rules().unwrap_or_default(),
    }))
}

async fn list_templates() -> Json<serde_json::Value> {
    Json(json!({
        "templates": db::list_report_templates().unwrap_or_default(),
        "defaults": ReportTemplateSet::default(),
        "variables": report_template_variables(),
    }))
}

async fn save_rules(
    Json(mut body): Json<RepoRuleSet>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(repo) = db::normalize_repo_name(&body.repo) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    body.repo = repo.clone();
    db::save_rules(&repo, &body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true, "repo": repo })))
}

async fn save_templates(
    Json(mut body): Json<ReportTemplateSet>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(repo) = db::normalize_repo_name(&body.repo) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    if body.check_title_template.trim().is_empty()
        || body.check_summary_template.trim().is_empty()
        || body.comment_template.trim().is_empty()
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    body.repo = repo.clone();
    db::save_report_templates(&repo, &body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true, "repo": repo })))
}

async fn delete_rules(
    axum::extract::Path(repo): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(repo) = db::normalize_repo_name(&repo) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    db::delete_rules(&repo).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true })))
}

async fn delete_templates(
    axum::extract::Path(repo): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(repo) = db::normalize_repo_name(&repo) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    db::delete_report_templates(&repo).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true })))
}

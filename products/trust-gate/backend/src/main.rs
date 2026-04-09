mod auth;
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
use patchhive_product_core::startup::{count_errors, log_checks, StartupCheck};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::{
    auth::{auth_enabled, generate_and_save_key, verify_token},
    models::RepoRuleSet,
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

    let checks = startup::validate_config().await;
    log_checks(&checks);
    let _ = STARTUP_CHECKS.set(checks);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/auth/status", get(auth_status))
        .route("/auth/login", post(login))
        .route("/auth/generate-key", post(gen_key))
        .route("/health", get(health))
        .route("/startup/checks", get(startup_checks_route))
        .route("/rule-packs", get(pipeline::rule_packs))
        .route("/rules", get(list_rules).post(save_rules))
        .route("/rules/*repo", delete(delete_rules))
        .route("/review", post(pipeline::review))
        .route("/review/github/pr", post(pipeline::review_github_pr))
        .route("/webhooks/github", post(pipeline::github_webhook))
        .route("/history", get(pipeline::history))
        .route("/history/:id", get(pipeline::history_detail))
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(cors)
        .with_state(AppState::new());

    let addr = "0.0.0.0:8000";
    info!("🛡 TrustGate by PatchHive — listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn auth_status() -> Json<serde_json::Value> {
    Json(json!({"auth_enabled": auth_enabled()}))
}

#[derive(serde::Deserialize)]
struct LoginBody {
    api_key: String,
}

async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !verify_token(&body.api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(json!({"ok": true, "auth_enabled": true})))
}

async fn gen_key() -> Result<Json<serde_json::Value>, StatusCode> {
    if auth_enabled() {
        return Err(StatusCode::FORBIDDEN);
    }
    let key = generate_and_save_key();
    Ok(Json(json!({"api_key": key, "message": "Store this — it won't be shown again"})))
}

async fn health() -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS.get().map(|checks| count_errors(checks)).unwrap_or(0);
    let reviews = db::list_reviews().unwrap_or_default();

    Json(json!({
        "status": if errors > 0 { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "TrustGate by PatchHive",
        "review_count": db::review_count(),
        "rules_count": db::rule_count(),
        "repo_count": pipeline::unique_repos(&reviews),
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_path": db::db_path(),
        "mode": "review-first",
        "github": {
            "token_configured": github::github_token_configured(),
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

async fn save_rules(Json(mut body): Json<RepoRuleSet>) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(repo) = db::normalize_repo_name(&body.repo) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    body.repo = repo.clone();
    db::save_rules(&repo, &body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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

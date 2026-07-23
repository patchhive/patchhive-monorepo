pub mod agents;
pub mod ai_local;
patchhive_product_core::define_api_key_auth_module! {
    pub mod auth {
        patchhive_product_core::auth::ApiKeyAuthConfig::new("REAPER_API_KEY_HASH", "rr-")
            .with_service_token("REAPER_SERVICE_TOKEN_HASH", "rr-svc-")
            .with_service_default_name("hivecore")
            .with_service_dispatch_paths([
                "/run",
                "/dry-run",
                "/api/products/repo-reaper/run",
                "/api/products/repo-reaper/dry-run",
            ])
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
                "/api/products/repo-reaper/health",
                "/api/products/repo-reaper/auth/login",
                "/api/products/repo-reaper/auth/status",
                "/api/products/repo-reaper/auth/generate-key",
                "/api/products/repo-reaper/auth/generate-service-token",
                "/api/products/repo-reaper/auth/rotate-service-token",
                "/api/products/repo-reaper/startup/checks",
                "/api/products/repo-reaper/capabilities",
                "/api/products/repo-reaper/webhook/github",
            ])
    }
}

pub mod db;
pub mod fix_worker;
pub mod git_ops;
pub mod github;
pub mod pipeline;
pub mod routes;
pub mod startup;
pub mod state;

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use patchhive_product_core::{
    contract,
    rate_limit::rate_limit_middleware,
    startup::{count_errors, log_checks, StartupCheck},
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
    state::AppState,
};

pub static STARTUP_CHECKS: OnceCell<Vec<StartupCheck>> = OnceCell::new();
static APP_STATE: OnceCell<AppState> = OnceCell::new();

pub async fn init_runtime() -> Result<()> {
    if APP_STATE.get().is_some() {
        return Ok(());
    }

    db::init_db()?;
    match db::migrate_openrouter_provider_storage() {
        Ok(count) if count > 0 => info!(count, "migrated OpenRouter-backed RepoReaper agents"),
        Ok(_) => {}
        Err(err) => tracing::warn!("RepoReaper OpenRouter provider migration failed: {err}"),
    }
    if let Err(err) = db::migrate_agent_secret_storage() {
        tracing::warn!("RepoReaper agent secret migration failed: {err}");
    }

    let orphans = db::recover_orphaned_runs();
    if !orphans.is_empty() {
        tracing::warn!("Recovered {} orphaned run(s): {:?}", orphans.len(), orphans);
    }

    let state = AppState::new();
    restore_runtime_state(&state).await;

    let checks = startup::validate_config(&state.http).await;
    log_checks(&checks);
    let _ = STARTUP_CHECKS.set(checks);

    let http = state.http.clone();
    let scheduler_state = state.clone();
    APP_STATE
        .set(state)
        .map_err(|_| anyhow::anyhow!("RepoReaper runtime initialized concurrently"))?;
    tokio::spawn(startup::pr_poll_loop(http));
    tokio::spawn(routes::webhook::scheduler_loop(scheduler_state));
    Ok(())
}

async fn restore_runtime_state(state: &AppState) {
    match db::load_active_agents() {
        Ok(agents) if !agents.is_empty() => {
            let mut map = state.agents.write().await;
            for agent in agents {
                map.insert(agent.id.clone(), agent);
            }
            info!("restored {} RepoReaper agent(s) from SQLite", map.len());
        }
        Ok(_) => {}
        Err(err) => tracing::warn!("failed to restore RepoReaper active agent team: {err}"),
    }

    if db::get_setting("watch_mode", "false") == "true" {
        state
            .watch_mode
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

pub fn router() -> Router {
    let state = APP_STATE.get().cloned().unwrap_or_else(AppState::new);
    Router::new()
        .route("/auth/status", get(auth_status))
        .route("/auth/login", post(login))
        .route("/auth/generate-key", post(gen_key))
        .route("/auth/generate-service-token", post(gen_service_token))
        .route("/auth/rotate-service-token", post(rotate_service_token))
        .route("/health", get(health))
        .route("/startup/checks", get(startup_checks_route))
        .route("/capabilities", get(capabilities))
        .route("/run", post(pipeline::run))
        .route("/dry-run", post(pipeline::dry_run))
        .merge(routes::config::router())
        .merge(routes::history::router())
        .merge(routes::webhook::router())
        .layer(middleware::from_fn(auth::auth_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .with_state(state)
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

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let agents_count = state.agents.read().await.len();
    let worker_capacity_available = state.process_worker_semaphore.available_permits();
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();
    let github_verified = STARTUP_CHECKS
        .get()
        .map(|checks| patchhive_product_core::github_permissions::github_token_verified(checks))
        .unwrap_or(false);
    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "RepoReaper by PatchHive",
        "bot": std::env::var("BOT_GITHUB_USER").unwrap_or_else(|_| "(not set)".into()),
        "agents": agents_count,
        "run_active": state.run_active.load(std::sync::atomic::Ordering::SeqCst),
        "worker_capacity": {
            "limit": state.process_worker_limit,
            "available": worker_capacity_available,
            "active": state.process_worker_limit.saturating_sub(worker_capacity_available),
        },
        "watch_mode": state.watch_mode.load(std::sync::atomic::Ordering::SeqCst),
        "lifetime_cost": db::get_lifetime_cost(),
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "github_ready": github_verified,
        "github": {
            "token_configured": patchhive_product_core::github_auth::github_write_token_configured(
                patchhive_product_core::github_auth::REPO_REAPER_GITHUB_TOKEN_RW,
            ),
            "token_verified": github_verified,
        },
    }))
}

async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

async fn capabilities() -> Json<contract::ProductCapabilities> {
    Json(contract::capabilities(
        "repo-reaper",
        "RepoReaper",
        vec![
            contract::action(
                "run",
                "Run patch hunt",
                "POST",
                "/run",
                "Find candidate issues, generate fixes, validate them, and open pull requests.",
                true,
            )
            .mutating(true)
            .requires_approval(true)
            .scheduleable(true)
            .opens_pr(true)
            .target_selection_modes([
                contract::TargetSelectionMode::Direct,
                contract::TargetSelectionMode::Discovery,
            ])
            .credential_requirements([
                "github:contents:write",
                "github:pull_requests:write",
                "provider:ai",
            ]),
            contract::action(
                "dry_run",
                "Run dry stalk",
                "POST",
                "/dry-run",
                "Discover and score candidate work without writing patches or opening pull requests.",
                true,
            )
            .read_only(true)
            .scheduleable(true)
            .target_selection_modes([
                contract::TargetSelectionMode::Direct,
                contract::TargetSelectionMode::Discovery,
            ])
            .credential_requirements(["github:issues:read", "provider:ai"]),
        ],
        vec![
            contract::link("history", "History", "/history"),
            contract::link("leaderboard", "Leaderboard", "/leaderboard"),
            contract::link("rejected", "Rejected patches", "/rejected"),
            contract::link("pr_tracking", "PR tracking", "/pr-tracking"),
        ],
    ))
}

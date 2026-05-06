use std::{collections::HashMap, env, time::Duration};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::sleep;

use crate::{
    db,
    models::{
        ok, FirstStackSetupResponse, SetupLauncherProductStatus, SetupLauncherStatus,
        SetupProductCredentialRequirements, SetupProductLogsResponse, SetupProductStatus,
    },
    state::{product_catalog, AppState},
};

use super::overview::build_runtime_products;
use super::provision::provision_service_token_for_product;
use super::{api_error, fetch_product_auth_status, pick_url};

pub(super) const DOWNSTREAM_FIRST_STACK_SLUGS: [&str; 3] =
    ["signal-hive", "trust-gate", "repo-reaper"];
const FIRST_STACK_SLUGS: [&str; 4] = ["signal-hive", "trust-gate", "repo-reaper", "hive-core"];

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LauncherStackStatusBody {
    launcher_available: bool,
    message: String,
    repo_root: String,
    docker_available: bool,
    docker_compose_available: bool,
    products: Vec<SetupLauncherProductStatus>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LauncherActionBody {
    ok: bool,
    actions: Vec<String>,
    products: Vec<SetupLauncherProductStatus>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LauncherRequirementsBody {
    stack_id: String,
    products: Vec<SetupProductCredentialRequirements>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LauncherEnvWriteBody {
    ok: bool,
    actions: Vec<String>,
    product: SetupProductCredentialRequirements,
}

#[derive(Debug, Clone, Serialize)]
struct LauncherStartRequest<'a> {
    suite_bootstrap_secret: &'a str,
    products: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LauncherStopRequest {
    remove: bool,
}

#[derive(Debug, Clone, Serialize)]
struct LauncherProductActionRequest<'a> {
    suite_bootstrap_secret: &'a str,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetupProductEnvRequest {
    #[serde(default)]
    pub values: HashMap<String, String>,
    #[serde(default)]
    pub restart: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubTokenValidationRequest {
    pub token: String,
    #[serde(default)]
    pub expected_user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubTokenValidationResponse {
    pub ok: bool,
    pub login: String,
    pub user_matches: bool,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductLogsQuery {
    pub tail: Option<u16>,
}

#[derive(Debug, Clone)]
struct LauncherSnapshot {
    status: SetupLauncherStatus,
    products: HashMap<String, SetupLauncherProductStatus>,
}

#[derive(Debug, Clone)]
struct LauncherRequirementsSnapshot {
    products: HashMap<String, SetupProductCredentialRequirements>,
}

pub(super) async fn first_stack_status(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<FirstStackSetupResponse>> {
    Json(ok(build_first_stack_response(&state, Vec::new()).await))
}

pub(super) async fn start_first_stack(
    State(state): State<AppState>,
) -> Result<
    Json<crate::models::ApiEnvelope<FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    let mut actions = Vec::new();
    prepare_first_stack_for_verification(&state, &mut actions).await?;

    Ok(Json(ok(build_first_stack_response(&state, actions).await)))
}

pub(super) async fn pair_first_stack(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<FirstStackSetupResponse>> {
    let mut actions = Vec::new();
    if let Some(secret) = configured_suite_bootstrap_secret() {
        auto_pair_first_stack(&state, &secret, &mut actions).await;
    } else {
        actions.push(
            "HiveCore does not have PATCHHIVE_SUITE_BOOTSTRAP_SECRET configured yet, so automatic pairing is not ready."
                .into(),
        );
    }

    Json(ok(build_first_stack_response(&state, actions).await))
}

pub(super) async fn stop_first_stack(
    State(state): State<AppState>,
) -> Result<
    Json<crate::models::ApiEnvelope<FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    let launcher_base_url = launcher_base_url();
    let launcher = stop_launcher_stack(&state, &launcher_base_url).await?;
    Ok(Json(ok(build_first_stack_response(
        &state,
        launcher.actions,
    )
    .await)))
}

pub(super) async fn start_setup_product(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<
    Json<crate::models::ApiEnvelope<FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    ensure_launcher_product_slug(&slug)?;
    let secret = ensure_suite_bootstrap_secret();
    let launcher = run_launcher_product_action(&state, &slug, "start", Some(&secret)).await?;
    let mut actions = launcher.actions;
    wait_for_products(&state, &[slug.as_str()], &mut actions).await;
    auto_pair_products(&state, &secret, &[slug.as_str()], &mut actions).await;
    Ok(Json(ok(build_first_stack_response(&state, actions).await)))
}

pub(super) async fn stop_setup_product(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<
    Json<crate::models::ApiEnvelope<FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    ensure_launcher_product_slug(&slug)?;
    let launcher = run_launcher_product_action(&state, &slug, "stop", None).await?;
    Ok(Json(ok(build_first_stack_response(
        &state,
        launcher.actions,
    )
    .await)))
}

pub(super) async fn restart_setup_product(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<
    Json<crate::models::ApiEnvelope<FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    ensure_launcher_product_slug(&slug)?;
    let secret = ensure_suite_bootstrap_secret();
    let launcher = run_launcher_product_action(&state, &slug, "restart", Some(&secret)).await?;
    let mut actions = launcher.actions;
    wait_for_products(&state, &[slug.as_str()], &mut actions).await;
    auto_pair_products(&state, &secret, &[slug.as_str()], &mut actions).await;
    Ok(Json(ok(build_first_stack_response(&state, actions).await)))
}

pub(super) async fn setup_product_logs(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<ProductLogsQuery>,
) -> Result<
    Json<crate::models::ApiEnvelope<SetupProductLogsResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    ensure_launcher_product_slug(&slug)?;
    let tail = query.tail.unwrap_or(120).clamp(20, 500);
    let url = format!(
        "{}/products/{}/logs?tail={}",
        launcher_base_url().trim_end_matches('/'),
        slug,
        tail
    );
    let response = state
        .client
        .get(url)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|_| {
            api_error(
                StatusCode::BAD_GATEWAY,
                "launcher_unavailable",
                "HiveCore could not reach patchhive-launcher to read product logs.",
            )
        })?;

    if !response.status().is_success() {
        return Err(
            launcher_rejected(response, "patchhive-launcher could not read product logs.").await,
        );
    }

    let body = response
        .json::<SetupProductLogsResponse>()
        .await
        .map_err(|_| {
            api_error(
                StatusCode::BAD_GATEWAY,
                "launcher_invalid_response",
                "HiveCore could not parse the launcher logs response.",
            )
        })?;

    Ok(Json(ok(body)))
}

pub(super) async fn save_setup_product_env(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(body): Json<SetupProductEnvRequest>,
) -> Result<
    Json<crate::models::ApiEnvelope<FirstStackSetupResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    ensure_launcher_product_slug(&slug)?;
    if body.values.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "missing_setup_credentials",
            "Provide at least one setup credential value to save.",
        ));
    }

    let launcher = write_launcher_product_env(&state, &slug, &body.values).await?;
    let mut actions = launcher.actions;
    if body.restart {
        let secret = ensure_suite_bootstrap_secret();
        let restarted =
            run_launcher_product_action(&state, &slug, "restart", Some(&secret)).await?;
        actions.extend(restarted.actions);
        wait_for_products(&state, &[slug.as_str()], &mut actions).await;
        auto_pair_products(&state, &secret, &[slug.as_str()], &mut actions).await;
    }

    Ok(Json(ok(build_first_stack_response(&state, actions).await)))
}

pub(super) async fn validate_github_token(
    State(state): State<AppState>,
    Json(body): Json<GitHubTokenValidationRequest>,
) -> Result<
    Json<crate::models::ApiEnvelope<GitHubTokenValidationResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    let token = body.token.trim();
    if token.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "missing_github_token",
            "Provide a GitHub token to validate.",
        ));
    }

    let response = state
        .client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "hive-core/0.1")
        .timeout(Duration::from_secs(12))
        .send()
        .await
        .map_err(|_| {
            api_error(
                StatusCode::BAD_GATEWAY,
                "github_unreachable",
                "HiveCore could not reach GitHub to validate the token.",
            )
        })?;

    if response.status() == StatusCode::UNAUTHORIZED {
        return Ok(Json(ok(GitHubTokenValidationResponse {
            ok: false,
            login: String::new(),
            user_matches: false,
            message: "GitHub rejected this token as invalid or expired.".into(),
        })));
    }

    if !response.status().is_success() {
        return Ok(Json(ok(GitHubTokenValidationResponse {
            ok: false,
            login: String::new(),
            user_matches: false,
            message: format!(
                "GitHub returned HTTP {} while validating the token.",
                response.status()
            ),
        })));
    }

    let payload = response.json::<Value>().await.unwrap_or_default();
    let login = payload
        .get("login")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let expected = body
        .expected_user
        .unwrap_or_default()
        .trim()
        .trim_start_matches('@')
        .to_string();
    let user_matches = expected.is_empty() || expected.eq_ignore_ascii_case(&login);
    let message = if user_matches {
        format!("GitHub token is valid and authenticates as @{login}.")
    } else {
        format!("GitHub token is valid as @{login}, but the expected username was @{expected}.")
    };

    Ok(Json(ok(GitHubTokenValidationResponse {
        ok: true,
        login,
        user_matches,
        message,
    })))
}

pub(super) async fn build_first_stack_response(
    state: &AppState,
    actions: Vec<String>,
) -> FirstStackSetupResponse {
    let runtimes = build_runtime_products(state).await;
    let launcher = fetch_launcher_snapshot(state)
        .await
        .unwrap_or_else(|message| LauncherSnapshot {
            status: SetupLauncherStatus {
                available: false,
                message,
                repo_root: String::new(),
                docker_available: false,
                docker_compose_available: false,
            },
            products: HashMap::new(),
        });
    let requirements = fetch_launcher_requirements(state)
        .await
        .unwrap_or_else(|_| LauncherRequirementsSnapshot {
            products: HashMap::new(),
        });

    let mut products = Vec::new();
    for runtime in runtimes
        .into_iter()
        .filter(|item| FIRST_STACK_SLUGS.contains(&item.slug.as_str()))
    {
        let (auth_status, auth_status_error) =
            match fetch_product_auth_status(&state.client, &runtime.api_url).await {
                Ok(status) => (Some(status), String::new()),
                Err(message) => (None, message),
            };
        let needs_pair = !runtime.service_token_configured || runtime.legacy_api_key_configured;
        let pairing_ready = needs_pair
            && runtime.slug != "hive-core"
            && runtime.enabled
            && matches!(runtime.status.as_str(), "online" | "degraded")
            && auth_status
                .as_ref()
                .map(|status| {
                    status.service_auth_supported
                        && (status.suite_bootstrap_enabled || !status.auth_enabled)
                })
                .unwrap_or(false);

        products.push(SetupProductStatus {
            credentials: requirements
                .products
                .get(&runtime.slug)
                .map(|item| item.requirements.clone())
                .unwrap_or_default(),
            launcher: launcher.products.get(&runtime.slug).cloned(),
            runtime,
            auth_status,
            auth_status_error,
            pairing_ready,
        });
    }

    FirstStackSetupResponse {
        stack_id: "first-stack".into(),
        launcher: launcher.status,
        suite_bootstrap_configured: configured_suite_bootstrap_secret().is_some(),
        latest_smoke: db::latest_first_stack_smoke_run(),
        actions,
        products,
    }
}

pub(super) async fn prepare_first_stack_for_verification(
    state: &AppState,
    actions: &mut Vec<String>,
) -> Result<(), (StatusCode, Json<crate::models::ApiEnvelope<Value>>)> {
    let secret = ensure_suite_bootstrap_secret();
    let launcher_base_url = launcher_base_url();
    let products_to_start = downstream_products_to_start(state).await;
    if products_to_start.is_empty() {
        actions.push(
            "All first-stack downstream products already look reachable, so HiveCore skipped launcher start and moved straight to verification."
                .into(),
        );
    } else {
        let launcher =
            start_launcher_stack(state, &launcher_base_url, &secret, products_to_start).await?;
        actions.extend(launcher.actions);
    }

    wait_for_first_stack(state, actions).await;
    auto_pair_first_stack(state, &secret, actions).await;

    Ok(())
}

fn launcher_base_url() -> String {
    env::var("PATCHHIVE_LAUNCHER_URL").unwrap_or_else(|_| "http://localhost:8210".into())
}

fn configured_suite_bootstrap_secret() -> Option<String> {
    env::var("PATCHHIVE_SUITE_BOOTSTRAP_SECRET")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn ensure_suite_bootstrap_secret() -> String {
    if let Some(secret) = configured_suite_bootstrap_secret() {
        return secret;
    }

    let generated = format!("ph-suite-{}", uuid::Uuid::new_v4().simple());
    env::set_var("PATCHHIVE_SUITE_BOOTSTRAP_SECRET", &generated);
    generated
}

async fn fetch_launcher_snapshot(state: &AppState) -> Result<LauncherSnapshot, String> {
    let url = format!("{}/stacks/first", launcher_base_url().trim_end_matches('/'));
    let response = state
        .client
        .get(url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|err| format!("HiveCore could not reach patchhive-launcher: {err}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "patchhive-launcher returned HTTP {} for /stacks/first.",
            response.status()
        ));
    }

    let body = response
        .json::<LauncherStackStatusBody>()
        .await
        .map_err(|err| format!("HiveCore could not parse patchhive-launcher status: {err}"))?;

    let products = body
        .products
        .into_iter()
        .map(|product| (product.slug.clone(), product))
        .collect();

    Ok(LauncherSnapshot {
        status: SetupLauncherStatus {
            available: body.launcher_available,
            message: body.message,
            repo_root: body.repo_root,
            docker_available: body.docker_available,
            docker_compose_available: body.docker_compose_available,
        },
        products,
    })
}

async fn fetch_launcher_requirements(
    state: &AppState,
) -> Result<LauncherRequirementsSnapshot, String> {
    let url = format!(
        "{}/setup/requirements",
        launcher_base_url().trim_end_matches('/')
    );
    let response = state
        .client
        .get(url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|err| {
            format!("HiveCore could not reach patchhive-launcher requirements: {err}")
        })?;

    if !response.status().is_success() {
        return Err(format!(
            "patchhive-launcher returned HTTP {} for /setup/requirements.",
            response.status()
        ));
    }

    let body = response
        .json::<LauncherRequirementsBody>()
        .await
        .map_err(|err| format!("HiveCore could not parse launcher requirements: {err}"))?;
    let products = body
        .products
        .into_iter()
        .map(|product| (product.slug.clone(), product))
        .collect();

    Ok(LauncherRequirementsSnapshot { products })
}

async fn write_launcher_product_env(
    state: &AppState,
    slug: &str,
    values: &HashMap<String, String>,
) -> Result<LauncherEnvWriteBody, (StatusCode, Json<crate::models::ApiEnvelope<Value>>)> {
    let url = format!(
        "{}/setup/env/{}",
        launcher_base_url().trim_end_matches('/'),
        slug
    );
    let response = state
        .client
        .post(url)
        .json(&serde_json::json!({ "values": values }))
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|_| {
            api_error(
                StatusCode::BAD_GATEWAY,
                "launcher_unavailable",
                format!("HiveCore could not reach patchhive-launcher to save setup credentials for {slug}."),
            )
        })?;

    if !response.status().is_success() {
        return Err(launcher_rejected(
            response,
            "patchhive-launcher could not save setup credentials.",
        )
        .await);
    }

    response.json::<LauncherEnvWriteBody>().await.map_err(|_| {
        api_error(
            StatusCode::BAD_GATEWAY,
            "launcher_invalid_response",
            "HiveCore could not parse the launcher setup credential response.",
        )
    })
}

async fn start_launcher_stack(
    state: &AppState,
    launcher_base_url: &str,
    secret: &str,
    products: Vec<String>,
) -> Result<LauncherActionBody, (StatusCode, Json<crate::models::ApiEnvelope<Value>>)> {
    let url = format!(
        "{}/stacks/first/start",
        launcher_base_url.trim_end_matches('/')
    );
    let response = state
        .client
        .post(url)
        .json(&LauncherStartRequest {
            suite_bootstrap_secret: secret,
            products,
        })
        .timeout(Duration::from_secs(180))
        .send()
        .await
        .map_err(|_| {
            api_error(
                StatusCode::BAD_GATEWAY,
                "launcher_unavailable",
                "HiveCore could not reach patchhive-launcher to start the first stack.",
            )
        })?;

    if !response.status().is_success() {
        let error = response
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "patchhive-launcher could not start the first stack.".into());
        return Err(api_error(
            StatusCode::BAD_GATEWAY,
            "launcher_rejected",
            error,
        ));
    }

    response.json::<LauncherActionBody>().await.map_err(|_| {
        api_error(
            StatusCode::BAD_GATEWAY,
            "launcher_invalid_response",
            "HiveCore could not parse the launcher start response.",
        )
    })
}

async fn run_launcher_product_action(
    state: &AppState,
    slug: &str,
    action: &str,
    suite_bootstrap_secret: Option<&str>,
) -> Result<LauncherActionBody, (StatusCode, Json<crate::models::ApiEnvelope<Value>>)> {
    let url = format!(
        "{}/products/{}/{}",
        launcher_base_url().trim_end_matches('/'),
        slug,
        action
    );
    let mut request = state.client.post(url).timeout(Duration::from_secs(180));
    request = if let Some(secret) = suite_bootstrap_secret {
        request.json(&LauncherProductActionRequest {
            suite_bootstrap_secret: secret,
        })
    } else {
        request.json(&serde_json::json!({}))
    };

    let response = request.send().await.map_err(|_| {
        api_error(
            StatusCode::BAD_GATEWAY,
            "launcher_unavailable",
            format!("HiveCore could not reach patchhive-launcher to {action} {slug}."),
        )
    })?;

    if !response.status().is_success() {
        return Err(launcher_rejected(
            response,
            &format!("patchhive-launcher could not {action} {slug}."),
        )
        .await);
    }

    response.json::<LauncherActionBody>().await.map_err(|_| {
        api_error(
            StatusCode::BAD_GATEWAY,
            "launcher_invalid_response",
            format!("HiveCore could not parse the launcher {action} response."),
        )
    })
}

fn ensure_launcher_product_slug(
    slug: &str,
) -> Result<(), (StatusCode, Json<crate::models::ApiEnvelope<Value>>)> {
    if DOWNSTREAM_FIRST_STACK_SLUGS.contains(&slug) {
        Ok(())
    } else {
        Err(api_error(
            StatusCode::BAD_REQUEST,
            "unsupported_setup_product",
            "HiveCore setup controls currently manage SignalHive, TrustGate, and RepoReaper. HiveCore itself stays self-hosted.",
        ))
    }
}

async fn downstream_products_to_start(state: &AppState) -> Vec<String> {
    build_runtime_products(state)
        .await
        .into_iter()
        .filter(|item| item.slug != "hive-core")
        .filter(|item| item.enabled)
        .filter(|item| !matches!(item.status.as_str(), "online" | "degraded"))
        .map(|item| item.slug)
        .collect()
}

async fn stop_launcher_stack(
    state: &AppState,
    launcher_base_url: &str,
) -> Result<LauncherActionBody, (StatusCode, Json<crate::models::ApiEnvelope<Value>>)> {
    let url = format!(
        "{}/stacks/first/stop",
        launcher_base_url.trim_end_matches('/')
    );
    let response = state
        .client
        .post(url)
        .json(&LauncherStopRequest { remove: false })
        .timeout(Duration::from_secs(90))
        .send()
        .await
        .map_err(|_| {
            api_error(
                StatusCode::BAD_GATEWAY,
                "launcher_unavailable",
                "HiveCore could not reach patchhive-launcher to stop the first stack.",
            )
        })?;

    if !response.status().is_success() {
        let error = response
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "patchhive-launcher could not stop the first stack.".into());
        return Err(api_error(
            StatusCode::BAD_GATEWAY,
            "launcher_rejected",
            error,
        ));
    }

    response.json::<LauncherActionBody>().await.map_err(|_| {
        api_error(
            StatusCode::BAD_GATEWAY,
            "launcher_invalid_response",
            "HiveCore could not parse the launcher stop response.",
        )
    })
}

async fn launcher_rejected(
    response: reqwest::Response,
    fallback: &str,
) -> (StatusCode, Json<crate::models::ApiEnvelope<Value>>) {
    let error = response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| fallback.into());
    api_error(StatusCode::BAD_GATEWAY, "launcher_rejected", error)
}

async fn wait_for_first_stack(state: &AppState, actions: &mut Vec<String>) {
    wait_for_products(state, &DOWNSTREAM_FIRST_STACK_SLUGS, actions).await;
}

async fn wait_for_products(state: &AppState, slugs: &[&str], actions: &mut Vec<String>) {
    let overrides = db::product_overrides();
    for slug in slugs {
        let Some(definition) = product_catalog()
            .iter()
            .find(|product| product.slug == *slug)
        else {
            continue;
        };
        let api_url = pick_url(
            overrides.get(*slug).map(|item| item.api_url.as_str()),
            definition.default_api_url,
        );
        let ok = wait_for_health(state, &api_url).await;
        actions.push(if ok {
            format!("{} responded at /health.", definition.title)
        } else {
            format!(
                "{} did not become healthy before HiveCore timed out waiting.",
                definition.title
            )
        });
    }
}

async fn wait_for_health(state: &AppState, api_url: &str) -> bool {
    let health_url = format!("{}/health", api_url.trim_end_matches('/'));
    for _ in 0..20 {
        let response = state
            .client
            .get(&health_url)
            .timeout(Duration::from_secs(2))
            .send()
            .await;
        if matches!(response, Ok(ref res) if res.status().is_success()) {
            return true;
        }
        sleep(Duration::from_secs(2)).await;
    }
    false
}

async fn auto_pair_first_stack(state: &AppState, secret: &str, actions: &mut Vec<String>) {
    auto_pair_products(state, secret, &DOWNSTREAM_FIRST_STACK_SLUGS, actions).await;
}

async fn auto_pair_products(
    state: &AppState,
    secret: &str,
    slugs: &[&str],
    actions: &mut Vec<String>,
) {
    let runtimes = build_runtime_products(state).await;
    let overrides = db::product_overrides();

    for slug in slugs {
        let Some(definition) = product_catalog()
            .iter()
            .find(|product| product.slug == *slug)
        else {
            continue;
        };
        let runtime = runtimes.iter().find(|item| item.slug == *slug);
        let override_item = overrides.get(*slug);

        let Some(runtime) = runtime else {
            actions.push(format!(
                "HiveCore could not find runtime metadata for {}.",
                definition.title
            ));
            continue;
        };

        if !runtime.enabled {
            actions.push(format!(
                "Skipped {} because it is disabled in HiveCore settings.",
                definition.title
            ));
            continue;
        }

        if !matches!(runtime.status.as_str(), "online" | "degraded") {
            actions.push(format!(
                "Skipped {} because it is not reachable yet.",
                definition.title
            ));
            continue;
        }

        let auth_status = match fetch_product_auth_status(&state.client, &runtime.api_url).await {
            Ok(status) => status,
            Err(message) => {
                actions.push(format!(
                    "Skipped {} pairing: {}.",
                    definition.title, message
                ));
                continue;
            }
        };

        let needs_pair = !runtime.service_token_configured
            || runtime.legacy_api_key_configured
            || !auth_status.service_auth_enabled
            || auth_status.service_auth_legacy
            || auth_status.service_auth_expired;

        if !needs_pair {
            actions.push(format!(
                "HiveCore already has a healthy service token for {}.",
                definition.title
            ));
            continue;
        }

        if auth_status.auth_enabled && !auth_status.suite_bootstrap_enabled {
            actions.push(format!(
                "Skipped {} pairing because it requires operator auth and does not advertise suite bootstrap yet.",
                definition.title
            ));
            continue;
        }

        match provision_service_token_for_product(
            state,
            definition,
            override_item,
            "",
            "",
            Some(secret),
        )
        .await
        {
            Ok((_product, message)) => actions.push(message),
            Err((_status, body)) => {
                let message = body
                    .0
                    .error
                    .as_ref()
                    .map(|error| error.message.clone())
                    .unwrap_or_else(|| {
                        format!("HiveCore could not auto-pair {}.", definition.title)
                    });
                actions.push(message);
            }
        }
    }
}

use std::{env, time::Duration};

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::sleep;

use crate::{
    db,
    models::{ok, FirstStackSetupResponse, SetupLauncherStatus, SetupProductStatus},
    state::{product_catalog, AppState},
};

use super::overview::build_runtime_products;
use super::provision::provision_service_token_for_product;
use super::{api_error, fetch_product_auth_status, pick_url};

const FIRST_STACK_SLUGS: [&str; 4] = ["signal-hive", "trust-gate", "repo-reaper", "hive-core"];

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LauncherProductStatusBody {
    slug: String,
    title: String,
    product_dir: String,
    compose_file: String,
    compose_exists: bool,
    env_file: String,
    env_exists: bool,
    env_example_exists: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LauncherStackStatusBody {
    launcher_available: bool,
    message: String,
    repo_root: String,
    docker_compose_available: bool,
    products: Vec<LauncherProductStatusBody>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LauncherActionBody {
    ok: bool,
    actions: Vec<String>,
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
    let secret = ensure_suite_bootstrap_secret();
    let launcher_base_url = launcher_base_url();
    let products_to_start = downstream_products_to_start(&state).await;
    let mut actions = Vec::new();
    if products_to_start.is_empty() {
        actions.push(
            "All first-stack downstream products already look reachable, so HiveCore skipped launcher start and moved straight to verification."
                .into(),
        );
    } else {
        let launcher =
            start_launcher_stack(&state, &launcher_base_url, &secret, products_to_start).await?;
        actions.extend(launcher.actions);
    }

    wait_for_first_stack(&state, &mut actions).await;
    auto_pair_first_stack(&state, &secret, &mut actions).await;

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

async fn build_first_stack_response(
    state: &AppState,
    actions: Vec<String>,
) -> FirstStackSetupResponse {
    let runtimes = build_runtime_products(state).await;
    let launcher = fetch_launcher_status(state)
        .await
        .unwrap_or_else(|message| SetupLauncherStatus {
            available: false,
            message,
            repo_root: String::new(),
            docker_compose_available: false,
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
        let pairing_ready = runtime.slug != "hive-core"
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
            runtime,
            auth_status,
            auth_status_error,
            pairing_ready,
        });
    }

    FirstStackSetupResponse {
        stack_id: "first-stack".into(),
        launcher,
        suite_bootstrap_configured: configured_suite_bootstrap_secret().is_some(),
        actions,
        products,
    }
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

async fn fetch_launcher_status(state: &AppState) -> Result<SetupLauncherStatus, String> {
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

    Ok(SetupLauncherStatus {
        available: body.launcher_available,
        message: body.message,
        repo_root: body.repo_root,
        docker_compose_available: body.docker_compose_available,
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

async fn wait_for_first_stack(state: &AppState, actions: &mut Vec<String>) {
    let overrides = db::product_overrides();
    for slug in ["signal-hive", "trust-gate", "repo-reaper"] {
        let Some(definition) = product_catalog()
            .iter()
            .find(|product| product.slug == slug)
        else {
            continue;
        };
        let api_url = pick_url(
            overrides.get(slug).map(|item| item.api_url.as_str()),
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
    let runtimes = build_runtime_products(state).await;
    let overrides = db::product_overrides();

    for slug in ["signal-hive", "trust-gate", "repo-reaper"] {
        let Some(definition) = product_catalog()
            .iter()
            .find(|product| product.slug == slug)
        else {
            continue;
        };
        let runtime = runtimes.iter().find(|item| item.slug == slug);
        let override_item = overrides.get(slug);

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

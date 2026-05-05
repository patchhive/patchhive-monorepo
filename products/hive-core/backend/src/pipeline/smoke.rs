use std::time::Duration;

use axum::{extract::State, Json};
use patchhive_product_core::contract;
use reqwest::Method;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    db,
    models::{
        now_rfc3339, FirstStackSetupResponse, FirstStackSmokeRun, FirstStackSmokeStep,
        ProductRuntimeItem,
    },
    state::{product_catalog, AppState},
};

use super::{
    authorized_request, build_target_url, fetch_product_auth_status, fetch_product_capabilities,
    fetch_product_runs, parse_response_body, pick_url,
    setup::{build_first_stack_response, DOWNSTREAM_FIRST_STACK_SLUGS},
    ProductStoredAuth,
};

pub(super) async fn run_first_stack_smoke(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<FirstStackSetupResponse>> {
    let mut actions = Vec::new();
    let smoke = execute_first_stack_smoke(&state).await;
    let status = smoke.status.clone();
    let summary = smoke.summary.clone();

    match db::record_first_stack_smoke_run(&smoke) {
        Ok(()) => actions.push(format!(
            "Recorded first-stack smoke run {}: {summary}",
            smoke.id
        )),
        Err(err) => {
            tracing::warn!("failed to record first-stack smoke run: {err}");
            actions.push(format!(
                "First-stack smoke run finished as {status}, but HiveCore could not persist it: {err}"
            ));
        }
    }

    Json(crate::models::ok(
        build_first_stack_response(&state, actions).await,
    ))
}

async fn execute_first_stack_smoke(state: &AppState) -> FirstStackSmokeRun {
    let started_at = now_rfc3339();
    let mut steps = Vec::new();
    let runtimes = super::overview::build_runtime_products(state).await;
    let overrides = db::product_overrides();

    for slug in DOWNSTREAM_FIRST_STACK_SLUGS {
        let Some(runtime) = runtimes.iter().find(|item| item.slug == slug) else {
            push_step(
                &mut steps,
                slug,
                slug,
                "catalog",
                "fail",
                "HiveCore could not find runtime metadata for this first-stack product.",
                None,
                Value::Null,
            );
            continue;
        };

        smoke_runtime_checks(&mut steps, runtime);

        let Some(definition) = product_catalog()
            .iter()
            .find(|product| product.slug == slug)
        else {
            continue;
        };
        let override_item = overrides.get(slug);
        let api_url = pick_url(
            override_item.map(|item| item.api_url.as_str()),
            definition.default_api_url,
        );
        let auth = ProductStoredAuth::from_override(override_item);

        smoke_auth_checks(state, runtime, &api_url, &auth, &mut steps).await;
        smoke_capability_check(state, runtime, &api_url, &auth, &mut steps).await;
        smoke_safe_action(state, runtime, &api_url, &auth, &mut steps).await;
    }

    let status = summarize_smoke_status(&steps);
    let summary = summarize_smoke(&steps, &status);
    FirstStackSmokeRun {
        id: format!("smoke_{}", Uuid::now_v7()),
        status,
        started_at,
        finished_at: now_rfc3339(),
        summary,
        steps,
    }
}

fn smoke_runtime_checks(steps: &mut Vec<FirstStackSmokeStep>, runtime: &ProductRuntimeItem) {
    let reachable = matches!(runtime.status.as_str(), "online" | "degraded");
    push_step(
        steps,
        &runtime.slug,
        &runtime.title,
        "health",
        if reachable { "pass" } else { "fail" },
        if reachable {
            "Product API is reachable through HiveCore health polling."
        } else {
            "Product API is not reachable enough for a smoke run."
        },
        None,
        json!({
            "status": runtime.status,
            "api_url": runtime.api_url,
            "db_ok": runtime.health.db_ok,
        }),
    );

    let startup_status = if runtime.health.startup_errors > 0 {
        "fail"
    } else if runtime.health.startup_warns > 0 {
        "warn"
    } else {
        "pass"
    };
    push_step(
        steps,
        &runtime.slug,
        &runtime.title,
        "startup",
        startup_status,
        if runtime.health.startup_errors > 0 {
            "Startup checks have blocking errors."
        } else if runtime.health.startup_warns > 0 {
            "Startup checks have warnings, but no blocking errors."
        } else {
            "Startup checks have no blocking errors or warnings."
        },
        None,
        json!({
            "startup_errors": runtime.health.startup_errors,
            "startup_warns": runtime.health.startup_warns,
            "startup_infos": runtime.health.startup_infos,
        }),
    );
}

async fn smoke_auth_checks(
    state: &AppState,
    runtime: &ProductRuntimeItem,
    api_url: &str,
    auth: &ProductStoredAuth,
    steps: &mut Vec<FirstStackSmokeStep>,
) {
    if !auth.service_token_configured() {
        push_step(
            steps,
            &runtime.slug,
            &runtime.title,
            "service-token",
            "fail",
            "HiveCore does not have a saved service token for this product.",
            None,
            json!({ "auth_mode": auth.auth_mode() }),
        );
        return;
    }

    match fetch_product_auth_status(&state.client, api_url).await {
        Ok(status) => {
            let ok = status.service_auth_enabled
                && !status.service_auth_expired
                && status.service_auth_scoped
                && !status.service_auth_legacy;
            push_step(
                steps,
                &runtime.slug,
                &runtime.title,
                "service-token",
                if ok { "pass" } else { "fail" },
                if ok {
                    "Product reports an active scoped service token."
                } else {
                    "Product service-token status is not ready for scoped HiveCore dispatch."
                },
                None,
                json!({
                    "service_auth_enabled": status.service_auth_enabled,
                    "service_auth_scoped": status.service_auth_scoped,
                    "service_auth_legacy": status.service_auth_legacy,
                    "service_auth_expired": status.service_auth_expired,
                    "service_auth_scopes": status.service_auth_scopes,
                }),
            );
        }
        Err(message) => push_step(
            steps,
            &runtime.slug,
            &runtime.title,
            "service-token",
            "fail",
            format!("HiveCore could not read /auth/status: {message}"),
            None,
            Value::Null,
        ),
    }

    let (runs_ok, runs, runs_error) = fetch_product_runs(&state.client, api_url, auth).await;
    push_step(
        steps,
        &runtime.slug,
        &runtime.title,
        "service-token-runs",
        if runs_ok { "pass" } else { "fail" },
        if runs_ok {
            "Saved service token can read product-owned run history."
        } else {
            "Saved service token could not read product-owned run history."
        },
        None,
        json!({
            "run_count": runs.len(),
            "error": runs_error,
        }),
    );
}

async fn smoke_capability_check(
    state: &AppState,
    runtime: &ProductRuntimeItem,
    api_url: &str,
    auth: &ProductStoredAuth,
    steps: &mut Vec<FirstStackSmokeStep>,
) {
    match fetch_product_capabilities(&state.client, api_url, auth).await {
        Ok(capabilities) => {
            let expected_action = expected_smoke_action(&runtime.slug);
            let has_action = capabilities
                .actions
                .iter()
                .any(|action| action.id == expected_action);
            push_step(
                steps,
                &runtime.slug,
                &runtime.title,
                "capabilities",
                if has_action { "pass" } else { "fail" },
                if has_action {
                    "Product advertises the expected safe smoke action."
                } else {
                    "Product capabilities are reachable, but the expected smoke action is missing."
                },
                None,
                json!({
                    "expected_action": expected_action,
                    "actions": capabilities.actions.iter().map(|action| action.id.clone()).collect::<Vec<_>>(),
                }),
            );
        }
        Err(message) => push_step(
            steps,
            &runtime.slug,
            &runtime.title,
            "capabilities",
            "fail",
            format!("HiveCore could not read /capabilities: {message}"),
            None,
            Value::Null,
        ),
    }
}

async fn smoke_safe_action(
    state: &AppState,
    runtime: &ProductRuntimeItem,
    api_url: &str,
    auth: &ProductStoredAuth,
    steps: &mut Vec<FirstStackSmokeStep>,
) {
    if !auth.service_token_configured() {
        push_step(
            steps,
            &runtime.slug,
            &runtime.title,
            "safe-action",
            "skip",
            "Skipped safe action because HiveCore does not have a service token.",
            None,
            Value::Null,
        );
        return;
    }

    if runtime.health.startup_errors > 0 {
        push_step(
            steps,
            &runtime.slug,
            &runtime.title,
            "safe-action",
            "skip",
            "Skipped safe action because startup checks have blocking errors.",
            None,
            Value::Null,
        );
        return;
    }

    let Ok(capabilities) = fetch_product_capabilities(&state.client, api_url, auth).await else {
        push_step(
            steps,
            &runtime.slug,
            &runtime.title,
            "safe-action",
            "skip",
            "Skipped safe action because capabilities could not be loaded.",
            None,
            Value::Null,
        );
        return;
    };

    let action_id = expected_smoke_action(&runtime.slug);
    let Some(action) = capabilities
        .actions
        .iter()
        .find(|action| action.id == action_id)
    else {
        push_step(
            steps,
            &runtime.slug,
            &runtime.title,
            "safe-action",
            "skip",
            "Skipped safe action because the product did not advertise it.",
            None,
            json!({ "expected_action": action_id }),
        );
        return;
    };

    if action.destructive {
        push_step(
            steps,
            &runtime.slug,
            &runtime.title,
            "safe-action",
            "skip",
            "Skipped safe action because the product marked it destructive.",
            None,
            json!({ "action": action.id }),
        );
        return;
    }

    let payload = smoke_payload(&runtime.slug);
    match post_smoke_action(state, api_url, auth, action, payload).await {
        Ok((remote_status, evidence)) => push_step(
            steps,
            &runtime.slug,
            &runtime.title,
            "safe-action",
            "pass",
            format!(
                "HiveCore dispatched {} through the saved service token.",
                action.id
            ),
            Some(remote_status),
            evidence,
        ),
        Err((remote_status, message, evidence)) => push_step(
            steps,
            &runtime.slug,
            &runtime.title,
            "safe-action",
            "fail",
            message,
            remote_status,
            evidence,
        ),
    }
}

async fn post_smoke_action(
    state: &AppState,
    api_url: &str,
    auth: &ProductStoredAuth,
    action: &contract::ProductAction,
    payload: Value,
) -> Result<(u16, Value), (Option<u16>, String, Value)> {
    let target_url =
        build_target_url(api_url, &action.path, &Default::default()).map_err(|message| {
            (
                None,
                format!("HiveCore could not build the smoke action URL: {message}"),
                Value::Null,
            )
        })?;
    let method = Method::from_bytes(action.method.as_bytes()).map_err(|_| {
        (
            None,
            "Product advertised an invalid smoke action method.".into(),
            Value::Null,
        )
    })?;
    let mut request = authorized_request(state.client.request(method.clone(), target_url), auth)
        .timeout(Duration::from_secs(smoke_timeout_secs(action.id.as_str())));
    if method != Method::GET && method != Method::HEAD {
        request = request.json(&payload);
    }

    let response = request.send().await.map_err(|err| {
        (
            None,
            format!("HiveCore could not dispatch the smoke action: {err}"),
            json!({ "error": err.to_string() }),
        )
    })?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let evidence = smoke_response_evidence(&text);
    if status.is_success() {
        Ok((status.as_u16(), evidence))
    } else {
        Err((
            Some(status.as_u16()),
            format!("Product returned HTTP {status} for the smoke action."),
            evidence,
        ))
    }
}

fn expected_smoke_action(slug: &str) -> &'static str {
    match slug {
        "signal-hive" => "scan",
        "trust-gate" => "review_diff",
        "repo-reaper" => "dry_run",
        _ => "unknown",
    }
}

fn smoke_payload(slug: &str) -> Value {
    match slug {
        "signal-hive" => json!({
            "search_query": "",
            "topics": ["maintenance"],
            "languages": ["rust"],
            "min_stars": 0,
            "max_repos": 1,
            "issues_per_repo": 1,
            "stale_days": 1
        }),
        "trust-gate" => json!({
            "repo": "patchhive/smoke-fixture",
            "ai_source": "hivecore-smoke",
            "diff": "diff --git a/src/lib.rs b/src/lib.rs\nindex 1111111..2222222 100644\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,3 +1,4 @@\n pub fn add(left: i32, right: i32) -> i32 {\n+    // HiveCore smoke check: harmless comment-only diff.\n     left + right\n }\n"
        }),
        "repo-reaper" => json!({
            "language": "rust",
            "min_stars": 0,
            "max_repos": 1,
            "max_issues": 1,
            "labels": ["bug"],
            "concurrency": 1,
            "search_query": "repo:patchhive/patchhive2 is:issue is:open",
            "cost_budget_usd": 0.0,
            "retry_count": 0
        }),
        _ => json!({}),
    }
}

fn smoke_timeout_secs(action_id: &str) -> u64 {
    match action_id {
        "scan" | "dry_run" => 45,
        _ => 15,
    }
}

fn smoke_response_evidence(text: &str) -> Value {
    let parsed = parse_response_body(text);
    if parsed.get("raw").is_some() {
        let raw = parsed
            .get("raw")
            .and_then(Value::as_str)
            .unwrap_or_default();
        json!({ "raw": truncate(raw, 1600) })
    } else {
        parsed
    }
}

fn push_step(
    steps: &mut Vec<FirstStackSmokeStep>,
    slug: impl Into<String>,
    title: impl Into<String>,
    check: impl Into<String>,
    status: impl Into<String>,
    message: impl Into<String>,
    remote_status: Option<u16>,
    evidence: Value,
) {
    steps.push(FirstStackSmokeStep {
        slug: slug.into(),
        title: title.into(),
        check: check.into(),
        status: status.into(),
        message: message.into(),
        remote_status,
        evidence,
    });
}

fn summarize_smoke_status(steps: &[FirstStackSmokeStep]) -> String {
    if steps.iter().any(|step| step.status == "fail") {
        "blocked".into()
    } else if steps
        .iter()
        .any(|step| matches!(step.status.as_str(), "warn" | "skip"))
    {
        "attention".into()
    } else {
        "ready".into()
    }
}

fn summarize_smoke(steps: &[FirstStackSmokeStep], status: &str) -> String {
    let pass = steps.iter().filter(|step| step.status == "pass").count();
    let warn = steps.iter().filter(|step| step.status == "warn").count();
    let fail = steps.iter().filter(|step| step.status == "fail").count();
    let skip = steps.iter().filter(|step| step.status == "skip").count();
    match status {
        "ready" => format!("First stack is suite-ready: {pass} checks passed."),
        "attention" => {
            format!("First stack needs attention: {pass} passed, {warn} warned, {skip} skipped.")
        }
        _ => format!(
            "First stack is blocked: {pass} passed, {warn} warned, {fail} failed, {skip} skipped."
        ),
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.into()
    } else {
        let mut truncated = value.chars().take(max_chars).collect::<String>();
        truncated.push_str("...");
        truncated
    }
}

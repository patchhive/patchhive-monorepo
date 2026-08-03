use std::time::Duration;

use axum::{extract::State, Json};
use patchhive_product_core::contract;
use patchhive_product_core::smoke_manifest::{SmokeActionManifest, SmokeTier};
use patchhive_product_core::startup::{StartupCheck, StartupCheckLevel};
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
    authorized_get, authorized_request, build_target_url, fetch_product_auth_status,
    fetch_product_capabilities, fetch_product_runs, parse_response_body, resolve_api_url,
    setup::{
        build_first_stack_response, prepare_first_stack_for_verification,
        prepare_products_for_service_token_verification,
    },
    ProductStoredAuth, StartupChecksBody,
};

macro_rules! push_step {
    (
        $steps:expr,
        $slug:expr,
        $title:expr,
        $check:expr,
        $status:expr,
        $message:expr,
        $remote_status:expr,
        $evidence:expr $(,)?
    ) => {
        $steps.push(FirstStackSmokeStep {
            slug: $slug.into(),
            title: $title.into(),
            check: $check.into(),
            status: $status.into(),
            message: $message.into(),
            remote_status: $remote_status,
            evidence: $evidence,
        })
    };
}

pub(super) async fn run_first_stack_smoke(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<FirstStackSetupResponse>> {
    run_smoke_tier_response(&state, SmokeTier::FirstStack).await
}

pub(super) async fn run_setup_smoke_tier(
    State(state): State<AppState>,
    tier_slug: String,
) -> Json<crate::models::ApiEnvelope<FirstStackSetupResponse>> {
    let Some(tier) = SmokeTier::from_slug(&tier_slug) else {
        return Json(crate::models::ok(
            build_first_stack_response(
                &state,
                vec![format!(
                    "Unknown smoke tier {tier_slug}; available tiers are first-stack, read-only-fleet, write-dry-run, and release-gate."
                )],
            )
            .await,
        ));
    };

    run_smoke_tier_response(&state, tier).await
}

async fn run_smoke_tier_response(
    state: &AppState,
    tier: SmokeTier,
) -> Json<crate::models::ApiEnvelope<FirstStackSetupResponse>> {
    let mut actions = vec![format!(
        "HiveCore started {}: {}.",
        smoke_tier_label(tier),
        smoke_tier_description(tier)
    )];
    let mut preflight_steps = Vec::new();

    if matches!(tier, SmokeTier::FirstStack) {
        match prepare_first_stack_for_verification(state, &mut actions).await {
            Ok(()) => push_step!(
                &mut preflight_steps,
                "first-stack",
                "First Stack",
                "preflight",
                "pass",
                "HiveCore completed launch, health wait, and pairing preflight before running smoke actions.",
                None,
                json!({ "actions": actions }),
            ),
            Err((_status, body)) => {
                let message = body
                    .0
                    .error
                    .as_ref()
                    .map(|error| error.message.clone())
                    .unwrap_or_else(|| {
                        "HiveCore could not complete first-stack smoke preflight.".into()
                    });
                actions.push(format!("First-stack smoke preflight failed: {message}"));
                push_step!(
                    &mut preflight_steps,
                    "first-stack",
                    "First Stack",
                    "preflight",
                    "fail",
                    message,
                    None,
                    json!({ "actions": actions }),
                );
            }
        }
    } else if matches!(
        tier,
        SmokeTier::ReadOnlyFleet | SmokeTier::WriteDryRun | SmokeTier::ReleaseGate
    ) {
        let tier_slugs = smoke_tier_slugs(tier);
        prepare_products_for_service_token_verification(state, &tier_slugs, &mut actions).await;
        push_step!(
            &mut preflight_steps,
            tier.slug(),
            smoke_tier_label(tier),
            "pairing-preflight",
            "pass",
            format!(
                "HiveCore checked running products for service-token pairing before {}.",
                smoke_tier_label(tier)
            ),
            None,
            json!({ "tier": tier.slug(), "actions": actions }),
        );
    }

    let smoke = execute_smoke_tier(state, tier, preflight_steps).await;
    let status = smoke.status.clone();
    let summary = smoke.summary.clone();

    match db::record_first_stack_smoke_run(&smoke) {
        Ok(()) => actions.push(format!(
            "Recorded {} {}: {summary}",
            smoke_tier_label(tier),
            smoke.id
        )),
        Err(err) => {
            tracing::warn!("failed to record smoke run: {err}");
            actions.push(format!(
                "{} finished as {status}, but HiveCore could not persist it: {err}",
                smoke_tier_label(tier)
            ));
        }
    }

    Json(crate::models::ok(
        build_first_stack_response(state, actions).await,
    ))
}

fn smoke_tier_description(tier: SmokeTier) -> &'static str {
    match tier {
        SmokeTier::FirstStack => {
            "launch, health wait, service-token pairing, then safe product actions for SignalHive, TrustGate, and RepoReaper"
        }
        SmokeTier::ReadOnlyFleet => {
            "reachability, service-token, run-history, and capability inventory checks for every non-write product; no product actions are dispatched"
        }
        SmokeTier::WriteDryRun => {
            "RepoReaper only, using the saved service token and dry-run action so no PRs are opened"
        }
        SmokeTier::ReleaseGate => {
            "ReleaseSentry only, using the saved service token to dispatch a read-only release readiness check"
        }
    }
}

fn smoke_tier_label(tier: SmokeTier) -> &'static str {
    match tier {
        SmokeTier::FirstStack => "First-stack smoke",
        SmokeTier::ReadOnlyFleet => "Read-only fleet smoke",
        SmokeTier::WriteDryRun => "RepoReaper dry-run smoke",
        SmokeTier::ReleaseGate => "ReleaseSentry release-gate smoke",
    }
}

async fn execute_smoke_tier(
    state: &AppState,
    tier: SmokeTier,
    mut steps: Vec<FirstStackSmokeStep>,
) -> FirstStackSmokeRun {
    let started_at = now_rfc3339();
    let runtimes = super::overview::build_runtime_products(state).await;
    let overrides = db::product_overrides();

    smoke_tier_coverage_check(tier, &runtimes, &mut steps);

    for slug in smoke_tier_slugs(tier) {
        let Some(runtime) = runtimes.iter().find(|item| item.slug == slug) else {
            push_step!(
                &mut steps,
                slug,
                slug,
                "catalog",
                "fail",
                "HiveCore could not find runtime metadata for this smoke tier product.",
                None,
                Value::Null,
            );
            continue;
        };

        let Some(definition) = product_catalog()
            .iter()
            .find(|product| product.slug == slug)
        else {
            continue;
        };
        let override_item = overrides.get(slug);
        let api_url = resolve_api_url(override_item.map(|item| item.api_url.as_str()), definition);
        let auth = ProductStoredAuth::from_override(override_item);

        match tier {
            SmokeTier::FirstStack => {
                smoke_runtime_checks(state, &mut steps, runtime, &api_url, &auth).await;
                smoke_auth_checks(state, runtime, &api_url, &auth, &mut steps).await;
                smoke_capability_check(state, runtime, &api_url, &auth, &mut steps).await;
                smoke_safe_action(state, runtime, &api_url, &auth, &mut steps).await;
            }
            SmokeTier::ReadOnlyFleet => {
                smoke_runtime_checks(state, &mut steps, runtime, &api_url, &auth).await;
                smoke_optional_auth_check(state, runtime, &api_url, &auth, &mut steps).await;
                smoke_capability_inventory_check(state, runtime, &api_url, &auth, &mut steps).await;
            }
            SmokeTier::WriteDryRun => {
                smoke_runtime_checks(state, &mut steps, runtime, &api_url, &auth).await;
                smoke_auth_checks(state, runtime, &api_url, &auth, &mut steps).await;
                smoke_capability_check(state, runtime, &api_url, &auth, &mut steps).await;
                smoke_safe_action(state, runtime, &api_url, &auth, &mut steps).await;
            }
            SmokeTier::ReleaseGate => {
                smoke_runtime_checks(state, &mut steps, runtime, &api_url, &auth).await;
                smoke_auth_checks(state, runtime, &api_url, &auth, &mut steps).await;
                smoke_capability_check(state, runtime, &api_url, &auth, &mut steps).await;
                smoke_safe_action(state, runtime, &api_url, &auth, &mut steps).await;
            }
        }
    }

    let status = summarize_smoke_status(&steps);
    let summary = summarize_smoke(tier, &steps, &status);
    FirstStackSmokeRun {
        id: format!("smoke_{}", Uuid::now_v7()),
        tier: tier.slug().into(),
        status,
        started_at,
        finished_at: now_rfc3339(),
        summary,
        steps,
    }
}

fn smoke_tier_slugs(tier: SmokeTier) -> Vec<&'static str> {
    product_catalog()
        .iter()
        .filter(|product| product.smoke.participates_in(tier))
        .map(|product| product.slug.as_str())
        .collect()
}

fn smoke_tier_coverage_check(
    tier: SmokeTier,
    runtimes: &[ProductRuntimeItem],
    steps: &mut Vec<FirstStackSmokeStep>,
) {
    if !matches!(tier, SmokeTier::ReadOnlyFleet) {
        return;
    }

    let expected = smoke_tier_slugs(tier);
    let missing = expected
        .iter()
        .filter(|slug| !runtimes.iter().any(|item| item.slug == **slug))
        .copied()
        .collect::<Vec<_>>();
    let offline = expected
        .iter()
        .filter_map(|slug| {
            let runtime = runtimes.iter().find(|item| item.slug == *slug)?;
            if matches!(runtime.status.as_str(), "online" | "degraded") {
                None
            } else {
                Some(runtime.slug.as_str())
            }
        })
        .collect::<Vec<_>>();
    let reachable = expected.len() - missing.len() - offline.len();
    let ok = missing.is_empty() && offline.is_empty();

    push_step!(
        steps,
        tier.slug(),
        smoke_tier_label(tier),
        "fleet-coverage",
        if ok { "pass" } else { "fail" },
        if ok {
            format!(
                "HiveCore sees all {reachable}/{} non-write fleet products reachable before deeper checks.",
                expected.len()
            )
        } else {
            format!(
                "HiveCore sees {reachable}/{} non-write fleet products reachable; missing or offline products block fleet smoke.",
                expected.len()
            )
        },
        None,
        json!({
            "expected_products": expected,
            "reachable": reachable,
            "missing": missing,
            "offline": offline,
        }),
    );
}

async fn smoke_runtime_checks(
    state: &AppState,
    steps: &mut Vec<FirstStackSmokeStep>,
    runtime: &ProductRuntimeItem,
    api_url: &str,
    auth: &ProductStoredAuth,
) {
    let reachable = matches!(runtime.status.as_str(), "online" | "degraded");
    push_step!(
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
            "health_endpoint": runtime.health.health_endpoint,
        }),
    );

    let startup = runtime.health.startup_checks.value();
    let mut startup_status = match startup {
        Some(summary) if summary.errors > 0 => "fail",
        Some(summary) if summary.warnings > 0 => "warn",
        Some(_) => "pass",
        None => "fail",
    };
    let mut startup_message = match startup {
        Some(summary) if summary.errors > 0 => "Startup checks have blocking errors.".into(),
        Some(summary) if summary.warnings > 0 => {
            "Startup checks have warnings, but no blocking errors.".into()
        }
        Some(_) => "Startup checks have no blocking errors or warnings.".into(),
        None => runtime
            .health
            .startup_checks
            .reason()
            .unwrap_or("Startup checks were not observed.")
            .into(),
    };
    let mut evidence = json!(runtime.health.startup_checks);

    if startup.is_some_and(|summary| summary.errors == 0 && summary.warnings > 0) {
        let warning_policy = product_catalog()
            .iter()
            .find(|product| product.slug == runtime.slug)
            .map(|product| &product.smoke);
        match fetch_startup_warning_checks(state, api_url, auth).await {
            Ok(warnings)
                if !warnings.is_empty()
                    && warning_policy.is_some()
                    && warnings.iter().all(|warning| {
                        warning_policy.is_some_and(|policy| {
                            policy.acknowledges(warning.code.as_deref(), warning.status.as_deref())
                        })
                    }) =>
            {
                startup_status = "pass";
                startup_message = format!(
                    "Startup checks include only manifest-acknowledged warnings: {}",
                    startup_warning_summary(&warnings)
                );
                if let Some(map) = evidence.as_object_mut() {
                    map.insert("warnings".into(), json!(warnings));
                    map.insert(
                        "acknowledged_warnings".into(),
                        json!(warnings
                            .iter()
                            .map(startup_check_identity)
                            .collect::<Vec<_>>()),
                    );
                    map.insert("warning_policy".into(), json!("product_manifest"));
                }
            }
            Ok(warnings) => {
                if let Some(map) = evidence.as_object_mut() {
                    map.insert("warnings".into(), json!(warnings));
                }
            }
            Err(message) => {
                if let Some(map) = evidence.as_object_mut() {
                    map.insert("warning_details_error".into(), json!(message));
                }
            }
        }
    }

    push_step!(
        steps,
        &runtime.slug,
        &runtime.title,
        "startup",
        startup_status,
        startup_message,
        None,
        evidence,
    );
}

async fn fetch_startup_warning_checks(
    state: &AppState,
    api_url: &str,
    auth: &ProductStoredAuth,
) -> Result<Vec<StartupCheck>, String> {
    let checks_url = format!("{}/startup/checks", api_url.trim_end_matches('/'));
    let response = authorized_get(&state.client, &checks_url, auth)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|_| "Could not reach /startup/checks for warning details.".to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "/startup/checks returned HTTP {} while loading warning details.",
            response.status()
        ));
    }

    let body = response
        .json::<StartupChecksBody>()
        .await
        .map_err(|err| format!("Could not parse /startup/checks warning details: {err}"))?;

    Ok(body
        .checks
        .into_iter()
        .filter(|check| check.level == StartupCheckLevel::Warn)
        .collect())
}

fn startup_check_identity(check: &StartupCheck) -> Value {
    json!({
        "code": check.code,
        "status": check.status,
    })
}

fn startup_warning_summary(warnings: &[StartupCheck]) -> String {
    truncate(
        &warnings
            .iter()
            .map(|warning| warning.msg.trim().trim_end_matches('.'))
            .collect::<Vec<_>>()
            .join("; "),
        320,
    )
}

async fn smoke_auth_checks(
    state: &AppState,
    runtime: &ProductRuntimeItem,
    api_url: &str,
    auth: &ProductStoredAuth,
    steps: &mut Vec<FirstStackSmokeStep>,
) {
    if !auth.service_token_configured() {
        push_step!(
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
            push_step!(
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
        Err(message) => push_step!(
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
    push_step!(
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
    let Some(smoke_action) = smoke_action_manifest(&runtime.slug) else {
        push_step!(
            steps,
            &runtime.slug,
            &runtime.title,
            "capabilities",
            "fail",
            "The canonical product manifest does not declare a smoke action for this tier.",
            None,
            Value::Null,
        );
        return;
    };

    match fetch_product_capabilities(&state.client, api_url, auth).await {
        Ok(capabilities) => {
            let has_action = capabilities
                .actions
                .iter()
                .any(|action| action.id == smoke_action.id);
            push_step!(
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
                    "expected_action": smoke_action.id,
                    "actions": capabilities.actions.iter().map(|action| action.id.clone()).collect::<Vec<_>>(),
                }),
            );
        }
        Err(message) => push_step!(
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

async fn smoke_optional_auth_check(
    state: &AppState,
    runtime: &ProductRuntimeItem,
    api_url: &str,
    auth: &ProductStoredAuth,
    steps: &mut Vec<FirstStackSmokeStep>,
) {
    if !auth.service_token_configured() {
        push_step!(
            steps,
            &runtime.slug,
            &runtime.title,
            "service-token",
            "warn",
            "HiveCore does not have a saved service token yet; read-only smoke will continue with public control-plane checks.",
            None,
            json!({ "auth_mode": auth.auth_mode() }),
        );
        return;
    }

    smoke_auth_checks(state, runtime, api_url, auth, steps).await;
}

async fn smoke_capability_inventory_check(
    state: &AppState,
    runtime: &ProductRuntimeItem,
    api_url: &str,
    auth: &ProductStoredAuth,
    steps: &mut Vec<FirstStackSmokeStep>,
) {
    match fetch_product_capabilities(&state.client, api_url, auth).await {
        Ok(capabilities) => push_step!(
            steps,
            &runtime.slug,
            &runtime.title,
            "capabilities",
            "pass",
            "Product capabilities are reachable for read-only fleet smoke.",
            None,
            json!({
                "action_count": capabilities.actions.len(),
                "actions": capabilities.actions.iter().map(|action| action.id.clone()).collect::<Vec<_>>(),
                "hivecore": capabilities.hivecore,
            }),
        ),
        Err(message) => push_step!(
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
        push_step!(
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

    let Some(startup) = runtime.health.startup_checks.value() else {
        push_step!(
            steps,
            &runtime.slug,
            &runtime.title,
            "safe-action",
            "skip",
            "Skipped safe action because startup checks were not observed.",
            None,
            json!(runtime.health.startup_checks),
        );
        return;
    };

    if startup.errors > 0 {
        push_step!(
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
        push_step!(
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

    let Some(smoke_action) = smoke_action_manifest(&runtime.slug) else {
        push_step!(
            steps,
            &runtime.slug,
            &runtime.title,
            "safe-action",
            "fail",
            "The canonical product manifest does not declare a smoke action for this tier.",
            None,
            Value::Null,
        );
        return;
    };
    let Some(action) = capabilities
        .actions
        .iter()
        .find(|action| action.id == smoke_action.id)
    else {
        push_step!(
            steps,
            &runtime.slug,
            &runtime.title,
            "safe-action",
            "skip",
            "Skipped safe action because the product did not advertise it.",
            None,
            json!({ "expected_action": smoke_action.id }),
        );
        return;
    };

    if action.destructive {
        push_step!(
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

    match post_smoke_action(
        state,
        api_url,
        auth,
        action,
        smoke_action.payload.clone(),
        smoke_action.timeout_seconds,
    )
    .await
    {
        Ok((remote_status, evidence)) => push_step!(
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
        Err((remote_status, message, evidence)) => push_step!(
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
    timeout_seconds: u64,
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
        .timeout(Duration::from_secs(timeout_seconds));
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

fn smoke_action_manifest(slug: &str) -> Option<&'static SmokeActionManifest> {
    product_catalog()
        .iter()
        .find(|product| product.slug == slug)
        .and_then(|product| product.smoke.action.as_ref())
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

fn summarize_smoke(tier: SmokeTier, steps: &[FirstStackSmokeStep], status: &str) -> String {
    let pass = steps.iter().filter(|step| step.status == "pass").count();
    let warn = steps.iter().filter(|step| step.status == "warn").count();
    let fail = steps.iter().filter(|step| step.status == "fail").count();
    let skip = steps.iter().filter(|step| step.status == "skip").count();
    let acknowledged_warns = steps
        .iter()
        .filter_map(|step| step.evidence.get("acknowledged_warnings"))
        .filter_map(Value::as_array)
        .map(Vec::len)
        .sum::<usize>();
    match status {
        "ready" if acknowledged_warns > 0 => format!(
            "{} is suite-ready: {pass} checks passed, {acknowledged_warns} local warning{} acknowledged.",
            smoke_tier_label(tier),
            if acknowledged_warns == 1 { "" } else { "s" }
        ),
        "ready" => format!(
            "{} is suite-ready: {pass} checks passed.",
            smoke_tier_label(tier)
        ),
        "attention" => {
            format!(
                "{} needs attention: {pass} passed, {warn} warned, {skip} skipped.",
                smoke_tier_label(tier)
            )
        }
        _ => format!(
            "{} is blocked: {pass} passed, {warn} warned, {fail} failed, {skip} skipped.",
            smoke_tier_label(tier)
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

use std::time::Duration;

use axum::{extract::State, Json};
use patchhive_product_core::contract;
use patchhive_product_core::startup::StartupCheckLevel;
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
    fetch_product_capabilities, fetch_product_runs, parse_response_body, pick_url,
    setup::{
        build_first_stack_response, prepare_first_stack_for_verification,
        prepare_products_for_service_token_verification, DOWNSTREAM_FIRST_STACK_SLUGS,
    },
    ProductStoredAuth, StartupChecksBody,
};

const READ_ONLY_FLEET_SLUGS: [&str; 9] = [
    "signal-hive",
    "repo-memory",
    "trust-gate",
    "review-bee",
    "merge-keeper",
    "flake-sting",
    "dep-triage",
    "vuln-triage",
    "refactor-scout",
];
const WRITE_DRY_RUN_SLUGS: [&str; 1] = ["repo-reaper"];

#[derive(Clone, Copy)]
enum SmokeTier {
    FirstStack,
    ReadOnlyFleet,
    WriteDryRun,
}

impl SmokeTier {
    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "first-stack" => Some(Self::FirstStack),
            "read-only-fleet" => Some(Self::ReadOnlyFleet),
            "write-dry-run" => Some(Self::WriteDryRun),
            _ => None,
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::FirstStack => "first-stack",
            Self::ReadOnlyFleet => "read-only-fleet",
            Self::WriteDryRun => "write-dry-run",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::FirstStack => "First-stack smoke",
            Self::ReadOnlyFleet => "Read-only fleet smoke",
            Self::WriteDryRun => "RepoReaper dry-run smoke",
        }
    }
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
                    "Unknown smoke tier {tier_slug}; available tiers are first-stack, read-only-fleet, and write-dry-run."
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
        tier.label(),
        smoke_tier_description(tier)
    )
    .into()];
    let mut preflight_steps = Vec::new();

    if matches!(tier, SmokeTier::FirstStack) {
        match prepare_first_stack_for_verification(state, &mut actions).await {
            Ok(()) => push_step(
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
                push_step(
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
    } else if matches!(tier, SmokeTier::ReadOnlyFleet | SmokeTier::WriteDryRun) {
        prepare_products_for_service_token_verification(
            state,
            smoke_tier_slugs(tier),
            &mut actions,
        )
        .await;
        push_step(
            &mut preflight_steps,
            tier.slug(),
            tier.label(),
            "pairing-preflight",
            "pass",
            format!(
                "HiveCore checked running products for service-token pairing before {}.",
                tier.label()
            ),
            None,
            json!({ "tier": tier.slug(), "actions": actions }),
        );
    }

    let smoke = execute_smoke_tier(state, tier, preflight_steps).await;
    let status = smoke.status.clone();
    let summary = smoke.summary.clone();

    match db::record_first_stack_smoke_run(&smoke) {
        Ok(()) => actions.push(format!("Recorded {} {}: {summary}", tier.label(), smoke.id)),
        Err(err) => {
            tracing::warn!("failed to record smoke run: {err}");
            actions.push(format!(
                "{} finished as {status}, but HiveCore could not persist it: {err}",
                tier.label()
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

    for &slug in smoke_tier_slugs(tier) {
        let Some(runtime) = runtimes.iter().find(|item| item.slug == slug) else {
            push_step(
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
        let api_url = pick_url(
            override_item.map(|item| item.api_url.as_str()),
            definition.default_api_url,
        );
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

fn smoke_tier_slugs(tier: SmokeTier) -> &'static [&'static str] {
    match tier {
        SmokeTier::FirstStack => &DOWNSTREAM_FIRST_STACK_SLUGS,
        SmokeTier::ReadOnlyFleet => &READ_ONLY_FLEET_SLUGS,
        SmokeTier::WriteDryRun => &WRITE_DRY_RUN_SLUGS,
    }
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

    push_step(
        steps,
        tier.slug(),
        tier.label(),
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

    let mut startup_status = if runtime.health.startup_errors > 0 {
        "fail"
    } else if runtime.health.startup_warns > 0 {
        "warn"
    } else {
        "pass"
    };
    let mut startup_message = if runtime.health.startup_errors > 0 {
        "Startup checks have blocking errors."
    } else if runtime.health.startup_warns > 0 {
        "Startup checks have warnings, but no blocking errors."
    } else {
        "Startup checks have no blocking errors or warnings."
    }
    .to_string();
    let mut evidence = json!({
        "startup_errors": runtime.health.startup_errors,
        "startup_warns": runtime.health.startup_warns,
        "startup_infos": runtime.health.startup_infos,
    });

    if runtime.health.startup_errors == 0 && runtime.health.startup_warns > 0 {
        match fetch_startup_warning_messages(state, api_url, auth).await {
            Ok(warnings)
                if !warnings.is_empty()
                    && warnings
                        .iter()
                        .all(|warning| acknowledged_startup_warning(&runtime.slug, warning)) =>
            {
                startup_status = "pass";
                startup_message = format!(
                    "Startup checks include only acknowledged local-first-stack warnings: {}",
                    startup_warning_summary(&warnings)
                );
                if let Some(map) = evidence.as_object_mut() {
                    map.insert("warnings".into(), json!(warnings));
                    map.insert("acknowledged_warnings".into(), json!(warnings));
                    map.insert("warning_policy".into(), json!("local_first_stack"));
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

    push_step(
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

async fn fetch_startup_warning_messages(
    state: &AppState,
    api_url: &str,
    auth: &ProductStoredAuth,
) -> Result<Vec<String>, String> {
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
        .map(|check| check.msg)
        .collect())
}

fn acknowledged_startup_warning(slug: &str, warning: &str) -> bool {
    let normalized = warning.to_ascii_lowercase();
    normalized.contains("api-key auth is not enabled yet")
        || normalized.contains("public_url is not configured")
        || normalized.contains("public url is not configured")
        || normalized.contains("github webhook secret is not configured")
        || normalized.contains("webhook delivery until it is set")
        || normalized.contains("public-repo scans may still work")
        || normalized.contains("public dependency pr scans may still work")
        || normalized.contains("public reads may still work")
        || normalized.contains("github-backed ingestion is disabled")
        || matches!(
            slug,
            "trust-gate"
                if normalized.contains("trust_github_webhook_secret is not configured")
                    || normalized.contains("bot_github_token is missing")
        )
        || matches!(
            slug,
            "refactor-scout"
                if normalized.contains("refactor_scout_allowed_roots is not set")
                    || normalized.contains("defaults to the process working directory")
        )
}

fn startup_warning_summary(warnings: &[String]) -> String {
    truncate(
        &warnings
            .iter()
            .map(|warning| warning.trim().trim_end_matches('.'))
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

async fn smoke_optional_auth_check(
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
        Ok(capabilities) => push_step(
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
        "signal-hive" => "smoke_check",
        "trust-gate" => "review_diff",
        "repo-reaper" => "dry_run",
        _ => "unknown",
    }
}

fn smoke_payload(slug: &str) -> Value {
    match slug {
        "signal-hive" => json!({}),
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
            tier.label(),
            if acknowledged_warns == 1 { "" } else { "s" }
        ),
        "ready" => format!("{} is suite-ready: {pass} checks passed.", tier.label()),
        "attention" => {
            format!(
                "{} needs attention: {pass} passed, {warn} warned, {skip} skipped.",
                tier.label()
            )
        }
        _ => format!(
            "{} is blocked: {pass} passed, {warn} warned, {fail} failed, {skip} skipped.",
            tier.label()
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

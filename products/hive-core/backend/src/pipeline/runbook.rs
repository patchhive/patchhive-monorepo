//! Product runbooks: a recorded diagnostic pass over one product.
//!
//! The deck's runbooks were theatre. Steps like "Restart ingest worker pool across 4
//! replicas", "Force rotate leaking token" and "Failover NVD feed to mirror" were a
//! hardcoded array; "executing" one was `sleep(650 + random * 400)` followed by
//! marking every step done, and the drawer then wrote an audit entry of
//! `kind: "destructive"` saying it had happened.
//!
//! That is worse than having no runbooks. A fabricated metric misleads; a fabricated
//! audit trail of destructive operations corrupts the one record you would consult to
//! find out what was actually done to a system.
//!
//! So the steps here are only things HiveCore genuinely does, and each reports what it
//! actually saw. There is no step for restarting a worker or failing over a feed:
//! those are host operations belonging to `patchhive-launcher`, and inventing a
//! control-plane button for them is how a deck starts writing cheques the runtime
//! cannot cash.
//!
//! Every check is read-only. A runbook diagnoses; it does not act. Acting on a product
//! is a dispatch or a suite run, both of which carry approval, scope and credential
//! guards — and a diagnostic panel must not become a side door around them.

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    db,
    models::{now_rfc3339, ok, RunbookRun, RunbookStep},
    state::{product_catalog, AppState},
};

use super::{
    api_error, fetch_product_auth_status, overview::fetch_product_health, resolve_api_url,
    ProductStoredAuth,
};

type ApiResult<T> = Result<
    Json<crate::models::ApiEnvelope<T>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
>;

fn step(
    id: &str,
    label: &str,
    status: &str,
    message: impl Into<String>,
    evidence: Value,
) -> RunbookStep {
    RunbookStep {
        id: id.into(),
        label: label.into(),
        status: status.into(),
        message: message.into(),
        remote_status: None,
        evidence,
    }
}

pub(super) async fn run_product_runbook(
    State(state): State<AppState>,
    slug: String,
) -> ApiResult<RunbookRun> {
    let definition = product_catalog()
        .iter()
        .find(|product| product.slug == slug)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "unknown_product", "Unknown product."))?;

    let started_at = now_rfc3339();
    let overrides = db::product_overrides();
    let override_item = overrides.get(definition.slug);
    let auth = ProductStoredAuth::from_override(override_item);
    let api_url = resolve_api_url(override_item.map(|item| item.api_url.as_str()), definition);

    let mut steps = Vec::new();

    // 1. Reachability. Everything below depends on this, so a failure here is
    //    reported once rather than repeated as five downstream mysteries.
    let probe = fetch_product_health(&state.client, &api_url, &auth).await;
    let reachable = probe.health.status != "offline";
    steps.push(step(
        "reachable",
        "Product answers /health",
        if reachable { "ok" } else { "fail" },
        if reachable {
            format!("Reported status `{}`.", probe.health.status)
        } else {
            let detail = if probe.health.error.is_empty() {
                "No response.".to_string()
            } else {
                probe.health.error.clone()
            };
            format!("Unreachable at {api_url}. {detail}")
        },
        json!({ "api_url": api_url, "status": probe.health.status, "error": probe.health.error }),
    ));

    if !reachable {
        // Stop rather than emit four more failures that all mean "it is not running".
        // A runbook that reports five problems where there is one sends an operator
        // looking in four wrong places.
        steps.push(step(
            "halted",
            "Remaining checks",
            "skipped",
            "Skipped: the product is not reachable, so nothing below can be observed.",
            json!({}),
        ));
        return Ok(Json(ok(finish(definition, started_at, steps))));
    }

    // 2. Startup diagnostics the product reports about itself. These are product-domain
    //    findings, not toolchain noise — a warning here is evidence, not something to
    //    tidy away.
    let errors = probe.health.startup_errors;
    let warnings = probe.health.startup_warns;
    steps.push(step(
        "startup",
        "Startup checks",
        if errors > 0 {
            "fail"
        } else if warnings > 0 {
            "warn"
        } else {
            "ok"
        },
        if errors > 0 {
            format!("{errors} error(s) and {warnings} warning(s) reported at startup.")
        } else if warnings > 0 {
            format!("{warnings} warning(s) reported at startup.")
        } else {
            "No startup errors or warnings.".to_string()
        },
        json!({ "errors": errors, "warnings": warnings }),
    ));

    // 3. Contract conformance: what the product advertises versus what its manifest
    //    declares. This is the check that caught ReviewBee and TrustGate holding write
    //    scopes on actions declaring nothing.
    let drift = probe
        .contract_checks
        .iter()
        .filter(|check| check.status != "ok")
        .count();
    steps.push(step(
        "contract",
        "Advertised contract matches the manifest",
        if drift == 0 { "ok" } else { "warn" },
        if drift == 0 {
            format!("{} advertised action(s), no drift.", probe.actions.len())
        } else {
            format!("{drift} contract check(s) not ok.")
        },
        json!({
            "advertised_actions": probe.actions.len(),
            "checks": probe.contract_checks,
        }),
    ));

    // 4. Service-token posture. HiveCore cannot dispatch to a product it has no scoped
    //    token for, so an operator debugging "why did my suite run fail" should see it
    //    here rather than infer it from a 401.
    let auth_status = fetch_product_auth_status(&state.client, &api_url).await;
    let (token_status, token_message) = match &auth_status {
        Ok(status) if !status.service_auth_supported => {
            ("ok", "Product does not use service-token auth.".to_string())
        }
        Ok(status) if !status.service_auth_enabled => (
            "warn",
            "No service token provisioned. HiveCore cannot dispatch protected actions.".to_string(),
        ),
        Ok(status) if status.service_auth_expired => {
            ("fail", "Service token has expired.".to_string())
        }
        Ok(status) if !status.service_auth_scoped => (
            "warn",
            "Legacy token: reads runs but cannot dispatch actions.".to_string(),
        ),
        Ok(_) => ("ok", "Scoped service token active.".to_string()),
        Err(error) => (
            "warn",
            format!("Product did not report auth status: {error}"),
        ),
    };
    steps.push(step(
        "service_token",
        "HiveCore can authenticate to this product",
        token_status,
        token_message,
        json!({ "scopes": auth_status.as_ref().ok().map(|s| s.service_auth_scopes.clone()) }),
    ));

    // 5. Recent outcomes. A product can be reachable, conformant and authenticated and
    //    still be failing every run, which is the state most worth surfacing.
    let recent = &probe.recent_runs;
    let failed = recent.iter().filter(|run| run.status == "failed").count();
    steps.push(step(
        "recent_runs",
        "Recent run outcomes",
        if recent.is_empty() {
            "warn"
        } else if failed == recent.len() {
            "fail"
        } else if failed > 0 {
            "warn"
        } else {
            "ok"
        },
        if recent.is_empty() {
            "No recorded runs to judge from.".to_string()
        } else {
            format!("{failed} of the last {} run(s) failed.", recent.len())
        },
        json!({ "considered": recent.len(), "failed": failed }),
    ));

    Ok(Json(ok(finish(definition, started_at, steps))))
}

fn finish(
    definition: &crate::state::ProductDefinition,
    started_at: String,
    steps: Vec<RunbookStep>,
) -> RunbookRun {
    let count = |status: &str| steps.iter().filter(|s| s.status == status).count();
    let failed = count("fail");
    let warned = count("warn");

    let run = RunbookRun {
        id: format!("rbk_{}", Uuid::now_v7()),
        product_slug: definition.slug.into(),
        product_title: definition.title.into(),
        status: if failed > 0 {
            "failed"
        } else if warned > 0 {
            "degraded"
        } else {
            "ok"
        }
        .into(),
        started_at,
        finished_at: now_rfc3339(),
        summary: format!(
            "{} ok, {warned} warning(s), {failed} failure(s).",
            count("ok")
        ),
        steps,
    };

    if let Err(error) = db::record_runbook_run(&run) {
        tracing::warn!("failed to record runbook run {}: {error}", run.id);
    }
    run
}

pub(super) async fn list_runbook_runs() -> Json<crate::models::ApiEnvelope<Vec<RunbookRun>>> {
    Json(ok(db::runbook_runs(50)))
}

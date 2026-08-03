use std::{collections::HashMap, time::Duration};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::{
    db,
    models::{
        now_rfc3339, ok, CapabilitiesEvidence, HealthEndpointEvidence, Observation,
        OverviewResponse, OverviewSummary, ProductHealthSnapshot, ProductHealthStatus,
        ProductRunDetailResponse, ProductRunsSnapshotResponse, ProductRuntimeItem, RunsEvidence,
        StartupChecksEvidence, PRODUCT_TAGLINE, PRODUCT_TITLE, PRODUCT_VERSION,
    },
    startup,
    state::{product_catalog, AppState, ProductDefinition},
};

use super::{
    api_error, authorized_get, build_target_url, contract_check, contract_checks_for_failed_health,
    contract_checks_for_unavailable_product, contract_checks_with_health_error,
    contract_drift_count, fetch_product_capabilities, fetch_product_runs,
    hive_core_action_run_values, parse_response_body, pick_url, resolve_api_url,
    resolved_auth_mode, resolved_legacy_api_key_configured, resolved_machine_auth_configured,
    resolved_service_token_configured, ProductHealthBody, ProductProbeSnapshot, ProductStoredAuth,
    StartupChecksBody,
};
use patchhive_product_core::contract;

pub(super) async fn overview(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<OverviewResponse>> {
    let suite_settings = db::suite_settings();
    let products = materialized_runtime_products(&state);
    let summary = summarize_products(&products);
    let snapshot = match db::latest_suite_snapshot_cycle() {
        Ok(Some(cycle)) => Observation::observed(cycle),
        Ok(None) => Observation::not_observed("No background suite snapshot has completed yet."),
        Err(error) => Observation::failed(format!("Could not read suite snapshot state: {error}")),
    };
    Json(ok(OverviewResponse {
        product: PRODUCT_TITLE,
        tagline: PRODUCT_TAGLINE,
        suite_settings,
        snapshot,
        summary,
        products,
    }))
}

pub(super) async fn products(
    State(state): State<AppState>,
) -> Json<crate::models::ApiEnvelope<Vec<ProductRuntimeItem>>> {
    Json(ok(materialized_runtime_products(&state)))
}

pub(super) async fn product_runs(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<
    Json<crate::models::ApiEnvelope<ProductRunsSnapshotResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    let definition = product_catalog()
        .iter()
        .find(|product| product.slug == slug)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "unknown_product", "Unknown product."))?;
    let snapshot = match db::materialized_product_run_snapshot(&definition.slug) {
        Ok(Some(snapshot)) => snapshot,
        Ok(None) => unavailable_runs_snapshot(
            definition,
            Observation::not_observed("The background poller has not captured this product yet."),
        ),
        Err(error) => unavailable_runs_snapshot(
            definition,
            Observation::failed(format!(
                "Could not read the materialized run snapshot: {error}"
            )),
        ),
    };
    let _ = state;
    Ok(Json(ok(snapshot)))
}

fn unavailable_runs_snapshot(
    definition: &ProductDefinition,
    runs: Observation<Vec<contract::ProductRunSummary>>,
) -> ProductRunsSnapshotResponse {
    ProductRunsSnapshotResponse {
        slug: definition.slug.clone(),
        title: definition.title.clone(),
        api_url: definition.default_api_url.clone(),
        auth_mode: "unknown".into(),
        machine_auth_configured: false,
        service_token_configured: false,
        legacy_api_key_configured: false,
        checked_at: now_rfc3339(),
        runs,
    }
}

pub(super) async fn product_run_detail(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
) -> Result<
    Json<crate::models::ApiEnvelope<ProductRunDetailResponse>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
> {
    let definition = product_catalog()
        .iter()
        .find(|product| product.slug == slug)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "unknown_product", "Unknown product."))?;
    let overrides = db::product_overrides();
    let override_item = overrides.get(&definition.slug);
    let api_url = resolve_api_url(override_item.map(|item| item.api_url.as_str()), definition);
    let auth = ProductStoredAuth::from_override(override_item);

    if definition.slug == "hive-core" {
        let detail = db::action_event(&id)
            .map(|event| serde_json::to_value(event).unwrap_or(Value::Null))
            .ok_or_else(|| {
                api_error(StatusCode::NOT_FOUND, "run_not_found", "Run was not found.")
            })?;
        return Ok(Json(ok(ProductRunDetailResponse {
            slug: definition.slug.clone(),
            title: definition.title.clone(),
            api_url,
            auth_mode: resolved_auth_mode(definition, &auth),
            machine_auth_configured: resolved_machine_auth_configured(definition, &auth),
            service_token_configured: resolved_service_token_configured(definition, &auth),
            legacy_api_key_configured: resolved_legacy_api_key_configured(definition, &auth),
            checked_at: now_rfc3339(),
            detail_path: format!("/runs/{id}"),
            detail_ok: true,
            remote_status: Some(200),
            error: String::new(),
            detail,
        })));
    }

    let enabled = override_item.map(|item| item.enabled).unwrap_or(true);
    if !enabled {
        return Err(api_error(
            StatusCode::CONFLICT,
            "product_disabled",
            "HiveCore will not fetch run detail from a disabled product.",
        ));
    }
    if api_url.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "product_unconfigured",
            "Configure this product API URL before fetching run detail.",
        ));
    }

    if !auth.machine_auth_configured() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "product_service_token_missing",
            "Save or provision this product's service token in HiveCore settings before fetching protected run detail.",
        ));
    }

    let capabilities = fetch_product_capabilities(&state.client, &api_url, &auth)
        .await
        .map_err(|message| {
            api_error(StatusCode::BAD_GATEWAY, "capabilities_unavailable", message)
        })?;
    if !capabilities.hivecore.can_read_run_detail {
        return Err(api_error(
            StatusCode::CONFLICT,
            "run_detail_unsupported",
            "This product does not advertise run detail support yet.",
        ));
    }

    let detail_path = build_run_detail_path(&capabilities.routes.run_detail_template, &id)
        .map_err(|message| api_error(StatusCode::BAD_REQUEST, "invalid_run_id", message))?;
    let detail_url = build_target_url(&api_url, &detail_path, &HashMap::new())
        .map_err(|message| api_error(StatusCode::BAD_REQUEST, "invalid_run_detail_url", message))?;
    let response = authorized_get(&state.client, detail_url.as_str(), &auth)
        .timeout(Duration::from_secs(3))
        .send()
        .await;

    let checked_at = now_rfc3339();
    let (detail_ok, remote_status, error, detail) = match response {
        Ok(response) => {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            (
                status.is_success(),
                Some(status.as_u16()),
                if status.is_success() {
                    String::new()
                } else {
                    format!("{} returned HTTP {status}", detail_path)
                },
                parse_response_body(&text),
            )
        }
        Err(err) => (
            false,
            None,
            format!("Could not reach {detail_path}: {err}"),
            json!({ "error": err.to_string() }),
        ),
    };

    Ok(Json(ok(ProductRunDetailResponse {
        slug: definition.slug.clone(),
        title: definition.title.clone(),
        api_url,
        auth_mode: resolved_auth_mode(definition, &auth),
        machine_auth_configured: resolved_machine_auth_configured(definition, &auth),
        service_token_configured: resolved_service_token_configured(definition, &auth),
        legacy_api_key_configured: resolved_legacy_api_key_configured(definition, &auth),
        checked_at,
        detail_path,
        detail_ok,
        remote_status,
        error,
        detail,
    })))
}

pub(super) async fn build_runtime_products(state: &AppState) -> Vec<ProductRuntimeItem> {
    let overrides = db::product_overrides();
    futures_util::future::join_all(product_catalog().iter().map(|definition| {
        build_product_runtime(state, definition, overrides.get(&definition.slug))
    }))
    .await
}

pub(crate) fn materialized_runtime_products(_state: &AppState) -> Vec<ProductRuntimeItem> {
    let overrides = db::product_overrides();
    match db::materialized_product_snapshots() {
        Ok(snapshots) => {
            let mut by_slug = snapshots
                .into_iter()
                .map(|snapshot| (snapshot.slug.clone(), snapshot))
                .collect::<HashMap<_, _>>();
            product_catalog()
                .iter()
                .map(|definition| {
                    by_slug.remove(&definition.slug).unwrap_or_else(|| {
                        unavailable_product_runtime(
                            definition,
                            overrides.get(&definition.slug),
                            false,
                            "The background poller has not captured this product yet.",
                        )
                    })
                })
                .collect()
        }
        Err(error) => product_catalog()
            .iter()
            .map(|definition| {
                unavailable_product_runtime(
                    definition,
                    overrides.get(&definition.slug),
                    true,
                    &format!("Could not read the materialized suite snapshot: {error}"),
                )
            })
            .collect(),
    }
}

fn unavailable_product_runtime(
    definition: &ProductDefinition,
    override_item: Option<&crate::models::ProductOverride>,
    failed: bool,
    reason: &str,
) -> ProductRuntimeItem {
    let auth = ProductStoredAuth::from_override(override_item);
    let enabled = override_item.map(|item| item.enabled).unwrap_or(true);
    let not_applicable = !enabled;
    let reason = if not_applicable {
        "Product is disabled in HiveCore settings."
    } else {
        reason
    };
    let health_status = if enabled {
        ProductHealthStatus::Unknown
    } else {
        ProductHealthStatus::Disabled
    };
    ProductRuntimeItem {
        slug: definition.slug.clone(),
        title: definition.title.clone(),
        icon: definition.icon.clone(),
        lane: definition.lane.clone(),
        role: definition.role.clone(),
        repo: definition.repo.clone(),
        enabled,
        frontend_url: pick_url(
            override_item.map(|item| item.frontend_url.as_str()),
            &definition.default_frontend_url,
        ),
        api_url: resolve_api_url(override_item.map(|item| item.api_url.as_str()), definition),
        auth_mode: resolved_auth_mode(definition, &auth),
        machine_auth_configured: resolved_machine_auth_configured(definition, &auth),
        service_token_configured: resolved_service_token_configured(definition, &auth),
        legacy_api_key_configured: resolved_legacy_api_key_configured(definition, &auth),
        notes: override_item
            .map(|item| item.notes.clone())
            .unwrap_or_default(),
        status: health_status.as_str().into(),
        health: ProductHealthSnapshot {
            status: health_status,
            health_endpoint: unavailable_observation(failed, not_applicable, reason),
            version: unavailable_observation(failed, not_applicable, reason),
            database_ok: unavailable_observation(failed, not_applicable, reason),
            startup_checks: unavailable_observation(failed, not_applicable, reason),
            capabilities: unavailable_observation(failed, not_applicable, reason),
            runs: unavailable_observation(failed, not_applicable, reason),
            checked_at: now_rfc3339(),
        },
        hivecore: None,
        actions: Vec::new(),
        links: Vec::new(),
        contract_checks: contract_checks_for_unavailable_product("snapshot_unavailable"),
        contract_drift_count: 0,
        run_detail_template: String::new(),
        recent_runs: Vec::new(),
    }
}

fn unavailable_observation<T>(failed: bool, not_applicable: bool, reason: &str) -> Observation<T> {
    if not_applicable {
        Observation::not_applicable(reason)
    } else if failed {
        Observation::failed(reason)
    } else {
        Observation::not_observed(reason)
    }
}

static SNAPSHOT_LOOP_STARTED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

pub(crate) fn start_snapshot_loop() {
    if SNAPSHOT_LOOP_STARTED.set(()).is_err() {
        return;
    }
    tokio::spawn(async {
        let state = AppState::new();
        loop {
            refresh_suite_snapshot(&state).await;
            tokio::time::sleep(Duration::from_secs(snapshot_interval_seconds())).await;
        }
    });
}

async fn refresh_suite_snapshot(state: &AppState) {
    let cycle_id = format!("snapshot_{}", uuid::Uuid::now_v7());
    let started_at = now_rfc3339();
    if let Err(error) = db::start_suite_snapshot_cycle(&cycle_id, &started_at) {
        tracing::error!(%error, %cycle_id, "could not start suite snapshot cycle");
        return;
    }
    let products = build_runtime_products(state).await;
    let completed_at = now_rfc3339();
    if let Err(error) =
        db::complete_suite_snapshot_cycle(&cycle_id, &started_at, &completed_at, &products)
    {
        tracing::error!(%error, %cycle_id, "could not materialize suite snapshot");
        if let Err(settle_error) =
            db::fail_suite_snapshot_cycle(&cycle_id, &started_at, &completed_at, &error.to_string())
        {
            tracing::error!(%settle_error, %cycle_id, "could not record failed suite snapshot cycle");
        }
    }
}

fn snapshot_interval_seconds() -> u64 {
    std::env::var("HIVE_CORE_SNAPSHOT_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30)
        .clamp(5, 300)
}

pub(super) async fn build_product_runtime(
    state: &AppState,
    definition: &ProductDefinition,
    override_item: Option<&crate::models::ProductOverride>,
) -> ProductRuntimeItem {
    let enabled = override_item.map(|item| item.enabled).unwrap_or(true);
    let frontend_url = pick_url(
        override_item.map(|item| item.frontend_url.as_str()),
        &definition.default_frontend_url,
    );
    let api_url = resolve_api_url(override_item.map(|item| item.api_url.as_str()), definition);
    let notes = override_item
        .map(|item| item.notes.clone())
        .unwrap_or_default();
    let auth = ProductStoredAuth::from_override(override_item);

    let probe = if definition.slug == "hive-core" {
        local_hive_core_probe()
    } else if !enabled {
        ProductProbeSnapshot {
            health: ProductHealthSnapshot {
                status: ProductHealthStatus::Disabled,
                health_endpoint: Observation::not_applicable(
                    "Product is disabled in HiveCore settings.",
                ),
                version: Observation::not_applicable("Product is disabled in HiveCore settings."),
                database_ok: Observation::not_applicable(
                    "Product is disabled in HiveCore settings.",
                ),
                startup_checks: Observation::not_applicable(
                    "Product is disabled in HiveCore settings.",
                ),
                capabilities: Observation::not_applicable(
                    "Product is disabled in HiveCore settings.",
                ),
                runs: Observation::not_applicable("Product is disabled in HiveCore settings."),
                checked_at: now_rfc3339(),
            },
            hivecore: None,
            actions: Vec::new(),
            links: Vec::new(),
            contract_checks: contract_checks_for_unavailable_product("disabled"),
            run_detail_template: String::new(),
            recent_runs: Vec::new(),
        }
    } else if api_url.is_empty() {
        ProductProbeSnapshot {
            health: ProductHealthSnapshot {
                status: ProductHealthStatus::Unconfigured,
                health_endpoint: Observation::not_observed("Product API URL is not configured."),
                version: Observation::not_observed(
                    "Product API URL is required before version can be queried.",
                ),
                database_ok: Observation::not_observed(
                    "Product API URL is required before database health can be queried.",
                ),
                startup_checks: Observation::not_observed(
                    "Product API URL is required before startup checks can be queried.",
                ),
                capabilities: Observation::not_observed(
                    "Product API URL is required before capabilities can be queried.",
                ),
                runs: Observation::not_observed(
                    "Product API URL is required before run history can be queried.",
                ),
                checked_at: now_rfc3339(),
            },
            hivecore: None,
            actions: Vec::new(),
            links: Vec::new(),
            contract_checks: contract_checks_for_unavailable_product("unconfigured"),
            run_detail_template: String::new(),
            recent_runs: Vec::new(),
        }
    } else {
        fetch_product_health(&state.client, &definition.slug, &api_url, &auth).await
    };

    ProductRuntimeItem {
        slug: definition.slug.clone(),
        title: definition.title.clone(),
        icon: definition.icon.clone(),
        lane: definition.lane.clone(),
        role: definition.role.clone(),
        repo: definition.repo.clone(),
        enabled,
        frontend_url,
        api_url,
        auth_mode: resolved_auth_mode(definition, &auth),
        machine_auth_configured: resolved_machine_auth_configured(definition, &auth),
        service_token_configured: resolved_service_token_configured(definition, &auth),
        legacy_api_key_configured: resolved_legacy_api_key_configured(definition, &auth),
        notes,
        status: probe.health.status.as_str().into(),
        health: probe.health,
        hivecore: probe.hivecore,
        actions: probe.actions,
        links: probe.links,
        contract_drift_count: contract_drift_count(&probe.contract_checks),
        contract_checks: probe.contract_checks,
        run_detail_template: probe.run_detail_template,
        recent_runs: probe.recent_runs,
    }
}

pub(super) async fn fetch_product_health(
    client: &reqwest::Client,
    slug: &str,
    api_url: &str,
    auth: &ProductStoredAuth,
) -> ProductProbeSnapshot {
    let normalized = api_url.trim_end_matches('/');
    let checked_at = now_rfc3339();
    let health_url = format!("{normalized}/health");

    let probe_started = std::time::Instant::now();
    let health_response = authorized_get(client, &health_url, auth)
        .timeout(Duration::from_secs(3))
        .send()
        .await;
    // Time only the request. Parsing the body afterwards is HiveCore's cost, not the
    // product's, and folding it in would make every product look slower than it is.
    let latency_ms = probe_started.elapsed().as_millis() as u64;
    let Ok(health_response) = health_response else {
        // Record the failure. Uptime is the share of probes that succeeded, so an
        // unreachable product that leaves no row is a product with a perfect record —
        // which is the opposite of what just happened. The elapsed time is stored so
        // the row is complete, but `healthy = false` keeps it out of the latency
        // distribution: a timeout is not a round trip.
        db::record_product_probe(slug, latency_ms, false, &checked_at);
        return ProductProbeSnapshot {
            health: ProductHealthSnapshot {
                status: ProductHealthStatus::Offline,
                checked_at,
                health_endpoint: Observation::failed("Could not reach /health."),
                version: Observation::not_observed("/health could not be reached."),
                database_ok: Observation::not_observed("/health could not be reached."),
                startup_checks: Observation::not_observed(
                    "Health must pass before startup checks are meaningful.",
                ),
                capabilities: Observation::not_observed(
                    "Health must pass before capabilities are meaningful.",
                ),
                runs: Observation::not_observed(
                    "Health must pass before run history is meaningful.",
                ),
            },
            hivecore: None,
            actions: Vec::new(),
            links: Vec::new(),
            contract_checks: contract_checks_for_failed_health(),
            run_detail_template: String::new(),
            recent_runs: Vec::new(),
        };
    };

    if !health_response.status().is_success() {
        db::record_product_probe(slug, latency_ms, false, &checked_at);
        return ProductProbeSnapshot {
            health: ProductHealthSnapshot {
                status: ProductHealthStatus::Offline,
                checked_at,
                health_endpoint: Observation::failed(format!(
                    "/health returned HTTP {}",
                    health_response.status()
                )),
                version: Observation::not_observed("/health did not return a usable document."),
                database_ok: Observation::not_observed("/health did not return a usable document."),
                startup_checks: Observation::not_observed(
                    "Health must pass before startup checks are meaningful.",
                ),
                capabilities: Observation::not_observed(
                    "Health must pass before capabilities are meaningful.",
                ),
                runs: Observation::not_observed(
                    "Health must pass before run history is meaningful.",
                ),
            },
            hivecore: None,
            actions: Vec::new(),
            links: Vec::new(),
            contract_checks: contract_checks_with_health_error(format!(
                "/health returned HTTP {}",
                health_response.status()
            )),
            run_detail_template: String::new(),
            recent_runs: Vec::new(),
        };
    }

    let (health_body, health_endpoint, version, database_ok) = match health_response
        .json::<ProductHealthBody>()
        .await
    {
        Ok(body) => {
            let evidence = HealthEndpointEvidence {
                reported_status: body
                    .status
                    .clone()
                    .map(Observation::observed)
                    .unwrap_or_else(|| {
                        Observation::not_observed("/health did not report `status`.")
                    }),
                latency_ms,
                config_errors: body
                    .config_errors
                    .map(Observation::observed)
                    .unwrap_or_else(|| {
                        Observation::not_observed("/health did not report `config_errors`.")
                    }),
            };
            let version = body
                .version
                .clone()
                .map(Observation::observed)
                .unwrap_or_else(|| Observation::not_observed("/health did not report `version`."));
            let database_ok = body.db_ok.map(Observation::observed).unwrap_or_else(|| {
                Observation::not_observed("/health did not report database health.")
            });
            (
                Some(body),
                Observation::observed(evidence),
                version,
                database_ok,
            )
        }
        Err(error) => (
            None,
            Observation::failed(format!("Could not decode /health response: {error}")),
            Observation::not_observed("The /health document could not be decoded."),
            Observation::not_observed("The /health document could not be decoded."),
        ),
    };

    let checks_url = format!("{normalized}/startup/checks");
    let checks_response = authorized_get(client, &checks_url, auth)
        .timeout(Duration::from_secs(3))
        .send()
        .await;

    let (startup_checks, startup_ok, extra_error) = match checks_response {
        Ok(response) if response.status().is_success() => {
            match response.json::<StartupChecksBody>().await {
                Ok(body) => {
                    let (errors, warnings, infos) = startup::summarize_check_levels(&body.checks);
                    (
                        Observation::observed(StartupChecksEvidence {
                            errors,
                            warnings,
                            infos,
                        }),
                        true,
                        String::new(),
                    )
                }
                Err(error) => {
                    let reason = format!("Could not decode /startup/checks response: {error}");
                    (Observation::failed(&reason), false, reason)
                }
            }
        }
        Ok(response) => {
            let reason = format!("/startup/checks returned HTTP {}", response.status());
            (Observation::failed(&reason), false, reason)
        }
        Err(_) => {
            let reason = "Could not reach /startup/checks.".to_string();
            (Observation::failed(&reason), false, reason)
        }
    };

    let (capabilities_ok, actions, links, hivecore, run_detail_template, capabilities_error) =
        match fetch_product_capabilities(client, api_url, auth).await {
            Ok(body) => (
                true,
                body.actions,
                body.links,
                Some(body.hivecore),
                body.routes.run_detail_template,
                String::new(),
            ),
            Err(message) => (false, Vec::new(), Vec::new(), None, String::new(), message),
        };
    let capabilities = if capabilities_ok {
        Observation::observed(CapabilitiesEvidence {
            action_count: actions.len() as u32,
        })
    } else {
        Observation::failed(&capabilities_error)
    };
    let (runs_ok, recent_runs, runs_error) = fetch_product_runs(client, api_url, auth).await;
    let runs = if runs_ok {
        Observation::observed(RunsEvidence {
            run_count: recent_runs.len() as u32,
        })
    } else if auth.machine_auth_configured() {
        Observation::failed(&runs_error)
    } else {
        Observation::not_observed(if runs_error.is_empty() {
            "HiveCore has no machine credential for protected run history."
        } else {
            &runs_error
        })
    };
    let run_detail_ok = hivecore
        .as_ref()
        .map(|support| support.can_read_run_detail && !run_detail_template.trim().is_empty())
        .unwrap_or(false);
    let contract_checks = vec![
        contract_check("health", "Health", "/health", true, "ok", ""),
        contract_check(
            "startup_checks",
            "Startup checks",
            "/startup/checks",
            startup_ok,
            if startup_ok { "ok" } else { "failed" },
            &extra_error,
        ),
        contract_check(
            "capabilities",
            "Capabilities",
            "/capabilities",
            capabilities_ok,
            if capabilities_ok { "ok" } else { "failed" },
            &capabilities_error,
        ),
        contract_check(
            "runs",
            "Runs",
            "/runs",
            runs_ok,
            if runs_ok {
                "ok"
            } else if !auth.machine_auth_configured() {
                "locked"
            } else {
                "failed"
            },
            &runs_error,
        ),
        contract_check(
            "run_detail",
            "Run detail",
            if run_detail_template.trim().is_empty() {
                "/runs/{id}"
            } else {
                run_detail_template.as_str()
            },
            run_detail_ok,
            if run_detail_ok {
                "advertised"
            } else if !capabilities_ok {
                "skipped"
            } else {
                "missing"
            },
            if run_detail_ok {
                ""
            } else if !capabilities_ok {
                "Capabilities must pass before HiveCore can confirm run detail support."
            } else {
                "Product does not advertise run detail support."
            },
        ),
    ];

    let config_errors = health_body.as_ref().and_then(|body| body.config_errors);
    let base_status = health_body.as_ref().and_then(|body| body.status.as_deref());
    let database_unhealthy = health_body
        .as_ref()
        .and_then(|body| body.db_ok)
        .is_some_and(|ok| !ok);
    let runs_integration_failed = auth.machine_auth_configured() && !runs_ok;
    let startup_has_errors = startup_checks
        .value()
        .is_some_and(|summary| summary.errors > 0);
    let status = if health_endpoint.is_failed()
        || !startup_ok
        || startup_has_errors
        || config_errors.is_none_or(|errors| errors > 0)
        || base_status != Some("ok")
        || database_unhealthy
        || !capabilities_ok
        || runs_integration_failed
    {
        ProductHealthStatus::Degraded
    } else {
        ProductHealthStatus::Online
    };

    // One real sample per probe, retained so the deck can draw a latency history it
    // did not invent. Failures are recorded too: uptime is the share of probes that
    // succeeded, so dropping them would make every product look perfect.
    db::record_product_probe(
        slug,
        latency_ms,
        status == ProductHealthStatus::Online,
        &checked_at,
    );

    ProductProbeSnapshot {
        health: ProductHealthSnapshot {
            status,
            health_endpoint,
            version,
            database_ok,
            startup_checks,
            capabilities,
            runs,
            checked_at,
        },
        hivecore,
        actions,
        links,
        contract_checks,
        run_detail_template,
        recent_runs,
    }
}

pub(super) fn local_hive_core_probe() -> ProductProbeSnapshot {
    let checks = startup::startup_checks();
    let (startup_errors, startup_warns, startup_infos) = startup::summarize_check_levels(&checks);
    let db_ok = db::health_check();
    let status = if startup_errors > 0 || !db_ok {
        "degraded"
    } else {
        "online"
    };
    let actions = vec![contract::action(
        "save_settings",
        "Save suite settings",
        "PUT",
        "/settings",
        "Persist suite-wide defaults and per-product launch/API overrides.",
        false,
        contract::ActionSafety::operator_required(contract::ActionEffect::WritesLocalState),
    )
    .credential_requirements(["suite:control"])];
    let recent_runs = contract::runs_from_values("hive-core", hive_core_action_run_values(6)).runs;
    ProductProbeSnapshot {
        health: ProductHealthSnapshot {
            status: if status == "online" {
                ProductHealthStatus::Online
            } else {
                ProductHealthStatus::Degraded
            },
            health_endpoint: Observation::not_applicable(
                "HiveCore reads its own in-process state; there is no network probe.",
            ),
            version: Observation::observed(PRODUCT_VERSION.into()),
            database_ok: Observation::observed(db_ok),
            startup_checks: Observation::observed(StartupChecksEvidence {
                errors: startup_errors,
                warnings: startup_warns,
                infos: startup_infos,
            }),
            capabilities: Observation::observed(CapabilitiesEvidence {
                action_count: actions.len() as u32,
            }),
            runs: Observation::observed(RunsEvidence {
                run_count: recent_runs.len() as u32,
            }),
            checked_at: now_rfc3339(),
        },
        hivecore: Some(contract::HiveCoreLifecycleSupport {
            can_launch: true,
            can_start_runs: false,
            can_list_runs: true,
            can_read_run_detail: true,
            can_apply_settings: true,
        }),
        actions,
        links: vec![
            contract::link("overview", "Overview", "/overview"),
            contract::link("products", "Products", "/products"),
            contract::link("settings", "Settings", "/settings"),
        ],
        contract_checks: vec![
            contract_check("health", "Health", "/health", true, "ok", ""),
            contract_check(
                "startup_checks",
                "Startup checks",
                "/startup/checks",
                true,
                "ok",
                "",
            ),
            contract_check(
                "capabilities",
                "Capabilities",
                "/capabilities",
                true,
                "ok",
                "",
            ),
            contract_check("runs", "Runs", "/runs", true, "ok", ""),
            contract_check(
                "run_detail",
                "Run detail",
                "/runs/{id}",
                true,
                "advertised",
                "",
            ),
        ],
        run_detail_template: "/runs/{id}".into(),
        recent_runs,
    }
}

pub(super) fn build_run_detail_path(template: &str, id: &str) -> Result<String, String> {
    let id = id.trim();
    if id.is_empty() {
        return Err("Run id is required.".into());
    }
    if id.contains('/')
        || id.contains('?')
        || id.contains('#')
        || id.contains('{')
        || id.contains('}')
    {
        return Err(
            "Run id contains characters HiveCore will not place into a product path.".into(),
        );
    }
    let mut params = HashMap::new();
    params.insert("id".into(), id.to_string());
    let template = if template.trim().is_empty() {
        "/runs/{id}"
    } else {
        template.trim()
    };
    super::dispatch::fill_path_template(template, &params)
}

pub(super) fn summarize_products(products: &[ProductRuntimeItem]) -> OverviewSummary {
    let mut summary = OverviewSummary {
        total_products: products.len() as u32,
        ..OverviewSummary::default()
    };

    for product in products {
        match product.status.as_str() {
            "online" => {
                summary.enabled_products += 1;
                summary.online_products += 1;
            }
            "degraded" => {
                summary.enabled_products += 1;
                summary.degraded_products += 1;
            }
            "offline" => {
                summary.enabled_products += 1;
                summary.offline_products += 1;
            }
            "unconfigured" => {
                summary.enabled_products += 1;
                summary.unconfigured_products += 1;
            }
            "disabled" => {
                summary.disabled_products += 1;
            }
            "unknown" => {
                summary.enabled_products += 1;
                summary.unknown_products += 1;
            }
            _ => {
                summary.unknown_products += 1;
            }
        }
    }

    summary
}

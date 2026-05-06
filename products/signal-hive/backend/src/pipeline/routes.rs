// routes.rs — HTTP route handlers

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use patchhive_product_core::contract;
use serde_json::json;

use crate::state::AppState;

use super::scanning::{enrich_scan_record, run_scan_record};
use super::utils::{bad_request, clamp_params, internal_error};

pub async fn capabilities() -> Json<contract::ProductCapabilities> {
    Json(contract::capabilities(
        "signal-hive",
        "SignalHive",
        vec![
            contract::action(
                "smoke_check",
                "Run smoke check",
                "POST",
                "/smoke",
                "Verify SignalHive is ready for HiveCore dispatch without running a live GitHub scan.",
                false,
            ),
            contract::action(
                "scan",
                "Run signal scan",
                "POST",
                "/scan",
                "Discover maintenance signals across repositories from configured topics and languages.",
                true,
            ),
            contract::action(
                "run_schedule_now",
                "Run saved schedule",
                "POST",
                "/schedules/{name}/run",
                "Trigger a saved SignalHive scan schedule immediately.",
                true,
            ),
        ],
        vec![
            contract::link("history", "History", "/history"),
            contract::link("presets", "Presets", "/presets"),
            contract::link("schedules", "Schedules", "/schedules"),
        ],
    ))
}

pub async fn runs() -> Json<contract::ProductRunsResponse> {
    Json(contract::runs_from_history(
        "signal-hive",
        crate::db::list_scans().unwrap_or_default(),
    ))
}

pub async fn smoke_check() -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>
{
    let scans = crate::db::list_scans().map_err(internal_error)?;
    Ok(Json(json!({
        "ok": true,
        "service": "signal-hive",
        "check": "smoke_check",
        "scan_count": scans.len(),
        "latest_scan_id": scans.first().map(|scan| scan.id.clone()),
        "message": "SignalHive accepted HiveCore service-token dispatch without running a live GitHub scan."
    })))
}

pub async fn scan(
    State(state): State<AppState>,
    Json(params): Json<crate::models::ScanParams>,
) -> Result<Json<crate::models::ScanRecord>, (StatusCode, Json<serde_json::Value>)> {
    let params = clamp_params(params);
    if params.search_query.is_empty() && params.topics.is_empty() && params.languages.is_empty() {
        let allowlist = crate::db::repo_list_sets().map_err(internal_error)?.0;
        if allowlist.is_empty() {
            return Err(bad_request(
                "Provide at least a search query, topic, or language, or configure an allowlist.",
            ));
        }
    }

    let record = run_scan_record(&state, params, "manual", None)
        .await
        .map_err(internal_error)?;
    Ok(Json(record))
}

pub async fn history() -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let scans = crate::db::list_scans().map_err(internal_error)?;
    Ok(Json(json!({ "scans": scans })))
}

pub async fn history_detail(
    Path(id): Path<String>,
) -> Result<Json<crate::models::ScanRecord>, (StatusCode, Json<serde_json::Value>)> {
    match crate::db::get_scan(&id).map_err(internal_error)? {
        Some(mut scan) => {
            enrich_scan_record(&mut scan).map_err(internal_error)?;
            Ok(Json(scan))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Scan not found" })),
        )),
    }
}

pub async fn report(
    Path(id): Path<String>,
) -> Result<Json<crate::models::ScanReport>, (StatusCode, Json<serde_json::Value>)> {
    match crate::db::get_scan(&id).map_err(internal_error)? {
        Some(mut scan) => {
            enrich_scan_record(&mut scan).map_err(internal_error)?;
            Ok(Json(super::scanning::build_scan_report(&scan)))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Scan not found" })),
        )),
    }
}

pub async fn timeline(
    Path(id): Path<String>,
) -> Result<Json<crate::models::ScanTimeline>, (StatusCode, Json<serde_json::Value>)> {
    match crate::db::scan_timeline(&id, 12).map_err(internal_error)? {
        Some(timeline) => Ok(Json(timeline)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Scan not found" })),
        )),
    }
}

use anyhow::{anyhow, Context, Result};
use tracing::{info, warn};

use crate::{
    db,
    models::{RefactorScanResult, ScanRequest},
    state::AppState,
};

use super::scanning::{build_scan_result_for_input, MAX_SCAN_FILES};

pub async fn run_schedule_now(state: &AppState, schedule_name: &str) -> Result<RefactorScanResult> {
    let schedule = db::get_schedule(schedule_name)?
        .ok_or_else(|| anyhow!("RefactorScout schedule `{schedule_name}` was not found"))?;
    run_saved_schedule(state, &schedule.name, schedule.decode_payload()?).await
}

pub fn start_scheduler(state: AppState) {
    tokio::spawn(async move {
        loop {
            match db::claim_due_schedules(4) {
                Ok(schedules) => {
                    for schedule in schedules {
                        let name = schedule.name.clone();
                        match schedule.decode_payload::<ScanRequest>() {
                            Ok(request) => match run_saved_schedule(&state, &name, request).await {
                                Ok(record) => {
                                    info!(
                                        "RefactorScout scheduled scan '{name}' completed as {}",
                                        record.id
                                    );
                                }
                                Err(error) => {
                                    warn!("RefactorScout scheduled scan '{name}' failed: {error}");
                                }
                            },
                            Err(error) => {
                                if let Err(write_error) = db::record_schedule_result(
                                    &name,
                                    None,
                                    "error",
                                    Some(&error.to_string()),
                                ) {
                                    warn!(
                                        "failed to store RefactorScout schedule error for {name}: {write_error}"
                                    );
                                }
                                warn!(
                                    "RefactorScout scheduled scan '{name}' has an invalid payload: {error}"
                                );
                            }
                        }
                    }
                }
                Err(error) => warn!("RefactorScout scheduler poll failed: {error}"),
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });
}

async fn run_saved_schedule(
    state: &AppState,
    schedule_name: &str,
    request: ScanRequest,
) -> Result<RefactorScanResult> {
    let result = execute_scheduled_scan(state, schedule_name, request).await;
    match result {
        Ok(record) => {
            db::record_schedule_result(schedule_name, Some(&record.id), "ok", None)?;
            Ok(record)
        }
        Err(error) => {
            db::record_schedule_result(schedule_name, None, "error", Some(&error.to_string()))?;
            Err(error)
        }
    }
}

async fn execute_scheduled_scan(
    state: &AppState,
    schedule_name: &str,
    request: ScanRequest,
) -> Result<RefactorScanResult> {
    let repo_path = request.repo_path.trim();
    if repo_path.is_empty() {
        return Err(anyhow!("scheduled repository target is required"));
    }
    let max_files = request.max_files.clamp(25, MAX_SCAN_FILES);
    let mut result = build_scan_result_for_input(state, repo_path, max_files).await?;
    result.trigger_type = "schedule".into();
    result.schedule_name = Some(schedule_name.into());
    db::save_scan(&result).context("failed to save scheduled RefactorScout scan")?;
    Ok(result)
}

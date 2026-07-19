use anyhow::{anyhow, Context, Result};
use patchhive_product_core::{contract::TargetSelectionMode, scheduling::ProductSchedule};
use tracing::{info, warn};

use crate::{
    db,
    models::{RefactorScanResult, ScanRequest},
    state::AppState,
};

use super::{
    discovery::{normalize_discovery_scope, repository_policy_allows, select_repository},
    scanning::{build_scan_result_for_input, github_repo_target_for_input, MAX_SCAN_FILES},
};

pub async fn run_schedule_now(state: &AppState, schedule_name: &str) -> Result<RefactorScanResult> {
    let schedule = db::get_schedule(schedule_name)?
        .ok_or_else(|| anyhow!("RefactorScout schedule `{schedule_name}` was not found"))?;
    run_saved_schedule(state, &schedule).await
}

pub fn start_scheduler(state: AppState) {
    tokio::spawn(async move {
        loop {
            match db::claim_due_schedules(4) {
                Ok(schedules) => {
                    for schedule in schedules {
                        let name = schedule.name.clone();
                        match run_saved_schedule(&state, &schedule).await {
                            Ok(record) => {
                                info!(
                                    "RefactorScout scheduled scan '{name}' completed as {}",
                                    record.id
                                );
                            }
                            Err(error) => {
                                warn!("RefactorScout scheduled scan '{name}' failed: {error}");
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
    schedule: &ProductSchedule,
) -> Result<RefactorScanResult> {
    let result = execute_scheduled_scan(state, schedule).await;
    match result {
        Ok(record) => {
            db::record_schedule_result(&schedule.name, Some(&record.id), "ok", None)?;
            Ok(record)
        }
        Err(error) => {
            db::record_schedule_result(&schedule.name, None, "error", Some(&error.to_string()))?;
            Err(error)
        }
    }
}

async fn execute_scheduled_scan(
    state: &AppState,
    schedule: &ProductSchedule,
) -> Result<RefactorScanResult> {
    let request = schedule.decode_payload::<ScanRequest>()?;
    let repo_path = match schedule.target_selection_mode {
        TargetSelectionMode::Direct => {
            let repo_path = request.repo_path.trim();
            if repo_path.is_empty() {
                return Err(anyhow!("scheduled repository target is required"));
            }
            if let Some(target) = github_repo_target_for_input(repo_path) {
                let label = target.label();
                if !repository_policy_allows(state, &label).await? {
                    return Err(anyhow!(
                        "Repository policy blocks RefactorScout from scanning `{label}`."
                    ));
                }
            }
            repo_path.to_string()
        }
        TargetSelectionMode::Discovery => {
            let scope = normalize_discovery_scope(&request.discovery);
            let recent = db::recently_scanned_repositories(&schedule.name, scope.cooldown_days)?;
            select_repository(state, &scope, &recent).await?
        }
    };
    let max_files = request.max_files.clamp(25, MAX_SCAN_FILES);
    let mut result = build_scan_result_for_input(state, &repo_path, max_files).await?;
    result.trigger_type = "schedule".into();
    result.schedule_name = Some(schedule.name.clone());
    result.target_selection_mode = schedule.target_selection_mode;
    db::save_scan(&result).context("failed to save scheduled RefactorScout scan")?;
    Ok(result)
}

//! Suite runs: an ordered sequence of product dispatches, recorded as one unit.
//!
//! This is the smallest honest version of the orchestration described in
//! docs/hivecore-architecture.md §3.11 — a run whose steps are dispatches. It
//! deliberately does not yet resolve one step's inputs from a previous step's
//! artifacts; each step carries its own payload. Passing data between stages needs
//! agreed artifact shapes, and inventing them here would bake in a guess.
//!
//! Every step goes through the same `dispatch_once` a manual dispatch uses, so a
//! suite run cannot reach anything an operator could not reach by hand: destructive,
//! approval-gated and PR-opening actions are refused identically. A suite run is a
//! sequencing convenience, never an elevation of authority.

use axum::{extract::State, http::StatusCode, Json};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    db,
    models::{now_rfc3339, ok, StartSuiteRunRequest, SuiteRun, SuiteRunStep, SuiteRunStepInput},
    state::AppState,
};

use super::{api_error, dispatch::dispatch_once};

type ApiResult<T> = Result<
    Json<crate::models::ApiEnvelope<T>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
>;

pub(super) async fn start_suite_run(
    State(state): State<AppState>,
    Json(request): Json<StartSuiteRunRequest>,
) -> ApiResult<SuiteRun> {
    if request.steps.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "empty_suite_run",
            "A suite run needs at least one step.",
        ));
    }
    if request.steps.len() > 50 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "suite_run_too_long",
            "A suite run is limited to 50 steps.",
        ));
    }

    let name = if request.name.trim().is_empty() {
        format!("{} step suite run", request.steps.len())
    } else {
        request.name.trim().to_string()
    };

    let mut run = SuiteRun {
        id: format!("srun_{}", Uuid::now_v7()),
        name,
        status: "running".into(),
        started_at: now_rfc3339(),
        finished_at: String::new(),
        summary: String::new(),
        steps: request
            .steps
            .iter()
            .map(|step| SuiteRunStep {
                product: step.product.clone(),
                action: step.action.clone(),
                status: "queued".into(),
                message: String::new(),
                remote_status: None,
                event_id: String::new(),
                started_at: String::new(),
                finished_at: String::new(),
            })
            .collect(),
    };

    // Persist before executing. A run that dies mid-flight should leave a record
    // saying so, not vanish.
    if let Err(error) = db::record_suite_run(&run) {
        return Err(api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "suite_run_save_failed",
            format!("Could not record the suite run: {error}"),
        ));
    }

    execute(
        &state,
        &mut run,
        &request.steps,
        request.continue_on_failure,
    )
    .await;

    if let Err(error) = db::record_suite_run(&run) {
        tracing::warn!("failed to persist completed suite run {}: {error}", run.id);
    }

    Ok(Json(ok(run)))
}

async fn execute(
    state: &AppState,
    run: &mut SuiteRun,
    inputs: &[SuiteRunStepInput],
    continue_on_failure: bool,
) {
    let mut halted = false;

    for (index, input) in inputs.iter().enumerate() {
        if halted {
            run.steps[index].status = "skipped".into();
            run.steps[index].message =
                "Skipped because an earlier step failed and the run halts on failure.".into();
            continue;
        }

        run.steps[index].status = "running".into();
        run.steps[index].started_at = now_rfc3339();

        let payload = if input.payload.is_null() {
            Value::Object(Default::default())
        } else {
            input.payload.clone()
        };

        match dispatch_once(state, &input.product, &input.action, payload).await {
            Ok(response) => {
                let event = response.event;
                let dispatched = event.status == "dispatched";
                run.steps[index].status = if dispatched { "dispatched" } else { "failed" }.into();
                run.steps[index].remote_status = event.remote_status;
                run.steps[index].event_id = event.id;
                run.steps[index].message = if dispatched {
                    if response.started_run {
                        "Product accepted the action and started a run.".into()
                    } else {
                        "Product accepted the action.".into()
                    }
                } else {
                    event.error
                };
                if !dispatched && !continue_on_failure {
                    halted = true;
                }
            }
            Err((_status, body)) => {
                // HiveCore refused before reaching the product — a guard, a missing
                // token, an unknown action. Recorded as the step's own failure.
                run.steps[index].status = "failed".into();
                run.steps[index].message = body
                    .0
                    .error
                    .as_ref()
                    .map(|error| error.message.clone())
                    .unwrap_or_else(|| "HiveCore refused the step.".into());
                if !continue_on_failure {
                    halted = true;
                }
            }
        }

        run.steps[index].finished_at = now_rfc3339();
    }

    let dispatched = run
        .steps
        .iter()
        .filter(|s| s.status == "dispatched")
        .count();
    let failed = run.steps.iter().filter(|s| s.status == "failed").count();
    let skipped = run.steps.iter().filter(|s| s.status == "skipped").count();

    run.status = if failed == 0 {
        "completed"
    } else if halted {
        "halted"
    } else {
        "failed"
    }
    .into();
    run.finished_at = now_rfc3339();
    run.summary = format!("{dispatched} dispatched, {failed} failed, {skipped} skipped.");
}

pub(super) async fn list_suite_runs() -> Json<crate::models::ApiEnvelope<Vec<SuiteRun>>> {
    Json(ok(db::suite_runs(50)))
}

pub(super) async fn suite_run_detail(id: String) -> ApiResult<SuiteRun> {
    db::suite_run(&id).map(|run| Json(ok(run))).ok_or_else(|| {
        api_error(
            StatusCode::NOT_FOUND,
            "suite_run_not_found",
            "Suite run was not found.",
        )
    })
}

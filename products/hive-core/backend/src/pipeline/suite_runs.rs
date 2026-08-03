//! Suite runs: an ordered sequence of product dispatches, recorded as one unit.
//!
//! This is the orchestration described in docs/hivecore-architecture.md §3.11 — a run
//! whose steps are dispatches, optionally chained so one step acts on what an earlier
//! step found.
//!
//! Every dispatch goes through the same guarded dispatcher a manual dispatch uses.
//! Destructive actions remain refused; approval-gated and PR-opening actions become
//! durable pending approvals and halt the run before remote work starts. A suite run
//! sequences work; it never widens authority. Chaining does not change that — a step
//! expanded over ten targets is ten dispatches through the same guard, not one
//! privileged batch.

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    db,
    models::{
        now_rfc3339, ok, StartSuiteRunRequest, SuiteRun, SuiteRunStep, SuiteRunStepInput,
        SuiteRunTargets,
    },
    state::AppState,
};

use super::{api_error, dispatch::dispatch_with_approval};
use patchhive_product_core::approvals::ApprovalOrigin;

type ApiResult<T> = Result<
    Json<crate::models::ApiEnvelope<T>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
>;

/// Ceiling on how many dispatches one composed step may expand into.
///
/// Enforced here rather than trusted from the request: a cap the caller supplies is a
/// cap the caller can raise, which is not a cap. A composer that wants fewer can ask
/// for fewer; nothing can ask for more.
const MAX_TARGETS_PER_STEP: u32 = 25;

/// Ceiling on total dispatches in one run, counting expansions.
///
/// The composed-step limit is separate and smaller. This one exists because fan-out
/// multiplies: five composed steps each expanding to twenty-five is a hundred and
/// twenty-five dispatches from a form that looked like five.
const MAX_DISPATCHES_PER_RUN: usize = 100;

#[derive(Debug, Deserialize)]
pub struct TomlPipelineRequest {
    pub pipeline_toml: String,
}

#[derive(Debug, Deserialize)]
struct PipelineDocument {
    #[serde(default)]
    name: String,
    #[serde(default)]
    continue_on_failure: bool,
    #[serde(rename = "stage")]
    stages: Vec<PipelineStage>,
}

#[derive(Debug, Deserialize)]
struct PipelineStage {
    product: String,
    action: String,
    #[serde(default)]
    input: Option<toml::Value>,
    #[serde(default)]
    gate: Option<String>,
    #[serde(default)]
    targets: Option<SuiteRunTargets>,
}

pub(super) async fn execute_toml_pipeline(
    State(state): State<AppState>,
    Json(request): Json<TomlPipelineRequest>,
) -> ApiResult<SuiteRun> {
    if request.pipeline_toml.len() > 100_000 {
        return Err(api_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "pipeline_toml_too_large",
            "A declarative pipeline is limited to 100000 characters.",
        ));
    }
    let document = toml::from_str::<PipelineDocument>(&request.pipeline_toml).map_err(|error| {
        api_error(
            StatusCode::BAD_REQUEST,
            "invalid_pipeline_toml",
            format!("The pipeline TOML is invalid: {error}"),
        )
    })?;
    let mut steps = Vec::with_capacity(document.stages.len());
    for (index, stage) in document.stages.into_iter().enumerate() {
        if stage.product.trim().is_empty() || stage.action.trim().is_empty() {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "invalid_pipeline_stage",
                format!("Stage {} requires product and action.", index + 1),
            ));
        }
        let payload = stage
            .input
            .map(|value| serde_json::to_value(value).unwrap_or(Value::Null))
            .unwrap_or_else(|| Value::Object(Map::new()));
        if !payload.is_object() {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "invalid_pipeline_input",
                format!("Stage {} input must be a TOML table.", index + 1),
            ));
        }
        steps.push(SuiteRunStepInput {
            product: stage.product,
            action: stage.action,
            payload,
            targets: stage.targets,
            gate: stage.gate,
        });
    }
    start_suite_run(
        State(state),
        Json(StartSuiteRunRequest {
            name: document.name,
            steps,
            continue_on_failure: document.continue_on_failure,
        }),
    )
    .await
}

pub(super) async fn start_suite_run(
    State(state): State<AppState>,
    Json(request): Json<StartSuiteRunRequest>,
) -> ApiResult<SuiteRun> {
    let blocking_pauses = db::blocking_pauses(None, None, None).map_err(|error| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "pause_authority_unavailable",
            format!("HiveCore could not read pause authority: {error}"),
        )
    })?;
    if !blocking_pauses.is_empty() {
        return Err(api_error(
            StatusCode::LOCKED,
            "suite_run_paused",
            format!(
                "HiveCore blocked the suite run because these scopes are paused: {}.",
                blocking_pauses
                    .iter()
                    .map(|pause| pause.target.storage_key())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
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
    if let Err(message) = validate_targets(&request.steps) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_step_reference",
            message,
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
        // Steps are built as the run executes: a step with a target reference is not
        // one step but however many targets the earlier step turned out to produce,
        // which is not knowable before that step has run.
        steps: Vec::new(),
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

/// Reject references that cannot be satisfied, before anything is dispatched.
///
/// A forward or self reference is not a runtime failure to report halfway through a
/// run that has already touched repositories — it is a composition mistake, and the
/// honest moment to say so is before the first dispatch.
fn validate_targets(steps: &[SuiteRunStepInput]) -> Result<(), String> {
    for (index, step) in steps.iter().enumerate() {
        let Some(targets) = &step.targets else {
            continue;
        };
        let position = index + 1;
        if targets.from_step == 0 || targets.from_step >= position {
            return Err(format!(
                "Step {position} takes targets from step {}, which must be an earlier step in the same run.",
                targets.from_step
            ));
        }
        if targets.assign_to.trim().is_empty() {
            return Err(format!(
                "Step {position} takes targets from step {} but does not say which payload field to set.",
                targets.from_step
            ));
        }
    }
    Ok(())
}

/// The response bodies of completed steps, indexed 1-based to match the operator's view.
type StepOutputs = Vec<Option<Value>>;

async fn execute(
    state: &AppState,
    run: &mut SuiteRun,
    inputs: &[SuiteRunStepInput],
    continue_on_failure: bool,
) {
    let mut halted = false;
    let mut outputs: StepOutputs = vec![None; inputs.len() + 1];

    for (index, input) in inputs.iter().enumerate() {
        let position = index + 1;

        if halted {
            run.steps.push(skipped_step(
                input,
                "Skipped because an earlier step failed or is waiting for approval.",
            ));
            continue;
        }

        if run.steps.len() >= MAX_DISPATCHES_PER_RUN {
            run.steps.push(skipped_step(
                input,
                &format!(
                    "Skipped: this run reached the {MAX_DISPATCHES_PER_RUN} dispatch ceiling."
                ),
            ));
            continue;
        }

        if let Some(gate) = input
            .gate
            .as_deref()
            .map(str::trim)
            .filter(|gate| !gate.is_empty())
        {
            match evaluate_gate(gate, &outputs) {
                Ok(true) => {}
                Ok(false) => {
                    run.steps.push(skipped_step(
                        input,
                        &format!("Result gate evaluated false: {gate}"),
                    ));
                    continue;
                }
                Err(message) => {
                    run.steps.push(failed_step(
                        input,
                        &format!("Result gate could not be evaluated: {message}"),
                    ));
                    if !continue_on_failure {
                        halted = true;
                    }
                    continue;
                }
            }
        }

        // A plain step is one dispatch; a step with a target reference is one per
        // resolved target.
        let expansions = match plan_expansions(&outputs, input) {
            Ok(expansions) => expansions,
            Err(message) => {
                run.steps.push(failed_step(input, &message));
                if !continue_on_failure {
                    halted = true;
                }
                continue;
            }
        };

        let mut last_body = None;

        for (target, payload) in expansions {
            if run.steps.len() >= MAX_DISPATCHES_PER_RUN {
                run.steps.push(skipped_step(
                    input,
                    &format!(
                        "Skipped: this run reached the {MAX_DISPATCHES_PER_RUN} dispatch ceiling."
                    ),
                ));
                break;
            }

            let mut step = SuiteRunStep {
                product: input.product.clone(),
                action: input.action.clone(),
                payload: payload.clone(),
                target: target.clone(),
                status: "running".into(),
                message: String::new(),
                remote_status: None,
                event_id: String::new(),
                approval_id: String::new(),
                started_at: now_rfc3339(),
                finished_at: String::new(),
            };

            match dispatch_with_approval(
                state,
                &input.product,
                &input.action,
                payload,
                ApprovalOrigin::SuiteRun {
                    run_id: run.id.clone(),
                },
                None,
            )
            .await
            {
                Ok(crate::models::DispatchActionResponse::Dispatched { event, started_run }) => {
                    let event = *event;
                    let dispatched = event.status == "dispatched";
                    step.status = if dispatched { "dispatched" } else { "failed" }.into();
                    step.remote_status = event.remote_status;
                    step.event_id = event.id;
                    step.message = if dispatched {
                        if started_run {
                            "Product accepted the action and started a run.".into()
                        } else {
                            "Product accepted the action.".into()
                        }
                    } else {
                        event.error
                    };
                    if dispatched {
                        last_body = Some(crate::work_engine::normalized_response(
                            &event.response_json,
                        ));
                    } else if !continue_on_failure {
                        halted = true;
                    }
                }
                Ok(crate::models::DispatchActionResponse::ApprovalRequired { approval }) => {
                    step.status = "pending_approval".into();
                    step.approval_id = approval.id.clone();
                    step.message = format!(
                        "Waiting for operator approval {} before this exact dispatch can run.",
                        approval.id
                    );
                    halted = true;
                }
                Err((_status, body)) => {
                    // HiveCore refused before reaching the product — a guard, a
                    // missing token, an unknown action. Recorded as the step's own
                    // failure.
                    step.status = "failed".into();
                    step.message = body
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

            step.finished_at = now_rfc3339();
            run.steps.push(step);

            if halted {
                break;
            }
        }

        // Only a successful body is offered downstream. A later step must not fan out
        // over the contents of a failed response.
        outputs[position] = last_body;
    }

    summarize(run, halted);
}

fn summarize(run: &mut SuiteRun, halted: bool) {
    let count = |status: &str| run.steps.iter().filter(|s| s.status == status).count();
    let dispatched = count("dispatched");
    let failed = count("failed");
    let skipped = count("skipped");
    let pending_approval = count("pending_approval");

    run.status = if pending_approval > 0 {
        "awaiting_approval"
    } else if failed == 0 && dispatched > 0 && skipped == 0 {
        "completed"
    } else if failed == 0 {
        // No failures and no dispatches means every step was skipped. Reporting that
        // as "completed" would describe an empty run as a successful one.
        "halted"
    } else if halted {
        "halted"
    } else {
        "failed"
    }
    .into();
    run.finished_at = now_rfc3339();
    run.summary = format!(
        "{dispatched} dispatched, {pending_approval} awaiting approval, {failed} failed, {skipped} skipped."
    );
}

pub(super) fn record_approved_dispatch(
    run_id: &str,
    approval_id: &str,
    event: &crate::models::ProductActionEvent,
    started_run: bool,
) -> Result<(), String> {
    let mut run = db::suite_run_result(run_id)
        .map_err(|error| format!("could not read suite run {run_id}: {error}"))?
        .ok_or_else(|| format!("suite run {run_id} no longer exists"))?;
    apply_approved_dispatch(&mut run, approval_id, event, started_run)?;
    db::record_suite_run(&run)
        .map_err(|error| format!("could not save suite run {run_id}: {error}"))
}

fn apply_approved_dispatch(
    run: &mut SuiteRun,
    approval_id: &str,
    event: &crate::models::ProductActionEvent,
    started_run: bool,
) -> Result<(), String> {
    let run_id = run.id.clone();
    let step = run
        .steps
        .iter_mut()
        .find(|step| step.approval_id == approval_id)
        .ok_or_else(|| format!("suite run {run_id} does not contain approval {approval_id}"))?;
    let dispatched = event.status == "dispatched";
    step.status = if dispatched { "dispatched" } else { "failed" }.into();
    step.message = if dispatched {
        if started_run {
            "Operator approved the action; the product accepted it and started a run.".into()
        } else {
            "Operator approved the action; the product accepted it.".into()
        }
    } else {
        event.error.clone()
    };
    step.remote_status = event.remote_status;
    step.event_id.clone_from(&event.id);
    step.finished_at = now_rfc3339();
    summarize(run, true);
    Ok(())
}

fn base_payload(payload: &Value) -> Value {
    if payload.is_object() {
        payload.clone()
    } else {
        Value::Object(Map::new())
    }
}

fn skipped_step(input: &SuiteRunStepInput, message: &str) -> SuiteRunStep {
    stub_step(input, "skipped", message)
}

fn failed_step(input: &SuiteRunStepInput, message: &str) -> SuiteRunStep {
    stub_step(input, "failed", message)
}

fn stub_step(input: &SuiteRunStepInput, status: &str, message: &str) -> SuiteRunStep {
    SuiteRunStep {
        product: input.product.clone(),
        action: input.action.clone(),
        payload: base_payload(&input.payload),
        target: String::new(),
        status: status.into(),
        message: message.to_string(),
        remote_status: None,
        event_id: String::new(),
        approval_id: String::new(),
        started_at: now_rfc3339(),
        finished_at: now_rfc3339(),
    }
}

fn describe_source(targets: &SuiteRunTargets) -> String {
    let path = if targets.path.trim().is_empty() {
        "the response body".to_string()
    } else {
        format!("`{}`", targets.path.trim())
    };
    if targets.field.trim().is_empty() {
        path
    } else {
        format!("{path} field `{}`", targets.field.trim())
    }
}

/// Turn one composed step into the dispatches it will actually make.
///
/// Separated from the execution loop so the expansion is testable without a network:
/// how many dispatches a step becomes, and what payload each carries, is the part
/// that decides how much of the world a run touches, and it should not only be
/// exercised against live products.
///
/// Returns `(target, payload)` pairs. A plain step yields exactly one with an empty
/// target. A referencing step yields one per resolved target, each with `assign_to`
/// set to that target.
fn plan_expansions(
    outputs: &StepOutputs,
    input: &SuiteRunStepInput,
) -> Result<Vec<(String, Value)>, String> {
    let Some(targets) = &input.targets else {
        return Ok(vec![(String::new(), base_payload(&input.payload))]);
    };

    let values = resolve_targets(outputs, targets)?;
    if values.is_empty() {
        // Zero targets is a failure, not a quiet success. A step that dispatched
        // nothing and reported "completed" is how a run that did no work gets read as
        // a run that found nothing wrong.
        return Err(format!(
            "Resolved 0 targets from step {} ({}), so this step did not run.",
            targets.from_step,
            describe_source(targets)
        ));
    }

    Ok(values
        .into_iter()
        .map(|target| {
            let mut payload = base_payload(&input.payload);
            if let Value::Object(map) = &mut payload {
                map.insert(
                    targets.assign_to.trim().to_string(),
                    Value::String(target.clone()),
                );
            }
            (target, payload)
        })
        .collect())
}

/// Resolve an earlier step's output into a list of target strings.
///
/// Everything here fails loudly. A path that does not exist, a value that is not an
/// array, elements with no usable string — each is reported as itself rather than
/// resolving to an empty list, because an empty list and a wrong path are
/// indistinguishable at the call site and only one of them is the operator's fault.
fn resolve_targets(
    outputs: &StepOutputs,
    targets: &SuiteRunTargets,
) -> Result<Vec<String>, String> {
    let Some(Some(body)) = outputs.get(targets.from_step) else {
        return Err(format!(
            "Step {} produced no successful response to take targets from.",
            targets.from_step
        ));
    };

    let mut node = body;
    for segment in targets
        .path
        .split('.')
        .filter(|part| !part.trim().is_empty())
    {
        node = node.get(segment.trim()).ok_or_else(|| {
            format!(
                "Step {} response has no `{}` at `{}`.",
                targets.from_step,
                segment.trim(),
                targets.path.trim()
            )
        })?;
    }

    let items = node.as_array().ok_or_else(|| {
        format!(
            "Step {} {} is not a list, so it cannot supply targets.",
            targets.from_step,
            describe_source(targets)
        )
    })?;

    let field = targets.field.trim();
    let mut resolved = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for item in items {
        let value = if field.is_empty() {
            item.as_str().map(str::to_string)
        } else {
            item.get(field).and_then(Value::as_str).map(str::to_string)
        };
        let Some(value) = value.map(|value| value.trim().to_string()) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        // The same repository surfacing twice is one target. Dispatching to it twice
        // spends budget and doubles the evidence for one piece of work.
        if seen.insert(value.clone()) {
            resolved.push(value);
        }
    }

    let cap = if targets.max_targets == 0 {
        MAX_TARGETS_PER_STEP
    } else {
        targets.max_targets.min(MAX_TARGETS_PER_STEP)
    } as usize;
    resolved.truncate(cap);

    Ok(resolved)
}

fn evaluate_gate(expression: &str, outputs: &StepOutputs) -> Result<bool, String> {
    let expression = expression.trim();
    if let Some(reference) = expression
        .strip_prefix("exists(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return Ok(resolve_gate_reference(outputs, reference.trim()).is_ok());
    }
    for operator in ["!=", "==", ">=", "<=", ">", "<"] {
        if let Some((left, right)) = expression.split_once(operator) {
            let actual = resolve_gate_reference(outputs, left.trim())?;
            let expected = parse_gate_literal(right.trim())?;
            return compare_gate_values(actual, &expected, operator);
        }
    }
    Err("supported gates are exists(reference) or reference ==, !=, >, >=, <, <= literal".into())
}

fn resolve_gate_reference<'a>(
    outputs: &'a StepOutputs,
    reference: &str,
) -> Result<&'a Value, String> {
    let reference = reference.trim().trim_start_matches('$');
    let mut parts = reference.split('.');
    if parts.next() != Some("stages") {
        return Err("gate references must start with $stages.<step-number>".into());
    }
    let step = parts
        .next()
        .ok_or_else(|| "gate reference omitted a step number".to_string())?
        .parse::<usize>()
        .map_err(|_| "gate step number must be a positive integer".to_string())?;
    let mut value = outputs
        .get(step)
        .and_then(Option::as_ref)
        .ok_or_else(|| format!("stage {step} has no successful result"))?;
    for segment in parts {
        value = value
            .get(segment)
            .ok_or_else(|| format!("stage {step} result has no `{segment}` field"))?;
    }
    Ok(value)
}

fn parse_gate_literal(value: &str) -> Result<Value, String> {
    if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        return Ok(Value::String(value[1..value.len() - 1].to_owned()));
    }
    serde_json::from_str(value)
        .map_err(|_| "gate literals must be quoted strings, JSON numbers, booleans, or null".into())
}

fn compare_gate_values(actual: &Value, expected: &Value, operator: &str) -> Result<bool, String> {
    match operator {
        "==" => Ok(actual == expected),
        "!=" => Ok(actual != expected),
        ">" | ">=" | "<" | "<=" => {
            let left = actual
                .as_f64()
                .ok_or_else(|| "ordered gate comparison requires a numeric result".to_string())?;
            let right = expected
                .as_f64()
                .ok_or_else(|| "ordered gate comparison requires a numeric literal".to_string())?;
            Ok(match operator {
                ">" => left > right,
                ">=" => left >= right,
                "<" => left < right,
                "<=" => left <= right,
                _ => unreachable!(),
            })
        }
        _ => Err("unsupported gate operator".into()),
    }
}

pub(super) async fn list_suite_runs() -> Json<crate::models::ApiEnvelope<Vec<SuiteRun>>> {
    Json(ok(db::suite_runs(50)))
}

pub(super) async fn suite_run_detail(id: String) -> ApiResult<SuiteRun> {
    db::suite_run_result(&id)
        .map_err(|error| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "suite_run_read_failed",
                format!("Could not read suite run: {error}"),
            )
        })?
        .map(|run| Json(ok(run)))
        .ok_or_else(|| {
            api_error(
                StatusCode::NOT_FOUND,
                "suite_run_not_found",
                "Suite run was not found.",
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn step(product: &str, targets: Option<SuiteRunTargets>) -> SuiteRunStepInput {
        SuiteRunStepInput {
            product: product.into(),
            action: "scan".into(),
            payload: Value::Null,
            targets,
            gate: None,
        }
    }

    fn reference(from_step: usize, path: &str, field: &str, max: u32) -> SuiteRunTargets {
        SuiteRunTargets {
            from_step,
            path: path.into(),
            field: field.into(),
            assign_to: "repo".into(),
            max_targets: max,
        }
    }

    fn outputs(body: Value) -> StepOutputs {
        vec![None, Some(body), None]
    }

    #[test]
    fn a_forward_reference_is_rejected_before_anything_dispatches() {
        // Catching this at runtime would mean reporting a composition mistake halfway
        // through a run that has already touched repositories.
        let steps = vec![
            step("signal-hive", Some(reference(2, "repos", "full_name", 5))),
            step("repo-reaper", None),
        ];
        assert!(validate_targets(&steps).is_err());
    }

    #[test]
    fn a_self_reference_is_rejected() {
        let steps = vec![step(
            "signal-hive",
            Some(reference(1, "repos", "full_name", 5)),
        )];
        assert!(validate_targets(&steps).is_err());
    }

    #[test]
    fn a_reference_without_a_destination_field_is_rejected() {
        let mut targets = reference(1, "repos", "full_name", 5);
        targets.assign_to = "  ".into();
        let steps = vec![
            step("signal-hive", None),
            step("repo-reaper", Some(targets)),
        ];
        assert!(validate_targets(&steps).is_err());
    }

    #[test]
    fn targets_resolve_from_a_nested_path_and_field() {
        let body = json!({ "data": { "repos": [
            { "full_name": "owner/one" },
            { "full_name": "owner/two" },
        ]}});
        let resolved = resolve_targets(&outputs(body), &reference(1, "data.repos", "full_name", 5))
            .expect("targets should resolve");
        assert_eq!(resolved, vec!["owner/one", "owner/two"]);
    }

    #[test]
    fn the_server_cap_beats_a_larger_client_cap() {
        // A cap the caller can raise is not a cap.
        let items: Vec<Value> = (0..60)
            .map(|index| json!({ "full_name": format!("owner/repo{index}") }))
            .collect();
        let body = json!({ "repos": items });
        let resolved = resolve_targets(&outputs(body), &reference(1, "repos", "full_name", 10_000))
            .expect("targets should resolve");
        assert_eq!(resolved.len(), MAX_TARGETS_PER_STEP as usize);
    }

    #[test]
    fn a_smaller_client_cap_is_honoured() {
        let items: Vec<Value> = (0..10)
            .map(|index| json!({ "full_name": format!("owner/repo{index}") }))
            .collect();
        let resolved = resolve_targets(
            &outputs(json!({ "repos": items })),
            &reference(1, "repos", "full_name", 3),
        )
        .expect("targets should resolve");
        assert_eq!(resolved.len(), 3);
    }

    #[test]
    fn duplicate_targets_collapse_to_one_dispatch() {
        let body = json!({ "repos": [
            { "full_name": "owner/one" },
            { "full_name": "owner/one" },
            { "full_name": "owner/two" },
        ]});
        let resolved = resolve_targets(&outputs(body), &reference(1, "repos", "full_name", 25))
            .expect("targets should resolve");
        assert_eq!(resolved, vec!["owner/one", "owner/two"]);
    }

    #[test]
    fn a_wrong_path_is_an_error_not_an_empty_list() {
        // An empty list and a mistyped path are indistinguishable at the call site,
        // and only one of them is the operator's fault.
        let body = json!({ "repos": [{ "full_name": "owner/one" }] });
        let error = resolve_targets(&outputs(body), &reference(1, "results", "full_name", 5))
            .expect_err("a missing path should be reported");
        assert!(error.contains("results"));
    }

    #[test]
    fn a_non_list_value_is_reported_rather_than_iterated() {
        let body = json!({ "repos": "owner/one" });
        assert!(resolve_targets(&outputs(body), &reference(1, "repos", "full_name", 5)).is_err());
    }

    #[test]
    fn a_step_that_never_succeeded_supplies_no_targets() {
        // Fanning out over a failed step's body would act on whatever an error
        // response happened to contain.
        let empty: StepOutputs = vec![None, None];
        assert!(resolve_targets(&empty, &reference(1, "repos", "full_name", 5)).is_err());
    }

    #[test]
    fn elements_without_the_named_field_are_dropped_not_stringified() {
        let body = json!({ "repos": [
            { "full_name": "owner/one" },
            { "name": "no-full-name" },
            { "full_name": "" },
        ]});
        let resolved = resolve_targets(&outputs(body), &reference(1, "repos", "full_name", 25))
            .expect("targets should resolve");
        assert_eq!(resolved, vec!["owner/one"]);
    }

    #[test]
    fn a_bare_string_array_resolves_without_a_field() {
        let body = json!({ "repos": ["owner/one", "owner/two"] });
        let resolved = resolve_targets(&outputs(body), &reference(1, "repos", "", 25))
            .expect("targets should resolve");
        assert_eq!(resolved, vec!["owner/one", "owner/two"]);
    }

    #[test]
    fn an_all_skipped_run_is_not_reported_as_completed() {
        // Zero failures is not success when nothing ran.
        let mut run = SuiteRun {
            id: "srun_test".into(),
            name: "test".into(),
            status: "running".into(),
            started_at: now_rfc3339(),
            finished_at: String::new(),
            summary: String::new(),
            steps: vec![skipped_step(&step("signal-hive", None), "skipped")],
        };
        summarize(&mut run, false);
        assert_eq!(run.status, "halted");
    }

    #[test]
    fn a_pending_approval_is_not_reported_as_failure_or_completion() {
        let mut approval_step =
            stub_step(&step("repo-reaper", None), "pending_approval", "waiting");
        approval_step.approval_id = "apr_test".into();
        let mut run = SuiteRun {
            id: "srun_approval".into(),
            name: "approval test".into(),
            status: "running".into(),
            started_at: now_rfc3339(),
            finished_at: String::new(),
            summary: String::new(),
            steps: vec![approval_step],
        };
        summarize(&mut run, true);
        assert_eq!(run.status, "awaiting_approval");
        assert!(run.summary.contains("1 awaiting approval"));
    }

    #[test]
    fn result_gates_resolve_prior_stage_evidence_and_fail_closed() {
        let outputs = vec![None, Some(json!({"recommendation": "safe", "risk": 12}))];
        assert!(evaluate_gate("$stages.1.recommendation == 'safe'", &outputs).unwrap());
        assert!(!evaluate_gate("$stages.1.risk > 20", &outputs).unwrap());
        assert!(evaluate_gate("exists($stages.1.recommendation)", &outputs).unwrap());
        assert!(evaluate_gate("$stages.2.recommendation == 'safe'", &outputs).is_err());
    }

    #[test]
    fn declarative_pipeline_toml_decodes_stage_gates() {
        let document: PipelineDocument = toml::from_str(
            r#"
name = "safe maintenance"

[[stage]]
product = "signal-hive"
action = "scan"
input = { max_repos = 2 }

[[stage]]
product = "repo-reaper"
action = "dry_run"
gate = "$stages.1.summary.total_repos > 0"
"#,
        )
        .expect("pipeline TOML should decode");
        assert_eq!(document.stages.len(), 2);
        assert_eq!(
            document.stages[1].gate.as_deref(),
            Some("$stages.1.summary.total_repos > 0")
        );
    }

    #[test]
    fn approved_suite_dispatch_reconciles_without_resuming_skipped_steps() {
        let mut approval_step =
            stub_step(&step("repo-reaper", None), "pending_approval", "waiting");
        approval_step.approval_id = "apr_test".into();
        let skipped = skipped_step(&step("release-sentry", None), "waiting on approval");
        let mut run = SuiteRun {
            id: "srun_approval".into(),
            name: "approval test".into(),
            status: "awaiting_approval".into(),
            started_at: now_rfc3339(),
            finished_at: now_rfc3339(),
            summary: String::new(),
            steps: vec![approval_step, skipped],
        };
        let event = crate::models::ProductActionEvent {
            id: "evt_approved".into(),
            product_slug: "repo-reaper".into(),
            action_id: "hunt".into(),
            action_label: "Run hunt".into(),
            method: "POST".into(),
            path: "/hunts".into(),
            target_url: "http://localhost/hunts".into(),
            status: "dispatched".into(),
            remote_status: Some(202),
            request_json: json!({}),
            response_json: json!({"run_id": "run_1"}),
            error: String::new(),
            created_at: now_rfc3339(),
        };

        apply_approved_dispatch(&mut run, "apr_test", &event, true)
            .expect("approved dispatch should reconcile");
        assert_eq!(run.status, "halted");
        assert_eq!(run.steps[0].status, "dispatched");
        assert_eq!(run.steps[0].event_id, "evt_approved");
        assert_eq!(run.steps[0].approval_id, "apr_test");
        assert_eq!(run.steps[1].status, "skipped");
    }

    #[test]
    fn the_wire_shape_the_deck_sends_deserializes_into_a_reference() {
        // `targets` is an Option, so a field-name mismatch between the deck and this
        // struct does not error — it deserializes to None and the step runs once,
        // unchained, reporting success. That is the worst available failure mode:
        // silent, and it looks like it worked. This pins the exact JSON the composer
        // emits so a rename on either side breaks a test instead of a run.
        let body = json!({
            "name": "nightly",
            "continue_on_failure": false,
            "steps": [
                { "product": "signal-hive", "action": "scan", "payload": { "languages": ["rust"] } },
                {
                    "product": "refactor-scout",
                    "action": "scan",
                    "payload": {},
                    "targets": {
                        "from_step": 1,
                        "path": "repos",
                        "field": "full_name",
                        "assign_to": "repo",
                        "max_targets": 5
                    }
                }
            ]
        });

        let request: StartSuiteRunRequest =
            serde_json::from_value(body).expect("composer payload should deserialize");

        assert_eq!(request.steps.len(), 2);
        assert!(request.steps[0].targets.is_none());
        assert_eq!(request.steps[0].payload["languages"][0], "rust");

        let targets = request.steps[1]
            .targets
            .as_ref()
            .expect("second step should carry a target reference");
        assert_eq!(targets.from_step, 1);
        assert_eq!(targets.path, "repos");
        assert_eq!(targets.field, "full_name");
        assert_eq!(targets.assign_to, "repo");
        assert_eq!(targets.max_targets, 5);
        assert!(validate_targets(&request.steps).is_ok());
    }

    #[test]
    fn a_step_with_no_payload_or_targets_still_deserializes() {
        // The composer omits both for a plain step; neither may be required.
        let request: StartSuiteRunRequest = serde_json::from_value(json!({
            "steps": [{ "product": "signal-hive", "action": "scan" }]
        }))
        .expect("a bare step should deserialize");
        assert!(request.steps[0].targets.is_none());
        assert!(request.steps[0].payload.is_null());
        assert_eq!(base_payload(&request.steps[0].payload), json!({}));
    }

    fn step_with(payload: Value, targets: Option<SuiteRunTargets>) -> SuiteRunStepInput {
        SuiteRunStepInput {
            product: "refactor-scout".into(),
            action: "scan".into(),
            payload,
            targets,
            gate: None,
        }
    }

    #[test]
    fn a_plain_step_expands_to_exactly_one_dispatch_keeping_its_payload() {
        let input = step_with(json!({ "repo": "owner/one", "depth": 2 }), None);
        let planned = plan_expansions(&vec![None], &input).expect("plain step should plan");

        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].0, "");
        assert_eq!(planned[0].1, json!({ "repo": "owner/one", "depth": 2 }));
    }

    #[test]
    fn each_target_gets_its_own_payload_with_the_field_assigned() {
        // The whole feature in one assertion: N targets become N dispatches, the
        // named field carries the target, and everything else the operator typed
        // survives onto every one of them.
        let outputs = outputs(json!({ "repos": [
            { "full_name": "owner/one" },
            { "full_name": "owner/two" },
        ]}));
        let input = step_with(
            json!({ "depth": 3 }),
            Some(reference(1, "repos", "full_name", 25)),
        );

        let planned = plan_expansions(&outputs, &input).expect("targets should plan");

        assert_eq!(planned.len(), 2);
        assert_eq!(planned[0].0, "owner/one");
        assert_eq!(planned[0].1, json!({ "depth": 3, "repo": "owner/one" }));
        assert_eq!(planned[1].1, json!({ "depth": 3, "repo": "owner/two" }));
    }

    #[test]
    fn the_assigned_field_overwrites_a_conflicting_literal() {
        // If the operator both typed `repo` and referenced targets, the reference is
        // the more specific instruction and must win — otherwise every expansion
        // would silently dispatch against the same hardcoded repository.
        let outputs = outputs(json!({ "repos": ["owner/one", "owner/two"] }));
        let input = step_with(
            json!({ "repo": "owner/typed" }),
            Some(reference(1, "repos", "", 25)),
        );

        let planned = plan_expansions(&outputs, &input).expect("targets should plan");

        assert_eq!(planned.len(), 2);
        assert_eq!(planned[0].1["repo"], "owner/one");
        assert_eq!(planned[1].1["repo"], "owner/two");
    }

    #[test]
    fn a_non_object_payload_becomes_an_object_rather_than_dropping_the_target() {
        // Payload is `Value`, so a caller can send a string or a list. The target
        // still has to land somewhere; silently dispatching without it would act on
        // whatever the product defaults to.
        let outputs = outputs(json!({ "repos": ["owner/one"] }));
        let input = step_with(json!("not-an-object"), Some(reference(1, "repos", "", 25)));

        let planned = plan_expansions(&outputs, &input).expect("targets should plan");

        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].1, json!({ "repo": "owner/one" }));
    }

    #[test]
    fn zero_targets_plans_nothing_and_says_so() {
        let outputs = outputs(json!({ "repos": [] }));
        let input = step_with(json!({}), Some(reference(1, "repos", "full_name", 25)));

        let error = plan_expansions(&outputs, &input).expect_err("empty targets should fail");
        assert!(error.contains("Resolved 0 targets"));
    }

    #[test]
    fn planning_respects_the_server_cap_so_a_run_cannot_be_widened_by_payload() {
        let items: Vec<Value> = (0..80).map(|i| json!(format!("owner/repo{i}"))).collect();
        let outputs = outputs(json!({ "repos": items }));
        let input = step_with(json!({}), Some(reference(1, "repos", "", 999)));

        let planned = plan_expansions(&outputs, &input).expect("targets should plan");
        assert_eq!(planned.len(), MAX_TARGETS_PER_STEP as usize);
    }
}

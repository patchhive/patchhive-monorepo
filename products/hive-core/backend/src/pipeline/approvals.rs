use axum::{extract::State, http::StatusCode, Json};
use patchhive_product_core::approvals::{ApprovalOrigin, ApprovalState};
use serde_json::Value;

use crate::{
    db,
    models::{now_rfc3339, ok, ApprovalReasonRequest, ApprovalRecord, DispatchActionResponse},
    state::AppState,
};

use super::{api_error, dispatch::dispatch_with_approval, suite_runs::record_approved_dispatch};

type ApiResult<T> = Result<
    Json<crate::models::ApiEnvelope<T>>,
    (StatusCode, Json<crate::models::ApiEnvelope<Value>>),
>;

pub(super) async fn list_approvals() -> ApiResult<Vec<ApprovalRecord>> {
    db::approvals(100)
        .map(|approvals| Json(ok(approvals)))
        .map_err(|error| approval_store_error("read", error))
}

pub(super) async fn grant_approval(id: String) -> ApiResult<ApprovalRecord> {
    let approval = db::grant_approval(&id, &now_rfc3339())
        .map_err(|error| approval_store_error("grant", error))?
        .ok_or_else(|| approval_not_found(&id))?;
    if !matches!(approval.lifecycle, ApprovalState::Granted { .. }) {
        return Err(approval_state_conflict(&approval, "granted"));
    }
    Ok(Json(ok(approval)))
}

pub(super) async fn deny_approval(
    id: String,
    Json(request): Json<ApprovalReasonRequest>,
) -> ApiResult<ApprovalRecord> {
    let reason = required_reason(&request.reason).ok_or_else(|| {
        api_error(
            StatusCode::BAD_REQUEST,
            "approval_reason_required",
            "Denying an approval requires a reason.",
        )
    })?;
    let approval = db::deny_approval(&id, &reason, &now_rfc3339())
        .map_err(|error| approval_store_error("deny", error))?
        .ok_or_else(|| approval_not_found(&id))?;
    if !matches!(approval.lifecycle, ApprovalState::Denied { .. }) {
        return Err(approval_state_conflict(&approval, "denied"));
    }
    Ok(Json(ok(approval)))
}

pub(super) async fn revoke_approval(
    id: String,
    Json(request): Json<ApprovalReasonRequest>,
) -> ApiResult<ApprovalRecord> {
    let reason = required_reason(&request.reason).ok_or_else(|| {
        api_error(
            StatusCode::BAD_REQUEST,
            "approval_reason_required",
            "Revoking an approval requires a reason.",
        )
    })?;
    let approval = db::revoke_approval(&id, &reason, &now_rfc3339())
        .map_err(|error| approval_store_error("revoke", error))?
        .ok_or_else(|| approval_not_found(&id))?;
    if !matches!(approval.lifecycle, ApprovalState::Revoked { .. }) {
        return Err(approval_state_conflict(&approval, "revoked"));
    }
    Ok(Json(ok(approval)))
}

pub(super) async fn dispatch_approved(
    State(state): State<AppState>,
    id: String,
) -> ApiResult<DispatchActionResponse> {
    let approval = db::approval(&id)
        .map_err(|error| approval_store_error("read", error))?
        .ok_or_else(|| approval_not_found(&id))?;
    let body = serde_json::to_value(&approval.dispatch).map_err(|error| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "approval_dispatch_invalid",
            format!("HiveCore could not reconstruct the approved dispatch: {error}"),
        )
    })?;
    let suite_run_id = match &approval.subject.origin {
        ApprovalOrigin::SuiteRun { run_id } => Some(run_id.clone()),
        ApprovalOrigin::OperatorDispatch => None,
    };
    let response = dispatch_with_approval(
        &state,
        &approval.subject.product,
        &approval.subject.action_id,
        body,
        approval.subject.origin.clone(),
        Some(&id),
    )
    .await?;
    if let (Some(run_id), DispatchActionResponse::Dispatched { event, started_run }) =
        (&suite_run_id, &response)
    {
        record_approved_dispatch(run_id, &id, event, *started_run).map_err(|message| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "suite_run_reconcile_failed",
                format!(
                    "The approved product action was attempted and the approval was consumed, but HiveCore could not update its suite-run evidence: {message}"
                ),
            )
        })?;
    }
    Ok(Json(ok(response)))
}

fn required_reason(reason: &str) -> Option<String> {
    let reason = reason.trim();
    (!reason.is_empty()).then(|| reason.to_string())
}

fn approval_not_found(id: &str) -> (StatusCode, Json<crate::models::ApiEnvelope<Value>>) {
    api_error(
        StatusCode::NOT_FOUND,
        "approval_not_found",
        format!("Approval '{id}' was not found."),
    )
}

fn approval_state_conflict(
    approval: &ApprovalRecord,
    requested: &str,
) -> (StatusCode, Json<crate::models::ApiEnvelope<Value>>) {
    api_error(
        StatusCode::CONFLICT,
        "approval_state_conflict",
        format!(
            "Approval cannot become {requested} from state '{}'.",
            approval.lifecycle.label()
        ),
    )
}

fn approval_store_error(
    operation: &str,
    error: rusqlite::Error,
) -> (StatusCode, Json<crate::models::ApiEnvelope<Value>>) {
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "approval_store_failed",
        format!("HiveCore could not {operation} approval state: {error}"),
    )
}

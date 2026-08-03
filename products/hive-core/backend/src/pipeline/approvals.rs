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
        ApprovalOrigin::WorkItem { .. } | ApprovalOrigin::OperatorDispatch => None,
    };
    let work_item_id = match &approval.subject.origin {
        ApprovalOrigin::WorkItem { work_item_id } => Some(work_item_id.clone()),
        ApprovalOrigin::SuiteRun { .. } | ApprovalOrigin::OperatorDispatch => None,
    };
    let work_resources = if let Some(work_item_id) = &work_item_id {
        let item = db::work_item_for_approval(&id)
            .map_err(|error| approval_store_error("read work item for", error))?
            .ok_or_else(|| {
                api_error(
                    StatusCode::CONFLICT,
                    "approval_work_item_unavailable",
                    format!("Work item {work_item_id} is no longer waiting for this approval."),
                )
            })?;
        let github_rate =
            super::governance::observe_github_rate(&state, crate::models::now_rfc3339()).await;
        match db::claim_work_resources(&item, github_rate, 100, 900)
            .map_err(|error| approval_store_error("claim resources for", error))?
        {
            Ok(resources) => Some(resources),
            Err((admission, evidence)) => {
                return Err(api_error(
                    StatusCode::TOO_MANY_REQUESTS,
                    "work_resource_admission_denied",
                    format!(
                        "The approval remains granted, but current resource evidence denied dispatch: {}",
                        serde_json::to_string(&serde_json::json!({"admission": admission, "evidence": evidence}))
                            .unwrap_or_else(|_| "resource evidence could not be encoded".into())
                    ),
                ));
            }
        }
    } else {
        None
    };
    let heartbeat_stop = work_resources.as_ref().map(|resources| {
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let resources = resources.clone();
        let approval_id = id.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut stopped => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {
                        match db::renew_work_resources(&resources, 900) {
                            Ok(true) => {}
                            Ok(false) => break,
                            Err(error) => {
                                tracing::error!(%approval_id, %error, "could not renew approved work resource lease");
                                break;
                            }
                        }
                    }
                }
            }
        });
        stop
    });
    let dispatch_result = dispatch_with_approval(
        &state,
        &approval.subject.product,
        &approval.subject.action_id,
        body,
        approval.subject.origin.clone(),
        Some(&id),
    )
    .await;
    if let Some(stop) = heartbeat_stop {
        let _ = stop.send(());
    }
    if let Some(resources) = &work_resources {
        let actual = match dispatch_result.as_ref() {
            Ok(DispatchActionResponse::Dispatched { event, .. })
                if event.status == "dispatched" =>
            {
                let normalized = crate::work_engine::normalized_response(&event.response_json);
                crate::work_engine::response_cost_cents(&normalized).or(Some(100))
            }
            Ok(DispatchActionResponse::Dispatched { .. })
            | Ok(DispatchActionResponse::ApprovalRequired { .. })
            | Err(_) => None,
        };
        if let Err(error) = db::settle_work_resources(
            resources,
            actual,
            if actual.is_some() {
                "Approved work dispatch settled"
            } else {
                "Approved work dispatch failed"
            },
        ) {
            tracing::error!(approval_id = %id, %error, "could not settle approved work resources");
        }
    }
    let response = dispatch_result?;
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
    if let (Some(work_item_id), DispatchActionResponse::Dispatched { event, .. }) =
        (&work_item_id, &response)
    {
        let lifecycle = if event.status == "dispatched" {
            let normalized = crate::work_engine::normalized_response(&event.response_json);
            crate::work_engine::lifecycle_after_dispatch(
                &approval.subject.action_id,
                &event.id,
                &normalized,
            )
        } else {
            crate::conductor::WorkLifecycle::Failed {
                reason: event.error.clone(),
                failed_at: now_rfc3339(),
                retryable: false,
                next_attempt_at: None,
            }
        };
        db::settle_work_approval(
            &id,
            lifecycle,
            "approved_dispatch_settled",
            &serde_json::json!({"work_item_id": work_item_id, "event": event}),
        )
        .map_err(|error| approval_store_error("settle work item after", error))?;
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

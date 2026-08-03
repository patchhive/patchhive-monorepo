use axum::{extract::Path, http::StatusCode, Json};
use serde_json::Value;

use crate::{
    conductor::{ProposeWorkOutcome, ProposeWorkRequest, WorkItem, WorkProposal},
    db,
    models::{ok, ApiEnvelope},
};

use super::types::api_error;

pub async fn propose_work(
    Json(request): Json<ProposeWorkRequest>,
) -> Result<Json<ApiEnvelope<ProposeWorkOutcome>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    let proposal = WorkProposal::from_request(request)
        .map_err(|message| api_error(StatusCode::BAD_REQUEST, "invalid_work_proposal", &message))?;
    db::propose_work(proposal)
        .map(|outcome| Json(ok(outcome)))
        .map_err(|error| {
            tracing::error!(%error, "could not persist proposed work");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "work_ledger_unavailable",
                "HiveCore could not persist the work proposal.",
            )
        })
}

pub async fn list_work_items(
) -> Result<Json<ApiEnvelope<Vec<WorkItem>>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    db::work_items(200)
        .map(|items| Json(ok(items)))
        .map_err(|error| {
            tracing::error!(%error, "could not read work ledger");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "work_ledger_unavailable",
                "HiveCore could not read the work ledger.",
            )
        })
}

pub async fn work_item_detail(
    Path(id): Path<String>,
) -> Result<Json<ApiEnvelope<WorkItem>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    match db::work_item(&id) {
        Ok(Some(item)) => Ok(Json(ok(item))),
        Ok(None) => Err(api_error(
            StatusCode::NOT_FOUND,
            "work_item_not_found",
            "Work item was not found.",
        )),
        Err(error) => {
            tracing::error!(work_item_id = %id, %error, "could not read work item");
            Err(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "work_ledger_unavailable",
                "HiveCore could not read the work item.",
            ))
        }
    }
}

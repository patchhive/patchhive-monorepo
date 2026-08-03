use axum::{extract::Path, http::StatusCode, Json};
use serde_json::Value;

use crate::{
    conductor::{
        FindingReceipt, IngestFindingsOutcome, IngestFindingsRequest, ProposeWorkOutcome,
        ProposeWorkRequest, SuiteLedgerEvent, WorkHandoffEdge, WorkItem, WorkProposal,
    },
    db::{self, FindingIngestionError},
    models::{ok, ApiEnvelope},
    state::product_catalog,
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

pub async fn ingest_findings(
    Json(request): Json<IngestFindingsRequest>,
) -> Result<Json<ApiEnvelope<IngestFindingsOutcome>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    let findings = request.validated().map_err(|message| {
        api_error(
            StatusCode::BAD_REQUEST,
            "invalid_finding_ingestion",
            &message,
        )
    })?;
    for finding in &findings {
        if !product_catalog()
            .iter()
            .any(|product| product.slug == finding.source.product_slug)
        {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "unknown_finding_product",
                format!(
                    "{} is not a registered PatchHive product.",
                    finding.source.product_slug
                ),
            ));
        }
    }
    db::ingest_findings(findings)
        .map(|outcome| Json(ok(outcome)))
        .map_err(finding_ingestion_error)
}

pub async fn list_finding_receipts(
) -> Result<Json<ApiEnvelope<Vec<FindingReceipt>>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    db::finding_receipts(200)
        .map(|receipts| Json(ok(receipts)))
        .map_err(|error| {
            tracing::error!(%error, "could not read product finding receipts");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "finding_receipts_unavailable",
                "HiveCore could not read product finding receipts.",
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

pub async fn live_blast_radius(
    Path(slug): Path<String>,
) -> Result<Json<ApiEnvelope<Vec<WorkHandoffEdge>>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    if !product_catalog().iter().any(|product| product.slug == slug) {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            "unknown_product",
            "Unknown product.",
        ));
    }
    db::work_handoff_edges()
        .map(|edges| {
            Json(ok(edges
                .into_iter()
                .filter(|edge| edge.from_product == slug || edge.to_product == slug)
                .collect()))
        })
        .map_err(|error| {
            tracing::error!(%error, "could not derive live blast radius");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "blast_radius_unavailable",
                "HiveCore could not derive blast radius from the work ledger.",
            )
        })
}

pub async fn list_suite_ledger(
) -> Result<Json<ApiEnvelope<Vec<SuiteLedgerEvent>>>, (StatusCode, Json<ApiEnvelope<Value>>)> {
    db::suite_ledger_events(200)
        .map(|events| Json(ok(events)))
        .map_err(|error| {
            tracing::error!(%error, "could not read unified suite ledger");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "suite_ledger_unavailable",
                "HiveCore could not read the unified work and outcome ledger.",
            )
        })
}

fn finding_ingestion_error(error: FindingIngestionError) -> (StatusCode, Json<ApiEnvelope<Value>>) {
    match error {
        FindingIngestionError::UnknownMandate(id) => api_error(
            StatusCode::BAD_REQUEST,
            "finding_mandate_not_found",
            format!("Mandate {id} was not found."),
        ),
        FindingIngestionError::SourceConflict(source) => api_error(
            StatusCode::CONFLICT,
            "finding_source_conflict",
            format!("Finding source {source} was already ingested with a different work identity."),
        ),
        FindingIngestionError::Storage(error) => {
            tracing::error!(%error, "could not ingest product findings");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "finding_ingestion_unavailable",
                "HiveCore could not ingest product findings.",
            )
        }
    }
}

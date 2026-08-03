use axum::{extract::Path, http::StatusCode, Json};
use serde_json::Value;

use crate::{
    conductor::{
        ConductorTickRecord, ConductorTickTrigger, MandateConfig, MandateReasonRequest,
        MandateRecord, RunConductorTickOutcome, SaveMandateRequest, UpdateMandateRequest,
    },
    db::{self, MandateWriteError},
    models::{ok, ApiEnvelope},
};

use super::types::api_error;

type ApiFailure = (StatusCode, Json<ApiEnvelope<Value>>);

pub async fn list_mandates() -> Result<Json<ApiEnvelope<Vec<MandateRecord>>>, ApiFailure> {
    db::mandates(200)
        .map(|items| Json(ok(items)))
        .map_err(storage_error)
}

pub async fn mandate_detail(
    Path(id): Path<String>,
) -> Result<Json<ApiEnvelope<MandateRecord>>, ApiFailure> {
    match db::mandate(&id) {
        Ok(Some(mandate)) => Ok(Json(ok(mandate))),
        Ok(None) => Err(api_error(
            StatusCode::NOT_FOUND,
            "mandate_not_found",
            "Mandate was not found.",
        )),
        Err(error) => Err(storage_error(error)),
    }
}

pub async fn create_mandate(
    Json(request): Json<SaveMandateRequest>,
) -> Result<Json<ApiEnvelope<MandateRecord>>, ApiFailure> {
    let config = MandateConfig::from_request(request).map_err(validation_error)?;
    db::create_mandate(config)
        .map(|mandate| Json(ok(mandate)))
        .map_err(write_error)
}

pub async fn update_mandate(
    Path(id): Path<String>,
    Json(request): Json<UpdateMandateRequest>,
) -> Result<Json<ApiEnvelope<MandateRecord>>, ApiFailure> {
    let config = MandateConfig::from_request(request.mandate).map_err(validation_error)?;
    db::update_mandate(&id, request.expected_revision, config)
        .map(|mandate| Json(ok(mandate)))
        .map_err(write_error)
}

pub async fn activate_mandate(
    Path(id): Path<String>,
) -> Result<Json<ApiEnvelope<MandateRecord>>, ApiFailure> {
    db::activate_mandate(&id)
        .map(|mandate| Json(ok(mandate)))
        .map_err(write_error)
}

pub async fn pause_mandate(
    Path(id): Path<String>,
    Json(request): Json<MandateReasonRequest>,
) -> Result<Json<ApiEnvelope<MandateRecord>>, ApiFailure> {
    let reason = request.validated().map_err(validation_error)?;
    db::pause_mandate(&id, reason)
        .map(|mandate| Json(ok(mandate)))
        .map_err(write_error)
}

pub async fn archive_mandate(
    Path(id): Path<String>,
    Json(request): Json<MandateReasonRequest>,
) -> Result<Json<ApiEnvelope<MandateRecord>>, ApiFailure> {
    let reason = request.validated().map_err(validation_error)?;
    db::archive_mandate(&id, reason)
        .map(|mandate| Json(ok(mandate)))
        .map_err(write_error)
}

pub async fn run_conductor_tick() -> Result<Json<ApiEnvelope<RunConductorTickOutcome>>, ApiFailure>
{
    db::run_conductor_tick(ConductorTickTrigger::Operator)
        .map(|outcome| Json(ok(outcome)))
        .map_err(storage_error)
}

pub async fn list_conductor_ticks(
) -> Result<Json<ApiEnvelope<Vec<ConductorTickRecord>>>, ApiFailure> {
    db::conductor_ticks(100)
        .map(|ticks| Json(ok(ticks)))
        .map_err(storage_error)
}

fn validation_error(message: String) -> ApiFailure {
    api_error(StatusCode::BAD_REQUEST, "invalid_mandate", &message)
}

fn write_error(error: MandateWriteError) -> ApiFailure {
    match error {
        MandateWriteError::NotFound => api_error(
            StatusCode::NOT_FOUND,
            "mandate_not_found",
            "Mandate was not found.",
        ),
        MandateWriteError::RevisionConflict => api_error(
            StatusCode::CONFLICT,
            "mandate_revision_conflict",
            "The mandate changed since it was loaded. Refresh before saving.",
        ),
        MandateWriteError::DuplicateName => api_error(
            StatusCode::CONFLICT,
            "mandate_name_conflict",
            "A mandate with that name already exists.",
        ),
        MandateWriteError::InvalidLifecycle => api_error(
            StatusCode::CONFLICT,
            "mandate_lifecycle_conflict",
            "The mandate lifecycle does not allow that change.",
        ),
        MandateWriteError::Storage(error) => storage_error(error),
    }
}

fn storage_error(error: rusqlite::Error) -> ApiFailure {
    tracing::error!(%error, "HiveCore mandate/conductor persistence failed");
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "conductor_storage_unavailable",
        "HiveCore could not read or update conductor state.",
    )
}

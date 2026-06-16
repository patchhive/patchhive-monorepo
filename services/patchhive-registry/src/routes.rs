use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use crate::{
    models::{
        ErrorResponse, HealthResponse, OkResponse, RegisterInstallRequest, RegisterInstallResponse,
        RegistrySnapshot, SmokeUpdateRequest,
    },
    state::AppState,
};

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<Json<T>, ApiError>;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/v1/installs/register", post(register_install))
        .route("/v1/installs/:install_id/heartbeat", post(heartbeat))
        .route("/v1/installs/:install_id/smoke", post(smoke))
        .route("/v1/public/installs", get(public_installs))
        .route("/v1/public/installs/:public_slug", get(public_snapshot))
}

async fn root(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(health_payload(&state))
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(health_payload(&state))
}

async fn register_install(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterInstallRequest>,
) -> Result<(StatusCode, Json<RegisterInstallResponse>), ApiError> {
    let response = state
        .store
        .register_install(request)
        .map_err(internal_error)?;
    Ok((StatusCode::CREATED, Json(response)))
}

async fn heartbeat(
    State(state): State<Arc<AppState>>,
    Path(install_id): Path<String>,
    headers: HeaderMap,
    Json(snapshot): Json<RegistrySnapshot>,
) -> ApiResult<OkResponse> {
    require_token(&state, &install_id, &headers)?;
    state
        .store
        .save_heartbeat(&install_id, snapshot)
        .map_err(bad_request)?;
    Ok(Json(OkResponse { ok: true }))
}

async fn smoke(
    State(state): State<Arc<AppState>>,
    Path(install_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<SmokeUpdateRequest>,
) -> ApiResult<OkResponse> {
    require_token(&state, &install_id, &headers)?;
    state
        .store
        .save_smoke(&install_id, request)
        .map_err(bad_request)?;
    Ok(Json(OkResponse { ok: true }))
}

async fn public_installs(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Vec<crate::models::PublicInstallSummary>> {
    Ok(Json(state.store.public_installs().map_err(internal_error)?))
}

async fn public_snapshot(
    State(state): State<Arc<AppState>>,
    Path(public_slug): Path<String>,
) -> ApiResult<RegistrySnapshot> {
    match state
        .store
        .public_snapshot(&public_slug)
        .map_err(internal_error)?
    {
        Some(snapshot) => Ok(Json(snapshot)),
        None => Err(not_found("No public registry install found for that slug.")),
    }
}

fn health_payload(state: &AppState) -> HealthResponse {
    HealthResponse {
        service: "patchhive-registry",
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        db_ok: state.store.health_check(),
    }
}

fn require_token(state: &AppState, install_id: &str, headers: &HeaderMap) -> Result<(), ApiError> {
    let token = registry_token(headers).ok_or_else(|| unauthorized("Missing registry token."))?;
    match state.store.authorize(install_id, &token) {
        Ok(true) => Ok(()),
        Ok(false) => Err(unauthorized("Invalid registry token.")),
        Err(err) => Err(internal_error(err)),
    }
}

fn registry_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-patchhive-registry-token")
        .or_else(|| headers.get(AUTHORIZATION))
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim_start_matches("Bearer ").trim().to_string())
        .filter(|value| !value.is_empty())
}

fn bad_request(err: anyhow::Error) -> ApiError {
    error(StatusCode::BAD_REQUEST, "bad-request", err.to_string())
}

fn internal_error(err: anyhow::Error) -> ApiError {
    tracing::error!(error = %err, "registry request failed");
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal-error",
        "Registry request failed.".to_string(),
    )
}

fn unauthorized(message: impl Into<String>) -> ApiError {
    error(StatusCode::UNAUTHORIZED, "unauthorized", message.into())
}

fn not_found(message: impl Into<String>) -> ApiError {
    error(StatusCode::NOT_FOUND, "not-found", message.into())
}

fn error(status: StatusCode, code: &'static str, message: String) -> ApiError {
    (
        status,
        Json(ErrorResponse {
            error: code,
            message,
        }),
    )
}

#[allow(dead_code)]
fn into_response<T: serde::Serialize>(result: ApiResult<T>) -> axum::response::Response {
    match result {
        Ok(json) => json.into_response(),
        Err(err) => err.into_response(),
    }
}

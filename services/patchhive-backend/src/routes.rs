use std::sync::Arc;

use axum::{
    body::Body,
    extract::{ConnectInfo, Path, State},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, get, post},
    Json, Router,
};

use crate::{
    gateway,
    models::{
        AuthStatusResponse, ErrorResponse, HealthResponse, ProductResponse, SessionResponse,
        SetupResponse,
    },
    products,
    state::AppState,
};
use std::net::SocketAddr;

pub fn router(state: Arc<AppState>) -> Router {
    let suite_routes = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/api/health", get(health))
        .route("/api/auth/status", get(auth_status))
        .route("/api/auth/session", get(session))
        .route("/api/products", get(products))
        .route("/api/products/:product_key/health", get(product_health))
        .route(
            "/api/products/:product_key/*gateway_path",
            any(product_gateway),
        )
        .route("/api/setup/first-stack", get(first_stack_status))
        .route("/api/setup/first-stack/pair", post(pair_first_stack))
        .route("/api/runs", get(runs))
        .route("/api/events", get(events))
        .with_state(state);

    Router::new()
        .nest(
            "/api/products/merge-keeper",
            products::merge_keeper_router(),
        )
        .nest(
            "/api/products/release-sentry",
            products::release_sentry_router(),
        )
        .nest("/api/products/dep-triage", products::dep_triage_router())
        .nest("/api/products/vuln-triage", products::vuln_triage_router())
        .merge(suite_routes)
}

async fn root() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "patchhive-backend",
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        mode: "unknown",
        enabled_products: 0,
        db_ok: true,
        product_override_count: 0,
    })
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "patchhive-backend",
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        mode: state.config.product_selection.mode_label(),
        enabled_products: state.enabled_product_count(),
        db_ok: state.db_ok(),
        product_override_count: state.product_override_count(),
    })
}

async fn auth_status() -> Json<AuthStatusResponse> {
    Json(AuthStatusResponse {
        auth_enabled: false,
        bootstrap_required: false,
        service_auth_enabled: false,
        suite_bootstrap_enabled: false,
    })
}

async fn session(State(state): State<Arc<AppState>>) -> Json<SessionResponse> {
    Json(SessionResponse {
        service: "patchhive-backend",
        authenticated: true,
        auth_configured: false,
        mode: state.config.product_selection.mode_label(),
        enabled_products: state.enabled_product_count(),
    })
}

async fn products(State(state): State<Arc<AppState>>) -> Json<Vec<ProductResponse>> {
    Json(
        state
            .registry
            .products()
            .iter()
            .map(|product| product.to_response(state.product_enabled(product.key.as_str())))
            .collect(),
    )
}

async fn product_health(
    State(state): State<Arc<AppState>>,
    Path(product_key): Path<String>,
    peer: Option<ConnectInfo<SocketAddr>>,
    request: Request<Body>,
) -> Response {
    match state.registry.find(&product_key) {
        Some(product) if product.gateway_target_url().is_some() => {
            gateway::proxy_product_request(
                state,
                product_key,
                request,
                peer.map(|ConnectInfo(addr)| addr),
            )
            .await
        }
        Some(product) => {
            Json(product.to_health_response(state.product_enabled(product.key.as_str())))
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "unknown-product",
                message: format!("No PatchHive product is registered with key `{product_key}`."),
            }),
        )
            .into_response(),
    }
}

async fn product_gateway(
    State(state): State<Arc<AppState>>,
    Path((product_key, _gateway_path)): Path<(String, String)>,
    peer: Option<ConnectInfo<SocketAddr>>,
    request: Request<Body>,
) -> Response {
    gateway::proxy_product_request(
        state,
        product_key,
        request,
        peer.map(|ConnectInfo(addr)| addr),
    )
    .await
}

async fn first_stack_status(State(state): State<Arc<AppState>>) -> Json<SetupResponse> {
    Json(state.first_stack_status(Vec::new()))
}

async fn pair_first_stack(State(state): State<Arc<AppState>>) -> Json<SetupResponse> {
    Json(state.first_stack_status(vec![
        "Unified backend is connected to HiveCore. Gateway pairing is not implemented yet."
            .to_string(),
    ]))
}

async fn runs(State(state): State<Arc<AppState>>) -> Json<Vec<crate::models::RunSummary>> {
    Json(state.runs())
}

async fn events(State(state): State<Arc<AppState>>) -> Json<Vec<crate::models::SuiteEvent>> {
    Json(state.events())
}

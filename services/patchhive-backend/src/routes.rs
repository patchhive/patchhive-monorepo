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
        .route("/api/auth/login", post(login))
        .route("/api/auth/generate-key", post(generate_key))
        .route("/api/auth/session", get(session))
        .route("/api/products", get(products))
        .route("/api/products/auth-status", get(products_auth_status))
        .route(
            "/api/products/:product_key/service-token",
            post(provision_service_token),
        )
        .route("/api/products/:product_key/health", get(product_health))
        .route(
            "/api/products/:product_key/*gateway_path",
            any(product_gateway),
        )
        .route("/api/setup/first-stack", get(first_stack_status))
        .route("/api/setup/first-stack/pair", post(pair_first_stack))
        .route("/api/runs", get(runs))
        .route("/api/products/runs", get(products_runs))
        .route("/api/products/capabilities", get(products_capabilities))
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
        .nest("/api/products/flake-sting", products::flake_sting_router())
        .nest("/api/products/review-bee", products::review_bee_router())
        .nest("/api/products/trust-gate", products::trust_gate_router())
        .nest("/api/products/repo-memory", products::repo_memory_router())
        .nest("/api/products/signal-hive", products::signal_hive_router())
        .nest(
            "/api/products/refactor-scout",
            products::refactor_scout_router(),
        )
        .nest("/api/products/repo-reaper", products::repo_reaper_router())
        .nest("/api/products/hive-core", products::hive_core_router())
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
    let enabled = crate::auth::auth_enabled();
    Json(AuthStatusResponse {
        auth_enabled: enabled,
        // Nothing is protected until a key exists, so an unconfigured suite tells
        // the operator to bootstrap rather than silently running open.
        bootstrap_required: !enabled,
        service_auth_enabled: false,
        suite_bootstrap_enabled: false,
    })
}

#[derive(serde::Deserialize)]
struct LoginRequest {
    api_key: String,
}

async fn login(Json(body): Json<LoginRequest>) -> Response {
    if !crate::auth::auth_enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "auth-not-configured",
                message: "No suite API key is configured. Generate one from localhost first."
                    .into(),
            }),
        )
            .into_response();
    }
    if !crate::auth::verify_token(&body.api_key) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid-key",
                message: "That key is not valid for this suite.".into(),
            }),
        )
            .into_response();
    }
    Json(serde_json::json!({ "ok": true, "auth_enabled": true })).into_response()
}

/// First-key bootstrap. Localhost-only unless PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP is
/// set, and refuses once a key already exists so it cannot be used to reset auth.
async fn generate_key(
    headers: axum::http::HeaderMap,
    peer: Option<ConnectInfo<SocketAddr>>,
) -> Response {
    if crate::auth::auth_enabled() {
        return (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "auth-already-configured",
                message: "A suite API key already exists. Rotate it by editing PATCHHIVE_SUITE_API_KEY_HASH."
                    .into(),
            }),
        )
            .into_response();
    }

    let peer_addr = peer.map(|ConnectInfo(addr)| addr);
    if !crate::auth::bootstrap_request_allowed_from_peer(&headers, peer_addr) {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "bootstrap-local-only",
                message: "First-key generation is localhost-only. Set PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP=true to override."
                    .into(),
            }),
        )
            .into_response();
    }

    match crate::auth::generate_and_save_key() {
        Ok(key) => Json(serde_json::json!({
            "api_key": key,
            "message": "Store this — it will not be shown again."
        }))
        .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "key-generation-failed",
                message: format!("Could not save the suite API key: {error}"),
            }),
        )
            .into_response(),
    }
}

async fn session(State(state): State<Arc<AppState>>) -> Json<SessionResponse> {
    let configured = crate::auth::auth_enabled();
    Json(SessionResponse {
        service: "patchhive-backend",
        // This route is behind the auth middleware, so reaching it means the request
        // was authenticated — or that no key is configured and nothing is enforced.
        // Reporting `true` unconditionally, as this did, told the deck it had a
        // session on a suite with no auth at all.
        authenticated: configured,
        auth_configured: configured,
        mode: state.config.product_selection.mode_label(),
        enabled_products: state.enabled_product_count(),
    })
}

/// Loopback guard for the in-process aggregates.
///
/// These read data each product protects behind its own auth middleware — run
/// history, auth posture, advertised capabilities. Calling the handlers directly
/// bypasses that middleware, which is correct for a same-process read but must not
/// become a remotely readable hole if the backend is bound beyond localhost.
///
/// With suite auth configured, the middleware has already verified the operator key
/// before a request reaches here, so this adds nothing and defers.
///
/// With auth unconfigured the suite API is open, and loopback is the only boundary
/// left — so the aggregates stay local-only until a key exists. That makes the
/// unconfigured state safe by default rather than quietly readable.
fn aggregate_access_allowed(peer: Option<SocketAddr>) -> bool {
    if crate::auth::auth_enabled() {
        return true;
    }
    if std::env::var("PATCHHIVE_ALLOW_REMOTE_AGGREGATES")
        .map(|value| value.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return true;
    }
    match peer {
        Some(addr) => addr.ip().is_loopback(),
        // No peer information: refuse rather than assume local.
        None => false,
    }
}

fn aggregate_forbidden() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "aggregate-local-only",
            message: "Suite aggregates read product-protected data. Configure a suite API key \
(POST /api/auth/generate-key from localhost), or set PATCHHIVE_ALLOW_REMOTE_AGGREGATES=true \
only behind your own authenticated proxy."
                .into(),
        }),
    )
        .into_response()
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

/// Server-side aggregate of every mounted engine's auth posture.
async fn products_auth_status(
    State(state): State<Arc<AppState>>,
    peer: Option<ConnectInfo<SocketAddr>>,
) -> Response {
    if !aggregate_access_allowed(peer.map(|ConnectInfo(addr)| addr)) {
        return aggregate_forbidden();
    }
    Json(products::auth_statuses(&state.config)).into_response()
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct ProvisionRequest {
    rotate: bool,
}

/// Mint or rotate a product's service token. Returns the resulting posture only —
/// the token is written to the product's own storage and never sent to the browser.
async fn provision_service_token(
    State(state): State<Arc<AppState>>,
    Path(product_key): Path<String>,
    peer: Option<ConnectInfo<SocketAddr>>,
    headers: axum::http::HeaderMap,
    body: Option<Json<ProvisionRequest>>,
) -> Response {
    let rotate = body.map(|Json(request)| request.rotate).unwrap_or(false);
    let peer_addr = peer.map(|ConnectInfo(addr)| addr);

    match products::provision_service_token(&state.config, &product_key, &headers, peer_addr, rotate)
    {
        products::ProvisionOutcome::Provisioned(status) => Json(serde_json::json!({
            "key": product_key,
            "provisioned": true,
            "status": status,
        }))
        .into_response(),
        products::ProvisionOutcome::Forbidden => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "provisioning-forbidden",
                message: format!(
                    "{product_key} refused this caller. Provisioning is localhost-only unless an operator key or the suite bootstrap secret is supplied."
                ),
            }),
        )
            .into_response(),
        products::ProvisionOutcome::NotEnabled => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "product-not-enabled",
                message: format!("`{product_key}` is not enabled in this runtime."),
            }),
        )
            .into_response(),
        products::ProvisionOutcome::Unknown => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "unknown-product",
                message: format!("No PatchHive product is registered with key `{product_key}`."),
            }),
        )
            .into_response(),
        products::ProvisionOutcome::Failed(message) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "provisioning-failed",
                message: format!("Could not provision a service token for {product_key}: {message}"),
            }),
        )
            .into_response(),
    }
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

/// Server-side aggregate of every mounted engine's run history.
async fn products_runs(
    State(state): State<Arc<AppState>>,
    peer: Option<ConnectInfo<SocketAddr>>,
) -> Response {
    if !aggregate_access_allowed(peer.map(|ConnectInfo(addr)| addr)) {
        return aggregate_forbidden();
    }
    Json(products::all_runs(&state.config).await).into_response()
}

/// Runtime-advertised capabilities per engine, for drift comparison against the
/// manifest data already served by /api/products.
async fn products_capabilities(
    State(state): State<Arc<AppState>>,
    peer: Option<ConnectInfo<SocketAddr>>,
) -> Response {
    if !aggregate_access_allowed(peer.map(|ConnectInfo(addr)| addr)) {
        return aggregate_forbidden();
    }
    Json(products::advertised_capabilities(&state.config).await).into_response()
}

async fn events(State(state): State<Arc<AppState>>) -> Json<Vec<crate::models::SuiteEvent>> {
    Json(state.events())
}

#[cfg(test)]
mod tests {
    use super::router;
    use crate::{
        config::{Config, ProductSelection},
        state::AppState,
    };
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        Router,
    };
    use serde_json::Value;
    use std::{net::SocketAddr, path::PathBuf, sync::Arc};
    use tower::ServiceExt;

    fn test_app() -> (Router, PathBuf) {
        let db_path = std::env::temp_dir().join(format!(
            "patchhive-backend-contract-{}-{}.db",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let config = Config {
            bind_addr: "127.0.0.1:0".parse::<SocketAddr>().expect("test bind addr"),
            db_path: db_path.clone(),
            product_selection: ProductSelection::All,
        };
        let state = Arc::new(AppState::new(config).expect("test app state"));
        (router(state), db_path)
    }

    async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status();
        let body = to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .expect("response body");
        let value = serde_json::from_slice(&body).expect("JSON response");
        (status, value)
    }

    #[tokio::test]
    async fn suite_contract_endpoints_return_stable_json_shapes() {
        let (app, db_path) = test_app();
        for uri in [
            "/api/health",
            "/api/auth/status",
            "/api/products",
            "/api/setup/first-stack",
            "/api/runs",
            "/api/events",
        ] {
            let (status, body) = get_json(&app, uri).await;
            assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        }

        let (_, health) = get_json(&app, "/api/health").await;
        assert_eq!(health["service"], "patchhive-backend");
        assert_eq!(health["status"], "ok");
        assert_eq!(health["enabled_products"], 12);
        drop(app);
        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn registry_and_mounted_routers_agree_on_integrated_products() {
        let (app, db_path) = test_app();
        let (_, products) = get_json(&app, "/api/products").await;
        let products = products.as_array().expect("product list");
        let integrated = [
            "signal-hive",
            "merge-keeper",
            "release-sentry",
            "dep-triage",
            "vuln-triage",
            "flake-sting",
            "review-bee",
            "trust-gate",
            "repo-memory",
            "refactor-scout",
            "repo-reaper",
        ];

        for key in integrated {
            let product = products
                .iter()
                .find(|product| product["key"] == key)
                .unwrap_or_else(|| panic!("missing registry entry for {key}"));
            assert_eq!(product["migration_stage"], "integrated");
            assert_eq!(product["status"], "online");
            assert_eq!(product["route_prefix"], format!("/api/products/{key}"));

            let (status, capabilities) =
                get_json(&app, &format!("/api/products/{key}/capabilities")).await;
            assert_eq!(status, StatusCode::OK, "{key}: {capabilities}");
            assert_eq!(capabilities["product_slug"], key);
            assert_eq!(
                capabilities["schema_version"],
                "patchhive.product.contract.v1"
            );
            assert!(
                capabilities["operating_modes"]["triggers"]
                    .as_array()
                    .is_some_and(|modes| !modes.is_empty()),
                "{key} must advertise at least one run trigger"
            );
            assert!(
                capabilities["operating_modes"]["target_selection"]
                    .as_array()
                    .is_some_and(|modes| !modes.is_empty()),
                "{key} must advertise at least one target-selection mode"
            );
            if key == "signal-hive" {
                let target_modes = capabilities["operating_modes"]["target_selection"]
                    .as_array()
                    .expect("SignalHive target-selection modes");
                assert!(target_modes.iter().any(|mode| mode == "direct"));
                assert!(target_modes.iter().any(|mode| mode == "discovery"));
            }
        }
        drop(app);
        let _ = std::fs::remove_file(db_path);
    }

    #[tokio::test]
    async fn unknown_product_uses_the_suite_error_shape() {
        let (app, db_path) = test_app();
        let (status, body) = get_json(&app, "/api/products/not-a-product/health").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "unknown-product");
        assert!(body["message"]
            .as_str()
            .expect("error message")
            .contains("not-a-product"));
        drop(app);
        let _ = std::fs::remove_file(db_path);
    }
}

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
        .nest("/api/products/flake-sting", products::flake_sting_router())
        .nest("/api/products/review-bee", products::review_bee_router())
        .nest("/api/products/trust-gate", products::trust_gate_router())
        .nest("/api/products/repo-memory", products::repo_memory_router())
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
            "merge-keeper",
            "release-sentry",
            "dep-triage",
            "vuln-triage",
            "flake-sting",
            "review-bee",
            "trust-gate",
            "repo-memory",
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

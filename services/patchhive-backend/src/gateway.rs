use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{HeaderName, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::{models::ErrorResponse, registry::ProductManifest, state::AppState};

const MAX_GATEWAY_BODY_BYTES: usize = 25 * 1024 * 1024;

pub async fn proxy_product_request(
    state: Arc<AppState>,
    product_key: String,
    request: Request<Body>,
    peer_addr: Option<SocketAddr>,
) -> Response {
    let Some(product) = state.registry.find(&product_key) else {
        return json_error(
            StatusCode::NOT_FOUND,
            "unknown-product",
            format!("No PatchHive product is registered with key `{product_key}`."),
        );
    };

    if !state.product_enabled(product.key.as_str()) {
        return json_error(
            StatusCode::FORBIDDEN,
            "product-disabled",
            format!("{} is disabled by PATCHHIVE_PRODUCTS.", product.name),
        );
    }

    let Some(target_url) = product.gateway_target_url() else {
        return json_error(
            StatusCode::BAD_GATEWAY,
            "gateway-unconfigured",
            format!(
                "{} does not have a gateway target configured.",
                product.name
            ),
        );
    };

    let (parts, body) = request.into_parts();
    let method = parts.method.clone();
    let path = parts.uri.path().to_string();

    if product
        .route_claim_for(method.as_str(), path.as_str())
        .is_none()
    {
        return json_error(
            StatusCode::NOT_FOUND,
            "route-not-claimed",
            format!(
                "{} does not claim gateway route `{}` {}.",
                product.name, method, path
            ),
        );
    }

    if let Err(response) =
        check_gateway_health(&state, product, target_url.as_str(), path.as_str()).await
    {
        return response;
    }

    let Some(downstream_url) = downstream_url_for(
        product,
        target_url.as_str(),
        path.as_str(),
        parts.uri.query(),
    ) else {
        return json_error(
            StatusCode::BAD_GATEWAY,
            "gateway-prefix-mismatch",
            format!(
                "Route `{path}` does not live under {}.",
                product.route_prefix
            ),
        );
    };

    let body = match to_bytes(body, MAX_GATEWAY_BODY_BYTES).await {
        Ok(body) => body,
        Err(err) => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "gateway-body-read-failed",
                format!("Could not read request body for gateway dispatch: {err}"),
            );
        }
    };

    let reqwest_method = match reqwest::Method::from_bytes(method.as_str().as_bytes()) {
        Ok(method) => method,
        Err(err) => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "gateway-method-invalid",
                format!("Could not convert request method for gateway dispatch: {err}"),
            );
        }
    };

    let mut builder = state
        .http
        .request(reqwest_method, downstream_url)
        .body(body);
    for (name, value) in parts.headers.iter() {
        if should_skip_request_header(name) {
            continue;
        }
        builder = builder.header(name.as_str(), value.as_bytes());
    }
    if let Some(peer_addr) = peer_addr {
        builder = builder.header("x-forwarded-for", peer_addr.ip().to_string());
    }

    let downstream = match builder.send().await {
        Ok(response) => response,
        Err(err) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                "gateway-request-failed",
                format!(
                    "Could not reach {} gateway at {target_url}: {err}",
                    product.name
                ),
            );
        }
    };

    let status =
        StatusCode::from_u16(downstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let downstream_headers = downstream
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            if should_skip_response_header(name.as_str()) {
                return None;
            }
            let name = HeaderName::from_bytes(name.as_str().as_bytes()).ok()?;
            let value = HeaderValue::from_bytes(value.as_bytes()).ok()?;
            Some((name, value))
        })
        .collect::<Vec<_>>();
    let body = match downstream.bytes().await {
        Ok(body) => body,
        Err(err) => {
            return json_error(
                StatusCode::BAD_GATEWAY,
                "gateway-response-read-failed",
                format!("Could not read {} gateway response: {err}", product.name),
            );
        }
    };

    let mut response = Response::builder()
        .status(status)
        .body(Body::from(body))
        .unwrap_or_else(|_| {
            json_error(
                StatusCode::BAD_GATEWAY,
                "gateway-response-build-failed",
                "Could not build gateway response.".to_string(),
            )
        });
    for (name, value) in downstream_headers {
        response.headers_mut().insert(name, value);
    }
    response
}

async fn check_gateway_health(
    state: &AppState,
    product: &ProductManifest,
    target_url: &str,
    request_path: &str,
) -> Result<(), Response> {
    if product.health.endpoint.is_empty() || request_path == product.health.endpoint {
        return Ok(());
    }

    let Some(health_url) =
        downstream_url_for(product, target_url, product.health.endpoint.as_str(), None)
    else {
        return Err(json_error(
            StatusCode::BAD_GATEWAY,
            "gateway-health-prefix-mismatch",
            format!(
                "Health endpoint `{}` does not live under {}.",
                product.health.endpoint, product.route_prefix
            ),
        ));
    };

    let timeout = Duration::from_millis(product.health.timeout_ms.max(1));
    let response = match state.http.get(&health_url).timeout(timeout).send().await {
        Ok(response) => response,
        Err(err) => {
            return Err(json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "product-unavailable",
                format!(
                    "{} health check failed at {}: {err}",
                    product.name, product.health.endpoint
                ),
            ));
        }
    };

    let status = response.status().as_u16();
    if status == product.health.healthy_status {
        Ok(())
    } else {
        Err(json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "product-unavailable",
            format!(
                "{} health check returned {status}, expected {}.",
                product.name, product.health.healthy_status
            ),
        ))
    }
}

fn downstream_url_for(
    product: &ProductManifest,
    target_url: &str,
    path: &str,
    query: Option<&str>,
) -> Option<String> {
    let path_suffix = path.strip_prefix(product.route_prefix.as_str())?;
    let downstream_path = if path_suffix.is_empty() {
        "/"
    } else {
        path_suffix
    };
    let query = query.map(|query| format!("?{query}")).unwrap_or_default();
    Some(format!("{target_url}{downstream_path}{query}"))
}

fn json_error(status: StatusCode, error: &'static str, message: String) -> Response {
    (status, Json(ErrorResponse { error, message })).into_response()
}

fn should_skip_request_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str().to_ascii_lowercase().as_str(),
        "host"
            | "connection"
            | "content-length"
            | "transfer-encoding"
            | "upgrade"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "keep-alive"
            | "forwarded"
            | "x-forwarded-for"
            | "x-real-ip"
    )
}

fn should_skip_response_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "content-length"
            | "transfer-encoding"
            | "upgrade"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "keep-alive"
    )
}

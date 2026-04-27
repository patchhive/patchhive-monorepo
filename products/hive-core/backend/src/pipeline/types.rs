use std::collections::HashMap;
use std::time::Duration;

use axum::{
    http::StatusCode,
    Json,
};
use patchhive_product_core::auth::SERVICE_TOKEN_HEADER;
use patchhive_product_core::contract;
use reqwest::Url;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::models::{error, ProductContractCheck, ProductOverride};

pub type HiveApiError = (StatusCode, Json<crate::models::ApiEnvelope<Value>>);

pub fn api_error(
    status: StatusCode,
    code: impl Into<String>,
    message: impl Into<String>,
) -> HiveApiError {
    (status, Json(error(code, message, false)))
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub api_key: String,
}

#[derive(Deserialize)]
pub struct ProductHealthBody {
    pub status: Option<String>,
    pub version: Option<String>,
    pub config_errors: Option<u32>,
    pub db_ok: Option<bool>,
}

#[derive(Deserialize)]
pub struct StartupChecksBody {
    pub checks: Vec<patchhive_product_core::startup::StartupCheck>,
}

#[derive(Debug, Clone, Default)]
pub struct ProductStoredAuth {
    pub service_token: String,
    pub legacy_api_key: String,
}

impl ProductStoredAuth {
    pub fn from_override(override_item: Option<&ProductOverride>) -> Self {
        Self {
            service_token: override_item
                .map(|item| item.service_token.trim().to_string())
                .unwrap_or_default(),
            legacy_api_key: override_item
                .map(|item| item.legacy_api_key.trim().to_string())
                .unwrap_or_default(),
        }
    }

    pub fn service_token_configured(&self) -> bool {
        !self.service_token.is_empty()
    }

    pub fn legacy_api_key_configured(&self) -> bool {
        !self.legacy_api_key.is_empty()
    }

    pub fn machine_auth_configured(&self) -> bool {
        self.service_token_configured() || self.legacy_api_key_configured()
    }

    pub fn auth_mode(&self) -> &'static str {
        if self.service_token_configured() {
            "service_token"
        } else if self.legacy_api_key_configured() {
            "legacy_api_key"
        } else {
            "none"
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProductAuthStatusBody {
    #[serde(default)]
    pub auth_enabled: bool,
    #[serde(default)]
    pub service_auth_supported: bool,
    #[serde(default)]
    pub service_auth_enabled: bool,
    #[serde(default)]
    pub service_auth_scoped: bool,
    #[serde(default)]
    pub service_auth_legacy: bool,
    #[serde(default)]
    pub service_auth_scopes: Vec<String>,
    #[serde(default)]
    pub service_auth_expired: bool,
}

pub struct ProductProbeSnapshot {
    pub health: crate::models::ProductHealthSnapshot,
    pub hivecore: Option<patchhive_product_core::contract::HiveCoreLifecycleSupport>,
    pub actions: Vec<patchhive_product_core::contract::ProductAction>,
    pub links: Vec<patchhive_product_core::contract::ProductLink>,
    pub contract_checks: Vec<ProductContractCheck>,
    pub run_detail_template: String,
    pub recent_runs: Vec<patchhive_product_core::contract::ProductRunSummary>,
}

#[derive(Debug, Default)]
pub struct DispatchActionInput {
    pub payload: Value,
    pub path_params: HashMap<String, String>,
    pub query: HashMap<String, String>,
}

pub fn resolved_auth_mode(definition: &crate::state::ProductDefinition, auth: &ProductStoredAuth) -> String {
    if definition.slug == "hive-core" {
        "native".into()
    } else {
        auth.auth_mode().into()
    }
}

pub fn resolved_machine_auth_configured(
    definition: &crate::state::ProductDefinition,
    auth: &ProductStoredAuth,
) -> bool {
    definition.slug == "hive-core" || auth.machine_auth_configured()
}

pub fn resolved_service_token_configured(
    definition: &crate::state::ProductDefinition,
    auth: &ProductStoredAuth,
) -> bool {
    definition.slug != "hive-core" && auth.service_token_configured()
}

pub fn resolved_legacy_api_key_configured(
    definition: &crate::state::ProductDefinition,
    auth: &ProductStoredAuth,
) -> bool {
    definition.slug != "hive-core" && auth.legacy_api_key_configured()
}

pub fn pick_url(override_url: Option<&str>, default_url: &str) -> String {
    let override_url = override_url.unwrap_or("").trim();
    if override_url.is_empty() {
        default_url.to_string()
    } else {
        override_url.to_string()
    }
}

pub fn contract_check(
    id: impl Into<String>,
    label: impl Into<String>,
    path: impl Into<String>,
    ok: bool,
    status: impl Into<String>,
    error: impl Into<String>,
) -> ProductContractCheck {
    ProductContractCheck {
        id: id.into(),
        label: label.into(),
        path: path.into(),
        ok,
        status: status.into(),
        error: error.into(),
    }
}

pub fn contract_drift_count(checks: &[ProductContractCheck]) -> u32 {
    checks
        .iter()
        .filter(|check| {
            !check.ok
                && !matches!(
                    check.status.as_str(),
                    "locked" | "skipped" | "disabled" | "unconfigured"
                )
        })
        .count() as u32
}

pub fn contract_checks_for_unavailable_product(status: &str) -> Vec<ProductContractCheck> {
    let reason = if status == "disabled" {
        "Product is disabled in HiveCore settings."
    } else {
        "Product API URL is not configured."
    };
    [
        ("health", "Health", "/health"),
        ("startup_checks", "Startup checks", "/startup/checks"),
        ("capabilities", "Capabilities", "/capabilities"),
        ("runs", "Runs", "/runs"),
        ("run_detail", "Run detail", "/runs/{id}"),
    ]
    .into_iter()
    .map(|(id, label, path)| contract_check(id, label, path, false, status, reason))
    .collect()
}

pub fn contract_checks_for_failed_health() -> Vec<ProductContractCheck> {
    contract_checks_with_health_error("Could not reach /health.")
}

pub fn contract_checks_with_health_error(error: impl Into<String>) -> Vec<ProductContractCheck> {
    let error = error.into();
    vec![
        contract_check("health", "Health", "/health", false, "failed", error),
        contract_check(
            "startup_checks",
            "Startup checks",
            "/startup/checks",
            false,
            "skipped",
            "Health must pass before startup checks are meaningful.",
        ),
        contract_check(
            "capabilities",
            "Capabilities",
            "/capabilities",
            false,
            "skipped",
            "Health must pass before capabilities are meaningful.",
        ),
        contract_check(
            "runs",
            "Runs",
            "/runs",
            false,
            "skipped",
            "Health must pass before run history is meaningful.",
        ),
        contract_check(
            "run_detail",
            "Run detail",
            "/runs/{id}",
            false,
            "skipped",
            "Health must pass before run detail support is meaningful.",
        ),
    ]
}

pub fn authorized_request(
    request: reqwest::RequestBuilder,
    auth: &ProductStoredAuth,
) -> reqwest::RequestBuilder {
    if auth.service_token_configured() {
        request.header(SERVICE_TOKEN_HEADER, auth.service_token.trim())
    } else if auth.legacy_api_key_configured() {
        request.header("X-API-Key", auth.legacy_api_key.trim())
    } else {
        request
    }
}

pub fn authorized_get(
    client: &reqwest::Client,
    url: &str,
    auth: &ProductStoredAuth,
) -> reqwest::RequestBuilder {
    let request = client.get(url);
    authorized_request(request, auth)
}

pub fn build_target_url(
    api_url: &str,
    path: &str,
    query: &HashMap<String, String>,
) -> Result<Url, String> {
    let normalized = api_url.trim_end_matches('/');
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut url = Url::parse(&format!("{normalized}{path}"))
        .map_err(|err| format!("Could not build product action URL: {err}"))?;
    if !query.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query {
            pairs.append_pair(key, value);
        }
    }
    Ok(url)
}

pub fn parse_response_body(text: &str) -> Value {
    if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(text).unwrap_or_else(|_| json!({ "raw": text }))
    }
}

pub fn remote_error_message(value: &Value) -> Option<String> {
    value
        .get("error")
        .and_then(Value::as_str)
        .map(|message| message.to_string())
        .or_else(|| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .map(|message| message.to_string())
        })
}

pub fn persist_product_override(product: ProductOverride) -> Result<(), String> {
    let mut overrides = crate::db::product_overrides();
    overrides.insert(product.slug.clone(), product);
    let mut rows = overrides.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| left.slug.cmp(&right.slug));
    crate::db::replace_product_overrides(&rows).map_err(|err| {
        tracing::warn!("failed to persist HiveCore product override: {err}");
        err.to_string()
    })
}

pub async fn fetch_product_auth_status(
    client: &reqwest::Client,
    api_url: &str,
) -> Result<ProductAuthStatusBody, String> {
    let normalized = api_url.trim_end_matches('/');
    let auth_status_url = format!("{normalized}/auth/status");
    let response = client
        .get(&auth_status_url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|_| "Could not reach /auth/status.".to_string())?;

    if !response.status().is_success() {
        return Err(format!("/auth/status returned HTTP {}", response.status()));
    }

    response
        .json::<ProductAuthStatusBody>()
        .await
        .map_err(|err| format!("Could not parse /auth/status: {err}"))
}

pub async fn fetch_product_capabilities(
    client: &reqwest::Client,
    api_url: &str,
    auth: &ProductStoredAuth,
) -> Result<contract::ProductCapabilities, String> {
    let normalized = api_url.trim_end_matches('/');
    let capabilities_url = format!("{normalized}/capabilities");
    let response = authorized_get(client, &capabilities_url, auth)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|_| "Could not reach /capabilities.".to_string())?;

    if !response.status().is_success() {
        return Err(format!("/capabilities returned HTTP {}", response.status()));
    }

    response
        .json::<contract::ProductCapabilities>()
        .await
        .map_err(|err| format!("Could not parse /capabilities: {err}"))
}

pub async fn fetch_product_runs(
    client: &reqwest::Client,
    api_url: &str,
    auth: &ProductStoredAuth,
) -> (bool, Vec<contract::ProductRunSummary>, String) {
    if !auth.machine_auth_configured() {
        return (
            false,
            Vec::new(),
            "Product service token missing; recent runs unavailable.".into(),
        );
    }

    let normalized = api_url.trim_end_matches('/');
    let runs_url = format!("{normalized}/runs");
    let response = authorized_get(client, &runs_url, auth)
        .timeout(Duration::from_secs(3))
        .send()
        .await;

    let Ok(response) = response else {
        return (false, Vec::new(), "Could not reach /runs.".into());
    };

    if !response.status().is_success() {
        return (
            false,
            Vec::new(),
            format!("/runs returned HTTP {}", response.status()),
        );
    }

    match response.json::<contract::ProductRunsResponse>().await {
        Ok(body) => (true, body.runs.into_iter().take(6).collect(), String::new()),
        Err(err) => (false, Vec::new(), format!("Could not parse /runs: {err}")),
    }
}

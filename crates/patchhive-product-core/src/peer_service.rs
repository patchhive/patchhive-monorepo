use std::{
    collections::HashMap,
    fmt,
    sync::{OnceLock, RwLock},
};

use anyhow::{anyhow, Result};
use reqwest::RequestBuilder;

use crate::auth::SERVICE_TOKEN_HEADER;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PeerProduct {
    HiveCore,
    RepoMemory,
    ReviewBee,
    TrustGate,
}

#[derive(Clone, Eq, PartialEq)]
pub enum PeerServiceAuth {
    ServiceToken(String),
    ApiKey(String),
}

impl PeerServiceAuth {
    pub fn service_token(value: impl Into<String>) -> Result<Self> {
        nonempty_secret(value.into(), Self::ServiceToken, "service token")
    }

    pub fn api_key(value: impl Into<String>) -> Result<Self> {
        nonempty_secret(value.into(), Self::ApiKey, "API key")
    }

    pub fn apply(&self, request: RequestBuilder) -> RequestBuilder {
        match self {
            Self::ServiceToken(token) => request.header(SERVICE_TOKEN_HEADER, token),
            Self::ApiKey(key) => request.header("X-API-Key", key),
        }
    }
}

impl fmt::Debug for PeerServiceAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ServiceToken(_) => "ServiceToken(<redacted>)",
            Self::ApiKey(_) => "ApiKey(<redacted>)",
        })
    }
}

fn nonempty_secret(
    value: String,
    build: impl FnOnce(String) -> PeerServiceAuth,
    label: &str,
) -> Result<PeerServiceAuth> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(anyhow!("peer-service {label} must not be empty"));
    }
    Ok(build(value))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerServiceConfiguration {
    pub base_url: String,
    pub auth: PeerServiceAuth,
}

impl PeerServiceConfiguration {
    pub fn new(base_url: impl Into<String>, auth: PeerServiceAuth) -> Result<Self> {
        let base_url = base_url.into().trim().trim_end_matches('/').to_owned();
        if base_url.is_empty() {
            return Err(anyhow!("peer-service base URL must not be empty"));
        }
        Ok(Self { base_url, auth })
    }
}

fn configurations() -> &'static RwLock<HashMap<PeerProduct, PeerServiceConfiguration>> {
    static CONFIGURATIONS: OnceLock<RwLock<HashMap<PeerProduct, PeerServiceConfiguration>>> =
        OnceLock::new();
    CONFIGURATIONS.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn configure_peer_service(
    product: PeerProduct,
    configuration: PeerServiceConfiguration,
) -> Result<()> {
    let mut configured = configurations()
        .write()
        .map_err(|_| anyhow!("peer-service configuration lock is poisoned"))?;
    if let Some(existing) = configured.get(&product) {
        if existing == &configuration {
            return Ok(());
        }
        return Err(anyhow!(
            "peer-service configuration for {product:?} is already set"
        ));
    }
    configured.insert(product, configuration);
    Ok(())
}

pub fn peer_service(product: PeerProduct) -> Option<PeerServiceConfiguration> {
    configurations()
        .read()
        .ok()
        .and_then(|configured| configured.get(&product).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_redacts_credentials() {
        let configuration = PeerServiceConfiguration::new(
            "http://127.0.0.1:8100/api/products/repo-memory/",
            PeerServiceAuth::service_token("secret-value").expect("valid token"),
        )
        .expect("valid configuration");

        let rendered = format!("{configuration:?}");
        assert!(!rendered.contains("secret-value"));
        assert_eq!(
            configuration.base_url,
            "http://127.0.0.1:8100/api/products/repo-memory"
        );
    }

    #[test]
    fn empty_credentials_are_rejected() {
        assert!(PeerServiceAuth::service_token("  ").is_err());
        assert!(PeerServiceAuth::api_key("").is_err());
    }

    #[test]
    fn service_auth_uses_the_standard_machine_header() {
        let request = PeerServiceAuth::service_token("runtime-secret")
            .expect("valid token")
            .apply(reqwest::Client::new().post("http://127.0.0.1/example"))
            .build()
            .expect("request should build");

        assert_eq!(
            request
                .headers()
                .get(SERVICE_TOKEN_HEADER)
                .and_then(|value| value.to_str().ok()),
            Some("runtime-secret")
        );
        assert!(request.headers().get("X-API-Key").is_none());
    }
}

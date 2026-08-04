use anyhow::{anyhow, Result};
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    #[serde(default)]
    owned_by: String,
}

fn nonempty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn configured_url() -> Option<String> {
    nonempty_env("PATCHHIVE_AI_URL")
}

pub fn openai_base_url() -> String {
    configured_url()
        .or_else(|| nonempty_env("OPENAI_BASE_URL"))
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
}

fn authenticate_gateway_request(request: RequestBuilder) -> RequestBuilder {
    match nonempty_env("PATCHHIVE_AI_GATEWAY_API_KEY") {
        Some(key) => request.bearer_auth(key),
        None => request,
    }
}

pub fn is_configured_gateway_base(base: &str) -> bool {
    configured_url().is_some_and(|configured| normalize(&configured) == normalize(base))
}

pub async fn fetch_status(http: &Client) -> Value {
    let Some(url) = configured_url() else {
        return json!({ "configured": false });
    };

    match authenticate_gateway_request(http.get(health_url(&url)))
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
            Ok(data) => json!({
                "configured": true,
                "url": url,
                "ok": data["ok"].as_bool().unwrap_or(false),
                "gateway": data["gateway"].clone(),
                "provider_order": data["provider_order"].clone(),
                "providers": data["providers"].clone(),
                "base_url_hint": data["base_url_hint"].clone(),
            }),
            Err(error) => json!({
                "configured": true,
                "url": url,
                "ok": false,
                "error": format!("Could not parse PatchHive AI health response: {error}"),
            }),
        },
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            json!({
                "configured": true,
                "url": url,
                "ok": false,
                "error": format!("PatchHive AI gateway returned {status}: {body}"),
            })
        }
        Err(error) => json!({
            "configured": true,
            "url": url,
            "ok": false,
            "error": format!("Could not reach PatchHive AI gateway: {error}"),
        }),
    }
}

pub async fn fetch_models(http: &Client) -> Result<Vec<String>> {
    Ok(fetch_model_entries(http)
        .await?
        .into_iter()
        .map(|entry| entry.id)
        .collect())
}

pub async fn fetch_provider_models(http: &Client, provider: &str) -> Result<Vec<String>> {
    let status = fetch_status(http).await;
    let provider_status = status
        .get("providers")
        .and_then(Value::as_object)
        .and_then(|providers| providers.get(provider))
        .ok_or_else(|| anyhow!("PatchHive AI gateway did not report the {provider} provider"))?;
    let auth_status = provider_status
        .get("auth")
        .and_then(|auth| auth.get("status"))
        .and_then(Value::as_str);
    let authenticated = auth_status == Some("authenticated")
        || (auth_status.is_none()
            && provider_status.get("logged_in").and_then(Value::as_bool) == Some(true));
    if !authenticated {
        let evidence = auth_status.unwrap_or("unknown");
        return Err(anyhow!(
            "PatchHive AI provider {provider} is not authenticated (status: {evidence})"
        ));
    }

    let owner = format!("patchhive-{provider}");
    let models = fetch_model_entries(http)
        .await?
        .into_iter()
        .filter(|entry| entry.owned_by == owner)
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    if models.is_empty() {
        return Err(anyhow!(
            "PatchHive AI gateway reported no models for authenticated provider {provider}"
        ));
    }
    Ok(models)
}

async fn fetch_model_entries(http: &Client) -> Result<Vec<ModelEntry>> {
    let url = configured_url().ok_or_else(|| anyhow!("PATCHHIVE_AI_URL is not configured"))?;
    let resp = authenticate_gateway_request(http.get(models_url(&url)))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|error| {
            anyhow!("Could not reach PatchHive AI gateway models endpoint: {error}")
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("PatchHive AI gateway returned {status}: {body}"));
    }

    let list: ModelList = resp
        .json()
        .await
        .map_err(|error| anyhow!("Could not parse PatchHive AI models response: {error}"))?;
    Ok(list.data)
}

fn normalize(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

fn gateway_root(url: &str) -> String {
    let normalized = normalize(url);
    normalized
        .strip_suffix("/v1")
        .unwrap_or(&normalized)
        .to_string()
}

fn health_url(url: &str) -> String {
    format!("{}/health", gateway_root(url))
}

fn models_url(url: &str) -> String {
    format!("{}/v1/models", gateway_root(url))
}

use anyhow::{anyhow, Result};
use reqwest::Client;
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

pub fn is_local_openai_base(base: &str) -> bool {
    let base = normalize(base);
    if configured_url().is_some_and(|configured| normalize(&configured) == base) {
        return true;
    }

    let lower = base.to_ascii_lowercase();
    lower.contains("127.0.0.1") || lower.contains("localhost")
}

pub async fn fetch_status(http: &Client) -> Value {
    let Some(url) = configured_url() else {
        return json!({ "configured": false });
    };

    match http.get(health_url(&url)).timeout(Duration::from_secs(5)).send().await {
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
    let url = configured_url().ok_or_else(|| anyhow!("PATCHHIVE_AI_URL is not configured"))?;
    let resp = http
        .get(models_url(&url))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|error| anyhow!("Could not reach PatchHive AI gateway models endpoint: {error}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("PatchHive AI gateway returned {status}: {body}"));
    }

    let list: ModelList = resp
        .json()
        .await
        .map_err(|error| anyhow!("Could not parse PatchHive AI models response: {error}"))?;
    Ok(list.data.into_iter().map(|entry| entry.id).collect())
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

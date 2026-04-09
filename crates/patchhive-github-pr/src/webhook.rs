use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use http::HeaderMap;
use sha2::Sha256;

pub fn env_value(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

pub fn github_token_from_env() -> Option<String> {
    env_value(&["BOT_GITHUB_TOKEN", "GITHUB_TOKEN"])
}

pub fn verify_github_webhook_signature(headers: &HeaderMap, body: &[u8], secret: &str) -> Result<()> {
    if secret.trim().is_empty() {
        return Err(anyhow!("GitHub webhook secret is empty"));
    }

    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| anyhow!("Missing X-Hub-Signature-256 header"))?;

    let sig_hex = signature
        .strip_prefix("sha256=")
        .ok_or_else(|| anyhow!("Malformed GitHub webhook signature"))?;
    let sig_bytes = hex::decode(sig_hex).map_err(|_| anyhow!("Webhook signature was not valid hex"))?;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| anyhow!("Could not initialize webhook verifier"))?;
    mac.update(body);
    mac.verify_slice(&sig_bytes)
        .map_err(|_| anyhow!("GitHub webhook signature did not match"))?;

    Ok(())
}

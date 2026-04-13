use patchhive_product_core::startup::StartupCheck;
use reqwest::Client;

pub async fn validate_config(client: &Client) -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(format!(
        "DepTriage DB path: {}",
        crate::db::db_path()
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for this product starter.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing this starter beyond local development.",
        ));
    }

    match crate::github::validate_token(client).await {
        Ok(_) => checks.push(StartupCheck::info(
            "GitHub token is configured. DepTriage can read dependency PRs and security alerts with healthy rate limits.",
        )),
        Err(_) => checks.push(StartupCheck::warn(
            "BOT_GITHUB_TOKEN or GITHUB_TOKEN is not configured. Public dependency PR scans may still work, but Dependabot alerts and rate limits will be weaker.",
        )),
    }

    checks.push(StartupCheck::info(
        "DepTriage is read-only in the MVP. It ranks dependency PRs and alerts; it does not merge, close, or rewrite them.",
    ));
    checks.push(StartupCheck::info(
        "DepTriage turns dependency update noise into update-now, watch, and ignore-for-now queues without requiring AI for the first loop.",
    ));

    checks
}

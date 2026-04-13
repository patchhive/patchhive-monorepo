use patchhive_product_core::startup::StartupCheck;
use reqwest::Client;

pub async fn validate_config(client: &Client) -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(format!(
        "VulnTriage DB path: {}",
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
            "GitHub token is configured. VulnTriage can read code scanning and dependency alerts with healthier permissions.",
        )),
        Err(_) => checks.push(StartupCheck::warn(
            "BOT_GITHUB_TOKEN or GITHUB_TOKEN is not configured. Public reads may still work in some repos, but security APIs and rate limits will be weaker.",
        )),
    }

    checks.push(StartupCheck::info(
        "VulnTriage is read-only in the MVP. It ranks GitHub security findings; it does not dismiss alerts or mutate repositories.",
    ));
    checks.push(StartupCheck::info(
        "VulnTriage turns code scanning and dependency alerts into a ranked engineering queue without requiring AI for the first loop.",
    ));

    checks
}

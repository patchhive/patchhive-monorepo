use patchhive_product_core::startup::StartupCheck;

pub async fn validate_config() -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(format!(
        "TrustGate DB path: {}",
        crate::db::db_path()
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for TrustGate.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing TrustGate beyond local development.",
        ));
    }

    if crate::github::github_token_configured() {
        checks.push(StartupCheck::info(
            "GitHub token is configured. TrustGate can fetch private PR diffs and report results back as a status/check.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "BOT_GITHUB_TOKEN is missing. TrustGate can still fetch public PR diffs, but GitHub status/check reporting is disabled.",
        ));
    }

    if crate::github::webhook_secret_configured() {
        checks.push(StartupCheck::info(
            "GitHub webhook secret is configured. Public webhook ingestion is ready.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "TRUST_GITHUB_WEBHOOK_SECRET is not configured. The /webhooks/github endpoint will reject webhook delivery until it is set.",
        ));
    }

    if std::env::var("TRUSTGATE_PUBLIC_URL")
        .ok()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        checks.push(StartupCheck::info(
            "TRUSTGATE_PUBLIC_URL is configured, so GitHub reports can deep-link back to TrustGate history.",
        ));
    } else {
        checks.push(StartupCheck::info(
            "TRUSTGATE_PUBLIC_URL is not configured. GitHub reports will still post, but without a clickable details URL.",
        ));
    }

    checks.push(StartupCheck::info(
        "TrustGate reviews AI-generated diffs and returns safe, warn, or block recommendations.",
    ));

    checks
}

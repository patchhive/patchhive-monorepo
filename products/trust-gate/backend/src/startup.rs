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

    if crate::github::github_token().is_some() {
        checks.push(StartupCheck::info(
            "GitHub token is configured. TrustGate can fetch private PR diffs and report recommendations back to GitHub.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "GitHub token is not configured. TrustGate can still review pasted diffs and public PRs, but it cannot publish status/check output.",
        ));
    }

    if crate::github::webhook_secret().is_some() {
        checks.push(StartupCheck::info(
            "GitHub webhook secret is configured. TrustGate can accept signed pull_request webhooks.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "TRUST_GITHUB_WEBHOOK_SECRET is not configured. Public webhook ingestion is disabled until you add it.",
        ));
    }

    if std::env::var("TRUSTGATE_PUBLIC_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some()
    {
        checks.push(StartupCheck::info(
            "TRUSTGATE_PUBLIC_URL is configured. GitHub status output can link back to TrustGate review details.",
        ));
    } else {
        checks.push(StartupCheck::info(
            "TRUSTGATE_PUBLIC_URL is not configured. GitHub reports will still post, but without deep links back into TrustGate.",
        ));
    }

    checks.push(StartupCheck::info(
        "TrustGate can now review pasted diffs, fetch pull-request diffs directly from GitHub, and publish safe/warn/block recommendations back to PRs.",
    ));

    checks
}

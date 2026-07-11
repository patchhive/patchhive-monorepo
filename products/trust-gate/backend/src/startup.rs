use patchhive_product_core::{
    github_auth::verify_github_token,
    github_permissions::GitHubPermissionProfile,
    sqlite::db_path_message,
    startup::{StartupCheck, StartupCheckLevel},
};
use reqwest::Client;

pub async fn validate_config(client: &Client) -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(db_path_message(
        "TrustGate",
        crate::db::db_path(),
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info("API-key auth is enabled for TrustGate."));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing TrustGate beyond local development.",
        ));
    }

    let github_profile = GitHubPermissionProfile::DiffReview;
    match verify_github_token(client).await {
        Ok(_) => checks.push(github_profile.ready_check()),
        Err(err) => checks
            .push(github_profile.validation_failed_check(err.to_string(), StartupCheckLevel::Warn)),
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

    if patchhive_product_core::repo_memory::repo_memory_url().is_some() {
        checks.push(StartupCheck::info(
            "RepoMemory context is configured. TrustGate can enrich reviews and queue FailGuard candidates when it warns or blocks.",
        ));
    } else {
        checks.push(StartupCheck::info(
            "RepoMemory context is not configured. TrustGate will rely on repo rules alone and skip FailGuard candidate submission until PATCHHIVE_REPO_MEMORY_URL is set.",
        ));
    }

    checks
}

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

    if std::env::var("BOT_GITHUB_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .is_ok()
    {
        checks.push(StartupCheck::info(
            "GitHub token is configured. Future status-check integration can use it.",
        ));
    } else {
        checks.push(StartupCheck::info(
            "BOT_GITHUB_TOKEN is not configured. The TrustGate MVP still works because it reviews pasted diffs locally.",
        ));
    }

    checks.push(StartupCheck::info(
        "TrustGate reviews AI-generated diffs and returns safe, warn, or block recommendations.",
    ));

    checks
}

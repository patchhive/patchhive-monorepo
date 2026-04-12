use patchhive_product_core::startup::StartupCheck;

pub async fn validate_config() -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(format!(
        "MergeKeeper DB path: {}",
        crate::db::db_path()
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for MergeKeeper.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing MergeKeeper beyond local development.",
        ));
    }

    if crate::github::github_token_configured() {
        checks.push(StartupCheck::info(
            "GitHub token detected. MergeKeeper can read PR state, review pressure, and check health.",
        ));
    } else {
        checks.push(StartupCheck::error(
            "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required for GitHub-backed merge readiness checks.",
        ));
    }

    checks.push(StartupCheck::info(
        "MergeKeeper reads pull request merge pressure and returns a simple readiness state: ready, hold, or blocked.",
    ));

    checks
}

use patchhive_product_core::startup::StartupCheck;

pub async fn validate_config() -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(format!(
        "FlakeSting DB path: {}",
        crate::db::db_path()
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for FlakeSting.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing FlakeSting beyond local development.",
        ));
    }

    if crate::github::github_token_configured() {
        checks.push(StartupCheck::info(
            "GitHub token detected. FlakeSting can read workflow runs and jobs with healthier rate limits.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "BOT_GITHUB_TOKEN or GITHUB_TOKEN is not configured. Public-repo scans may still work, but GitHub rate limits will be much tighter.",
        ));
    }

    checks.push(StartupCheck::info(
        "FlakeSting reads recent GitHub Actions history, looks for fail/pass swings in test jobs or steps, and ranks likely flaky CI signals without using AI.",
    ));

    checks
}

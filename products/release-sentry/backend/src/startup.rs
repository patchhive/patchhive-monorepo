use patchhive_product_core::startup::StartupCheck;

pub async fn validate_config(client: &reqwest::Client) -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(format!(
        "ReleaseSentry DB path: {}",
        crate::db::db_path()
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for ReleaseSentry.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing ReleaseSentry beyond local development.",
        ));
    }

    checks.push(StartupCheck::info(
        "ReleaseSentry starts read-only: release readiness should collect evidence before it gates publishing or deploys.",
    ));

    if crate::github::github_token_configured() {
        match crate::github::validate_token(client).await {
            Ok(_) => checks.push(StartupCheck::info(
                "GitHub token detected. ReleaseSentry can read private repos, Actions history, issues, tags, releases, and changelog files.",
            )),
            Err(err) => checks.push(StartupCheck::warn(format!(
                "GitHub token is configured, but validation failed: {err}"
            ))),
        }
    } else {
        checks.push(StartupCheck::warn(
            "BOT_GITHUB_TOKEN is not configured. Public GitHub release checks may work, but rate limits and private repos will be limited.",
        ));
    }

    checks
}

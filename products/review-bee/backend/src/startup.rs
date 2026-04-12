use patchhive_product_core::startup::StartupCheck;

pub async fn validate_config() -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(format!(
        "ReviewBee DB path: {}",
        crate::db::db_path()
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for ReviewBee.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing ReviewBee beyond local development.",
        ));
    }

    if crate::github::github_token_configured() {
        checks.push(StartupCheck::info(
            "GitHub token detected. ReviewBee can fetch PR reviews and review threads.",
        ));
    } else {
        checks.push(StartupCheck::error(
            "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required for GitHub-backed review analysis.",
        ));
    }

    checks.push(StartupCheck::info(
        "ReviewBee clusters actionable PR review feedback into a merge checklist and keeps a local history of prior runs.",
    ));

    checks
}

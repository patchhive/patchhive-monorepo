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
            "GitHub token detected. ReviewBee can fetch PR reviews and maintain a PR comment when requested.",
        ));
    } else {
        checks.push(StartupCheck::error(
            "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required for GitHub-backed review analysis.",
        ));
    }

    if crate::github::webhook_secret_configured() {
        checks.push(StartupCheck::info(
            "GitHub webhook secret is configured. ReviewBee can auto-refresh on supported PR review events.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "REVIEW_BEE_GITHUB_WEBHOOK_SECRET is not configured. The /webhooks/github endpoint will reject webhook delivery until it is set.",
        ));
    }

    if crate::github::public_url_configured() {
        checks.push(StartupCheck::info(
            "REVIEW_BEE_PUBLIC_URL is configured. Maintained PR comments can deep-link back to ReviewBee history pages.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "REVIEW_BEE_PUBLIC_URL is not configured. ReviewBee can still post PR comments, but they will not include a public details link.",
        ));
    }

    checks.push(StartupCheck::info(
        "ReviewBee clusters actionable PR review feedback into a merge checklist, keeps a local history of prior runs, and can maintain a single PR comment artifact when GitHub publishing is enabled.",
    ));

    checks
}

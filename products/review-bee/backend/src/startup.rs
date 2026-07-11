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
        "ReviewBee",
        crate::db::db_path(),
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info("API-key auth is enabled for ReviewBee."));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing ReviewBee beyond local development.",
        ));
    }

    let github_profile = GitHubPermissionProfile::PrReview;
    match verify_github_token(client).await {
        Ok(_) => checks.push(github_profile.ready_check()),
        Err(err) => checks.push(
            github_profile.validation_failed_check(err.to_string(), StartupCheckLevel::Error),
        ),
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

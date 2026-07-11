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
        "FlakeSting",
        crate::db::db_path(),
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

    let github_profile = GitHubPermissionProfile::ActionsRead;
    match verify_github_token(client).await {
        Ok(_) => checks.push(github_profile.ready_check()),
        Err(err) => checks
            .push(github_profile.validation_failed_check(err.to_string(), StartupCheckLevel::Warn)),
    }

    checks.push(StartupCheck::info(
        "FlakeSting reads recent GitHub Actions history, looks for fail/pass swings in test jobs or steps, and ranks likely flaky CI signals without using AI.",
    ));

    checks
}

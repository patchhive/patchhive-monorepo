use patchhive_product_core::{
    github_permissions::GitHubPermissionProfile,
    sqlite::db_path_message,
    startup::{StartupCheck, StartupCheckLevel},
};
use reqwest::Client;

pub async fn validate_config(client: &Client) -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(db_path_message(
        "VulnTriage",
        crate::db::db_path(),
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for VulnTriage.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing VulnTriage beyond local development.",
        ));
    }

    let github_profile = GitHubPermissionProfile::SecurityTriage;
    match crate::github::validate_token(client).await {
        Ok(_) => checks.push(github_profile.ready_check()),
        Err(err) => checks
            .push(github_profile.validation_failed_check(err.to_string(), StartupCheckLevel::Warn)),
    }

    checks.push(StartupCheck::info(
        "VulnTriage is read-only in the MVP. It ranks GitHub security findings; it does not dismiss alerts or mutate repositories.",
    ));
    checks.push(StartupCheck::info(
        "VulnTriage turns code scanning and dependency alerts into a ranked engineering queue without requiring AI for the first loop.",
    ));

    checks
}

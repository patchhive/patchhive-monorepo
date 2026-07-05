use patchhive_product_core::{
    github_permissions::GitHubPermissionProfile,
    sqlite::db_path_message,
    startup::{StartupCheck, StartupCheckLevel},
};
use reqwest::Client;

pub async fn validate_config(client: &Client) -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(db_path_message(
        "DepTriage",
        crate::db::db_path(),
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info("API-key auth is enabled for DepTriage."));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing DepTriage beyond local development.",
        ));
    }

    let github_profile = GitHubPermissionProfile::DependencyTriage;
    match crate::github::validate_token(client).await {
        Ok(_) => checks.push(github_profile.ready_check()),
        Err(err) => checks
            .push(github_profile.validation_failed_check(err.to_string(), StartupCheckLevel::Warn)),
    }

    checks.push(StartupCheck::info(
        "DepTriage is read-only in the MVP. It ranks dependency PRs and alerts; it does not merge, close, or rewrite them.",
    ));
    checks.push(StartupCheck::info(
        "DepTriage turns dependency update noise into update-now, watch, and ignore-for-now queues without requiring AI for the first loop.",
    ));

    checks
}

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
        "RepoMemory",
        crate::db::db_path(),
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for RepoMemory.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing RepoMemory beyond local development.",
        ));
    }

    let github_profile = GitHubPermissionProfile::RepoHistory;
    match verify_github_token(client).await {
        Ok(_) => checks.push(github_profile.ready_check()),
        Err(err) => checks
            .push(github_profile.validation_failed_check(err.to_string(), StartupCheckLevel::Warn)),
    }

    checks.push(StartupCheck::info(
        "RepoMemory builds durable repo memory from merged PRs, reviewer feedback, and past bugs.",
    ));
    checks.push(StartupCheck::info(
        "RepoMemory does not require a live AI provider for the MVP loop. It uses GitHub data plus deterministic extraction heuristics.",
    ));

    checks
}

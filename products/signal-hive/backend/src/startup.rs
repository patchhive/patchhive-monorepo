use patchhive_product_core::{
    github_auth::verify_github_token,
    github_permissions::GitHubPermissionProfile,
    sqlite::db_path_message,
    startup::{StartupCheck, StartupCheckLevel},
};

pub async fn validate_config(client: &reqwest::Client) -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(db_path_message(
        "SignalHive",
        crate::db::db_path(),
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for SignalHive.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing SignalHive beyond local development.",
        ));
    }

    let github_profile = GitHubPermissionProfile::RepoDiscovery;
    match verify_github_token(client).await {
        Ok(_) => checks.push(github_profile.ready_check()),
        Err(err) => checks.push(
            github_profile.validation_failed_check(err.to_string(), StartupCheckLevel::Error),
        ),
    }

    checks.push(StartupCheck::info(
        "SignalHive is read-only: it scans repos and issues but does not open PRs or write code.",
    ));

    checks
}

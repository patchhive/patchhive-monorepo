use patchhive_product_core::{
    github_permissions::GitHubPermissionProfile,
    startup::{StartupCheck, StartupCheckLevel},
};

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

    let github_profile = GitHubPermissionProfile::ReleaseRead;
    if crate::github::github_token_configured() {
        match crate::github::validate_token(client).await {
            Ok(_) => checks.push(github_profile.ready_check()),
            Err(err) => checks.push(
                github_profile.validation_failed_check(err.to_string(), StartupCheckLevel::Warn),
            ),
        }
    } else {
        checks.push(github_profile.missing_check(StartupCheckLevel::Warn));
    }

    checks
}

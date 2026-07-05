use patchhive_product_core::{
    github_permissions::GitHubPermissionProfile,
    sqlite::db_path_message,
    startup::{StartupCheck, StartupCheckLevel},
};

pub async fn validate_config() -> Vec<StartupCheck> {
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
    if crate::github::github_token_configured() {
        checks.push(github_profile.ready_check());
    } else {
        checks.push(github_profile.missing_check(StartupCheckLevel::Warn));
    }

    checks.push(StartupCheck::info(
        "FlakeSting reads recent GitHub Actions history, looks for fail/pass swings in test jobs or steps, and ranks likely flaky CI signals without using AI.",
    ));

    checks
}

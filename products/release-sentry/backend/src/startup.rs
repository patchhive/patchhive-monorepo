use patchhive_product_core::startup::StartupCheck;

pub async fn validate_config() -> Vec<StartupCheck> {
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

    checks
}

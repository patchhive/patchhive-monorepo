use patchhive_product_core::startup::StartupCheck;

use crate::state::{allowed_roots_env_example, AppState};

pub async fn validate_config(state: &AppState) -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(format!(
        "RefactorScout DB path: {}",
        crate::db::db_path()
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for RefactorScout.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing RefactorScout beyond local development.",
        ));
    }

    let allowed_roots = state.allowed_root_labels();
    if allowed_roots.is_empty() {
        checks.push(StartupCheck::error(
            "RefactorScout has no readable allowed roots configured, so local repo scans will fail.",
        ));
    } else {
        checks.push(StartupCheck::info(format!(
            "Filesystem scans are confined to: {}",
            allowed_roots.join(", ")
        )));
    }

    if std::env::var_os("REFACTOR_SCOUT_ALLOWED_ROOTS").is_none() {
        checks.push(StartupCheck::warn(format!(
            "REFACTOR_SCOUT_ALLOWED_ROOTS is not set, so RefactorScout currently defaults to the process working directory. Set it to a path list like {} before you point the app at broader local checkouts.",
            allowed_roots_env_example()
        )));
    }

    if state.remote_fs_enabled {
        checks.push(StartupCheck::warn(
            "REFACTOR_SCOUT_ALLOW_REMOTE_FS is enabled. Authenticated non-local clients may trigger filesystem scans against the configured allowed roots.",
        ));
    } else {
        checks.push(StartupCheck::info(
            "Filesystem scans are limited to localhost callers unless REFACTOR_SCOUT_ALLOW_REMOTE_FS=true.",
        ));
    }

    checks.push(StartupCheck::info(
        "RefactorScout is currently a read-only local repo scanner for safe refactor opportunities such as oversized files, oversized functions, and repeated string literals.",
    ));

    checks
}

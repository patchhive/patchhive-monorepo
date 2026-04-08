use patchhive_product_core::startup::StartupCheck;

pub async fn validate_config() -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(format!(
        "__PRODUCT_TITLE__ DB path: {}",
        crate::db::db_path()
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for this product starter.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing this starter beyond local development.",
        ));
    }

    checks.push(StartupCheck::info(
        "This product still uses the shared PatchHive starter scaffold. Replace the overview route and checks with product-specific behavior as soon as the loop is real.",
    ));

    checks
}

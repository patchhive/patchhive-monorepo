use once_cell::sync::OnceCell;
use patchhive_product_core::secrets::{validate_encryption_secret, TokenProtector};
use patchhive_product_core::sqlite::db_path_message;
use patchhive_product_core::startup::{StartupCheck, StartupCheckLevel};

static STARTUP_CHECKS: OnceCell<Vec<StartupCheck>> = OnceCell::new();

pub fn set_startup_checks(checks: Vec<StartupCheck>) {
    let _ = STARTUP_CHECKS.set(checks);
}

pub fn startup_checks() -> Vec<StartupCheck> {
    STARTUP_CHECKS.get().cloned().unwrap_or_default()
}

pub async fn validate_config() -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(db_path_message(
        "HiveCore",
        crate::db::db_path(),
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info("API-key auth is enabled for HiveCore."));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing HiveCore beyond local development.",
        ));
    }

    checks.push(StartupCheck::info(
        "HiveCore ships with a built-in localhost product registry and can persist per-product URL overrides for subdomains or remote deployments.",
    ));

    let override_count = crate::db::product_override_count();
    if override_count == 0 {
        checks.push(StartupCheck::info(
            "HiveCore is currently using its built-in default product URLs. Save suite settings to override them per environment.",
        ));
    } else {
        checks.push(StartupCheck::ok(format!(
            "HiveCore has {} persisted product override{} ready for launch links and health polling.",
            override_count,
            if override_count == 1 { "" } else { "s" }
        )));
    }

    let repository_policy_count = crate::db::repository_policies().len();
    checks.push(StartupCheck::ok(format!(
        "HiveCore repository safety is active with {repository_policy_count} structured polic{}; local exclusions and trusted-repository elevations are available to suite products.",
        if repository_policy_count == 1 { "y" } else { "ies" }
    )));
    checks.push(StartupCheck::ok(format!(
        "Atomic pull-request budgets are active with a suite-wide ceiling of {}. RepoReaper reserves capacity before PR creation and releases it when monitored PRs close or merge.",
        crate::db::suite_pr_limit()
    )));
    checks.push(StartupCheck::info(
        "The public patchhive.dev repository-owner opt-out registry is not connected yet. HiveCore currently enforces operator-managed exclusions only.",
    ));

    let token_stats = crate::db::service_token_storage_stats();
    let protector = TokenProtector::from_env("HIVECORE_ENCRYPTION_KEY");
    if let Ok(secret) = std::env::var("HIVECORE_ENCRYPTION_KEY") {
        let secret = secret.trim();
        if !secret.is_empty() {
            match validate_encryption_secret(secret) {
                Ok(()) => checks.push(StartupCheck::ok(
                    "HIVECORE_ENCRYPTION_KEY is configured with sufficient machine-random key material.",
                )),
                Err(error) => checks.push(StartupCheck::error(format!(
                    "HIVECORE_ENCRYPTION_KEY is not safe encryption key material: {error}"
                ))),
            }
        }
    }
    if token_stats.total == 0 {
        checks.push(StartupCheck::info(
            "HiveCore has no saved downstream product service tokens yet.",
        ));
    } else if protector.configured() {
        if token_stats.plaintext == 0 {
            checks.push(StartupCheck::ok(format!(
                "HiveCore has {} saved product service token{} encrypted at rest.",
                token_stats.total,
                if token_stats.total == 1 { "" } else { "s" }
            )));
        } else {
            checks.push(StartupCheck::warn(format!(
                "HiveCore still has {} plaintext product service token{} in SQLite. Restart with HIVECORE_ENCRYPTION_KEY and let boot migration finish before trusting at-rest protection.",
                token_stats.plaintext,
                if token_stats.plaintext == 1 { "" } else { "s" }
            )));
        }
    } else {
        if token_stats.encrypted > 0 {
            checks.push(StartupCheck::warn(format!(
                "HIVECORE_ENCRYPTION_KEY is not set, but {} saved product service token{} are encrypted. HiveCore cannot read them until that key is restored.",
                token_stats.encrypted,
                if token_stats.encrypted == 1 { "" } else { "s" }
            )));
        }
        if token_stats.plaintext > 0 {
            checks.push(StartupCheck::warn(format!(
                "HIVECORE_ENCRYPTION_KEY is not set. HiveCore currently keeps {} product service token{} in plaintext SQLite storage.",
                token_stats.plaintext,
                if token_stats.plaintext == 1 { "" } else { "s" }
            )));
        }
    }

    checks.push(StartupCheck::info(
        "HiveCore provides visibility, saved defaults, live product health polling, repository policy, and shared outbound PR capacity. Additional products should adopt the same typed policy client before gaining write actions.",
    ));

    checks
}

pub fn summarize_check_levels(checks: &[StartupCheck]) -> (u32, u32, u32) {
    let mut errors = 0;
    let mut warns = 0;
    let mut infos = 0;

    for check in checks {
        match check.level {
            StartupCheckLevel::Error => errors += 1,
            StartupCheckLevel::Warn => warns += 1,
            _ => infos += 1,
        }
    }

    (errors, warns, infos)
}

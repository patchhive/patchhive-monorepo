use crate::github;
use patchhive_product_core::startup::StartupCheck;

pub async fn validate_config(client: &reqwest::Client) -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    if std::env::var("BOT_GITHUB_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .is_err()
    {
        checks.push(StartupCheck::error(
            "BOT_GITHUB_TOKEN is missing. SignalHive cannot scan GitHub without it.",
        ));
    } else {
        match github::validate_token(client).await {
            Ok(_) => checks.push(StartupCheck::info("GitHub token looks valid.")),
            Err(err) => checks.push(StartupCheck::warn(format!(
                "GitHub token check failed: {err}"
            ))),
        }
    }

    checks.push(StartupCheck::info(format!(
        "SignalHive DB path: {}",
        crate::db::db_path()
    )));
    checks.push(StartupCheck::info(
        "SignalHive is read-only: it scans repos and issues but does not open PRs or write code.",
    ));

    checks
}

use serde_json::json;

use crate::github;

pub async fn validate_config(client: &reqwest::Client) -> Vec<serde_json::Value> {
    let mut checks = Vec::new();

    if std::env::var("BOT_GITHUB_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .is_err()
    {
        checks.push(json!({
            "level": "error",
            "msg": "BOT_GITHUB_TOKEN is missing. SignalHive cannot scan GitHub without it."
        }));
    } else {
        match github::validate_token(client).await {
            Ok(_) => checks.push(json!({
                "level": "info",
                "msg": "GitHub token looks valid."
            })),
            Err(err) => checks.push(json!({
                "level": "warn",
                "msg": format!("GitHub token check failed: {err}")
            })),
        }
    }

    checks.push(json!({
        "level": "info",
        "msg": format!("SignalHive DB path: {}", crate::db::db_path())
    }));
    checks.push(json!({
        "level": "info",
        "msg": "SignalHive is read-only: it scans repos and issues but does not open PRs or write code."
    }));

    checks
}

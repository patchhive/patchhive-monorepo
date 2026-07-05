use patchhive_product_core::{sqlite::db_path_message, startup::StartupCheck};
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

    match crate::github::validate_token(client).await {
        Ok(_) => checks.push(StartupCheck::info(
            "GitHub token is configured. RepoMemory can ingest merged PRs, review feedback, and closed issues.",
        )),
        Err(_) => checks.push(StartupCheck::warn(
            "BOT_GITHUB_TOKEN is missing. RepoMemory can load, but GitHub-backed ingestion is disabled until a token is configured.",
        )),
    }

    checks.push(StartupCheck::info(
        "RepoMemory builds durable repo memory from merged PRs, reviewer feedback, and past bugs.",
    ));
    checks.push(StartupCheck::info(
        "RepoMemory does not require a live AI provider for the MVP loop. It uses GitHub data plus deterministic extraction heuristics.",
    ));

    checks
}

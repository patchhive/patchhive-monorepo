use patchhive_product_core::{
    ai_gateway::AiGatewayConfiguration,
    github_auth::verify_github_token,
    github_permissions::GitHubPermissionProfile,
    sqlite::db_path_message,
    startup::{StartupCheck, StartupCheckLevel},
};
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
        ).with_identity("api_key_auth", "missing"));
    }

    let github_profile = GitHubPermissionProfile::RepoHistory;
    match verify_github_token(client).await {
        Ok(_) => checks.push(github_profile.ready_check()),
        Err(err) => checks
            .push(github_profile.validation_failed_check(err.to_string(), StartupCheckLevel::Warn)),
    }

    checks.push(StartupCheck::info(
        "RepoMemory builds durable repo memory from merged PRs, reviewer feedback, and past bugs.",
    ));
    match crate::pipeline::failguard_ai::enabled_from_environment() {
        Ok(false) => checks.push(StartupCheck::info(
            "FailGuard AI interpretation is explicitly disabled; deterministic candidate capture remains active.",
        )),
        Err(error) => checks.push(
            StartupCheck::warn(format!(
                "FailGuard AI interpretation configuration is invalid: {error}"
            ))
            .with_identity("failguard_ai", "invalid"),
        ),
        Ok(true) => match AiGatewayConfiguration::from_environment() {
            Ok(Some(_)) => checks.push(StartupCheck::info(
                "FailGuard AI interpretation is configured. Model output remains review-only and cannot promote guardrails.",
            )),
            Ok(None) => checks.push(StartupCheck::info(
                "FailGuard deterministic capture is ready; AI interpretation will be not_observed until PATCHHIVE_AI_URL is configured.",
            )),
            Err(error) => checks.push(
                StartupCheck::warn(format!(
                    "FailGuard AI interpretation configuration is invalid: {error}"
                ))
                .with_identity("failguard_ai", "invalid"),
            ),
        },
    }

    checks
}

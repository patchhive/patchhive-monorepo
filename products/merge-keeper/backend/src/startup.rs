use patchhive_product_core::{
    github_permissions::GitHubPermissionProfile,
    sqlite::db_path_message,
    startup::{StartupCheck, StartupCheckLevel},
};

pub async fn validate_config() -> Vec<StartupCheck> {
    let mut checks = Vec::new();

    checks.push(StartupCheck::info(db_path_message(
        "MergeKeeper",
        crate::db::db_path(),
    )));

    if crate::auth::auth_enabled() {
        checks.push(StartupCheck::info(
            "API-key auth is enabled for MergeKeeper.",
        ));
    } else {
        checks.push(StartupCheck::warn(
            "API-key auth is not enabled yet. Generate a key before exposing MergeKeeper beyond local development.",
        ));
    }

    let github_profile = GitHubPermissionProfile::MergeReadiness;
    if crate::github::github_token_configured() {
        checks.push(github_profile.ready_check());
    } else {
        checks.push(github_profile.missing_check(StartupCheckLevel::Error));
    }

    checks.push(StartupCheck::info(
        "MergeKeeper reads pull request merge pressure and returns a simple readiness state: ready, hold, or blocked.",
    ));
    if crate::pipeline::assessment::approval_required_default() {
        checks.push(StartupCheck::info(
            "Approval policy defaults to required. Manual runs can disable it for repos that do not require review approval.",
        ));
    } else {
        checks.push(StartupCheck::info(
            "Approval policy defaults to optional. Manual runs can still require approval per assessment.",
        ));
    }

    if let Some(url) = crate::integrations::review_bee_url() {
        checks.push(StartupCheck::info(format!(
            "ReviewBee integration is configured at {url}. MergeKeeper can layer active review churn into readiness."
        )));
    } else {
        checks.push(StartupCheck::info(
            "ReviewBee integration is not configured. MergeKeeper will rely on GitHub review state alone.",
        ));
    }

    if let Some(url) = crate::integrations::trust_gate_url() {
        checks.push(StartupCheck::info(format!(
            "TrustGate integration is configured at {url}. MergeKeeper can keep risky PRs on hold even when checks are green."
        )));
    } else {
        checks.push(StartupCheck::info(
            "TrustGate integration is not configured. MergeKeeper will not layer diff-risk policy into readiness yet.",
        ));
    }

    if patchhive_product_core::repo_memory::repo_memory_url().is_some() {
        checks.push(StartupCheck::info(
            "RepoMemory integration is configured. MergeKeeper can pull repo-specific expectations into its merge call.",
        ));
    } else {
        checks.push(StartupCheck::info(
            "RepoMemory integration is not configured. MergeKeeper will skip repo-specific memory hints for now.",
        ));
    }

    if crate::github::webhook_secret_configured() {
        checks.push(StartupCheck::info(
            "GitHub webhook secret is configured. MergeKeeper can refresh itself from supported PR events.",
        ));
    } else {
        checks.push(StartupCheck::info(
            "GitHub webhook secret is not configured. MergeKeeper will stay manual until MERGE_KEEPER_GITHUB_WEBHOOK_SECRET is set.",
        ));
    }

    if crate::github::public_url_configured() {
        checks.push(StartupCheck::info(
            "Public URL is configured. MergeKeeper can link PR comments back to a shareable run view.",
        ));
    } else {
        checks.push(StartupCheck::info(
            "Public URL is not configured. GitHub artifacts can still be published when the token has write access, but deep links back to the app stay local-only.",
        ));
    }

    checks
}

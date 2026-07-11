use crate::startup::{StartupCheck, StartupCheckLevel};

pub const GITHUB_TOKEN_CHECK_CODE: &str = "github_token";
pub const GITHUB_TOKEN_VERIFIED: &str = "verified";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHubPermissionProfile {
    ActionsRead,
    AutonomousWrite,
    DependencyTriage,
    DiffReview,
    MergeReadiness,
    PrReview,
    RepoDiscovery,
    RepoHistory,
    ReleaseRead,
    SecurityTriage,
}

impl GitHubPermissionProfile {
    pub fn ready_message(self) -> &'static str {
        match self {
            Self::ActionsRead => {
                "GitHub accepted the configured token. FlakeSting will verify Actions access for each target repository during a scan."
            }
            Self::AutonomousWrite => {
                "GitHub accepted the configured token. RepoReaper will verify repository, branch, pull-request, and publishing access for each target during a run."
            }
            Self::DependencyTriage => {
                "GitHub accepted the configured token. DepTriage will verify dependency PR and Dependabot access for each target repository during a scan."
            }
            Self::DiffReview => {
                "GitHub accepted the configured token. TrustGate will verify PR-diff and report-publishing access against each target pull request."
            }
            Self::MergeReadiness => {
                "GitHub accepted the configured token. MergeKeeper will verify PR, check, and publishing access against each target pull request."
            }
            Self::PrReview => {
                "GitHub accepted the configured token. ReviewBee will verify PR-read and comment-publish access against each target pull request."
            }
            Self::RepoDiscovery => {
                "GitHub accepted the configured token. SignalHive will verify repository and issue access against each target during a scan."
            }
            Self::RepoHistory => {
                "GitHub accepted the configured token. RepoMemory will verify PR, review, and issue-history access for each target during ingestion."
            }
            Self::ReleaseRead => {
                "GitHub accepted the configured token. ReleaseSentry will verify repository and release-data access for each target during an assessment."
            }
            Self::SecurityTriage => {
                "GitHub accepted the configured token. VulnTriage will verify code-scanning and Dependabot access for each target repository during a scan."
            }
        }
    }

    pub fn missing_message(self) -> &'static str {
        match self {
            Self::ActionsRead => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is not configured. Public-repo scans may still work, but GitHub rate limits will be much tighter."
            }
            Self::AutonomousWrite => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required for RepoReaper discovery, branch creation, and pull-request publishing."
            }
            Self::DependencyTriage => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is not configured. Public dependency PR scans may still work, but Dependabot alerts and rate limits will be weaker."
            }
            Self::DiffReview => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is not configured. TrustGate can still review pasted diffs, but GitHub PR reads and reporting are unavailable."
            }
            Self::MergeReadiness => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required for GitHub-backed merge readiness checks and GitHub report publishing."
            }
            Self::PrReview => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required for GitHub-backed review analysis."
            }
            Self::RepoDiscovery => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required for GitHub-backed SignalHive scans."
            }
            Self::RepoHistory => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is not configured. RepoMemory can load, but GitHub-backed ingestion is unavailable."
            }
            Self::ReleaseRead => {
                "BOT_GITHUB_TOKEN is not configured. Public GitHub release checks may work, but rate limits and private repos will be limited."
            }
            Self::SecurityTriage => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is not configured. Public reads may still work in some repos, but security APIs and rate limits will be weaker."
            }
        }
    }

    pub fn recommended_scopes(self) -> &'static str {
        match self {
            Self::ActionsRead => "Metadata (read), Actions (read).",
            Self::AutonomousWrite => {
                "Metadata (read), Contents (read/write), Issues (read), Pull requests (read/write), Checks (read), and Workflows (read)."
            }
            Self::DependencyTriage => {
                "Metadata (read), Pull requests (read), Dependabot alerts (read)."
            }
            Self::DiffReview => {
                "Metadata (read), Contents (read), Pull requests (read). Add Checks (write), Commit statuses (write), and Issues (write) for GitHub reporting."
            }
            Self::MergeReadiness => {
                "Metadata (read), Pull requests (read), Checks (read), Commit statuses (read). Add Checks (write), Commit statuses (write), and Issues (write) for GitHub publishing."
            }
            Self::PrReview => {
                "Metadata (read), Pull requests (read). Add Issues (write) when publishing ReviewBee maintained comments."
            }
            Self::RepoDiscovery => {
                "Metadata (read), Contents (read), Issues (read), Pull requests (read)."
            }
            Self::RepoHistory => {
                "Metadata (read), Contents (read), Issues (read), Pull requests (read)."
            }
            Self::ReleaseRead => {
                "Metadata (read), Contents (read), Pull requests (read), Actions (read), Commit statuses (read), and Releases/Deployments read access where available."
            }
            Self::SecurityTriage => {
                "Metadata (read), Code scanning alerts (read), Dependabot alerts (read)."
            }
        }
    }

    pub fn ready_check(self) -> StartupCheck {
        StartupCheck::ok(self.ready_message())
            .with_identity(GITHUB_TOKEN_CHECK_CODE, GITHUB_TOKEN_VERIFIED)
    }

    pub fn missing_check(self, level: StartupCheckLevel) -> StartupCheck {
        startup_check(level, self.missing_message())
            .with_identity(GITHUB_TOKEN_CHECK_CODE, "missing")
    }

    pub fn validation_failed_check(
        self,
        error: impl AsRef<str>,
        level: StartupCheckLevel,
    ) -> StartupCheck {
        let error = error.as_ref();
        if github_token_error_is_missing(error) {
            return self.missing_check(level);
        }
        startup_check(
            level,
            format!("GitHub token is configured, but validation failed: {error}"),
        )
        .with_identity(GITHUB_TOKEN_CHECK_CODE, "failed")
    }
}

pub fn github_token_verified(checks: &[StartupCheck]) -> bool {
    crate::startup::check_has_status(checks, GITHUB_TOKEN_CHECK_CODE, GITHUB_TOKEN_VERIFIED)
}

fn startup_check(level: StartupCheckLevel, message: impl Into<String>) -> StartupCheck {
    match level {
        StartupCheckLevel::Ok => StartupCheck::ok(message),
        StartupCheckLevel::Info => StartupCheck::info(message),
        StartupCheckLevel::Warn => StartupCheck::warn(message),
        StartupCheckLevel::Error => StartupCheck::error(message),
    }
}

fn github_token_error_is_missing(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("[missing_token]")
        || lower.contains("bot_github_token is not set")
        || lower.contains("github_token is not set")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_profile_documents_dependabot_scope() {
        assert!(GitHubPermissionProfile::DependencyTriage
            .recommended_scopes()
            .contains("Dependabot alerts"));
    }

    #[test]
    fn merge_readiness_separates_identity_from_target_permissions() {
        let message = GitHubPermissionProfile::MergeReadiness.ready_message();

        assert!(message.contains("GitHub accepted the configured token"));
        assert!(message.contains("verify PR, check, and publishing access against each target"));
        assert!(GitHubPermissionProfile::MergeReadiness
            .recommended_scopes()
            .contains("Checks (write)"));
    }

    #[test]
    fn verified_check_carries_machine_readable_identity_status() {
        let check = GitHubPermissionProfile::PrReview.ready_check();

        assert_eq!(check.level, StartupCheckLevel::Ok);
        assert_eq!(check.code.as_deref(), Some(GITHUB_TOKEN_CHECK_CODE));
        assert_eq!(check.status.as_deref(), Some(GITHUB_TOKEN_VERIFIED));
        assert!(github_token_verified(&[check]));
    }

    #[test]
    fn failed_check_is_not_treated_as_verified() {
        let check = GitHubPermissionProfile::PrReview
            .validation_failed_check("Bad credentials", StartupCheckLevel::Error);

        assert_eq!(check.status.as_deref(), Some("failed"));
        assert!(!github_token_verified(&[check]));
    }

    #[test]
    fn missing_validation_errors_use_profile_missing_copy() {
        let check = GitHubPermissionProfile::SecurityTriage.validation_failed_check(
            "[missing_token]: BOT_GITHUB_TOKEN is not set",
            StartupCheckLevel::Warn,
        );

        assert_eq!(check.level, StartupCheckLevel::Warn);
        assert!(check.msg.contains("security APIs"));
    }

    #[test]
    fn rejected_token_errors_keep_validation_detail() {
        let check = GitHubPermissionProfile::ReleaseRead
            .validation_failed_check("Bad credentials", StartupCheckLevel::Warn);

        assert!(check.msg.contains("validation failed"));
        assert!(check.msg.contains("Bad credentials"));
    }
}

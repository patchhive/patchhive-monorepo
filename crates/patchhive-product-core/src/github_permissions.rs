use crate::{
    github_auth::{
        github_read_token_source, github_read_token_uses_legacy_name, REPO_REAPER_GITHUB_TOKEN_RW,
    },
    startup::{StartupCheck, StartupCheckLevel},
};

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
                "GitHub accepted the shared read credential. FlakeSting will verify Actions access for each target repository during a scan."
            }
            Self::AutonomousWrite => {
                "GitHub accepted RepoReaper's dedicated write credential. RepoReaper will verify repository, branch, pull-request, and publishing access for each target during a run."
            }
            Self::DependencyTriage => {
                "GitHub accepted the configured token. DepTriage will verify dependency PR and Dependabot access for each target repository during a scan."
            }
            Self::DiffReview => {
                "GitHub accepted the shared read credential. TrustGate will verify PR-diff access against each target pull request; publishing uses its separate write credential."
            }
            Self::MergeReadiness => {
                "GitHub accepted the shared read credential. MergeKeeper will verify PR and check access against each target pull request; publishing uses its separate write credential."
            }
            Self::PrReview => {
                "GitHub accepted the shared read credential. ReviewBee will verify PR-read access against each target pull request; comment publishing uses its separate write credential."
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
                "PATCHHIVE_GITHUB_TOKEN_RO is not configured. Public-repo scans may still work, but GitHub rate limits will be much tighter."
            }
            Self::AutonomousWrite => {
                "REPO_REAPER_GITHUB_TOKEN_RW is required for RepoReaper discovery, branch creation, and pull-request publishing."
            }
            Self::DependencyTriage => {
                "PATCHHIVE_GITHUB_TOKEN_RO is not configured. Public dependency PR scans may still work, but Dependabot alerts and rate limits will be weaker."
            }
            Self::DiffReview => {
                "PATCHHIVE_GITHUB_TOKEN_RO is not configured. TrustGate can still review pasted diffs, but GitHub PR reads are unavailable."
            }
            Self::MergeReadiness => {
                "PATCHHIVE_GITHUB_TOKEN_RO is required for GitHub-backed merge-readiness analysis."
            }
            Self::PrReview => {
                "PATCHHIVE_GITHUB_TOKEN_RO is required for GitHub-backed review analysis."
            }
            Self::RepoDiscovery => {
                "PATCHHIVE_GITHUB_TOKEN_RO is required for GitHub-backed SignalHive scans."
            }
            Self::RepoHistory => {
                "PATCHHIVE_GITHUB_TOKEN_RO is not configured. RepoMemory can load, but GitHub-backed ingestion is unavailable."
            }
            Self::ReleaseRead => {
                "PATCHHIVE_GITHUB_TOKEN_RO is not configured. Public GitHub release checks may work, but rate limits and private repos will be limited."
            }
            Self::SecurityTriage => {
                "PATCHHIVE_GITHUB_TOKEN_RO is not configured. Public reads may still work in some repos, but security APIs and rate limits will be weaker."
            }
        }
    }

    pub fn recommended_scopes(self) -> &'static str {
        match self {
            Self::ActionsRead => "Classic PAT with public_repo (recommended for public repositories) or repo (private repositories). PatchHive uses it for Actions reads only.",
            Self::AutonomousWrite => {
                "Dedicated classic PAT with public_repo (public repositories) or repo (private repositories). Add workflow only when RepoReaper is allowed to modify GitHub Actions workflow files."
            }
            Self::DependencyTriage => {
                "Classic PAT with public_repo or repo. Dependabot alert reads additionally require security_events and repository security features to be enabled."
            }
            Self::DiffReview => {
                "Shared read classic PAT with public_repo or repo. Use a separate TRUST_GATE_GITHUB_TOKEN_RW classic PAT with public_repo or repo when publishing commit statuses and maintained PR comments."
            }
            Self::MergeReadiness => {
                "Shared read classic PAT with public_repo or repo. Use a separate MERGE_KEEPER_GITHUB_TOKEN_RW classic PAT with public_repo or repo when publishing commit statuses and maintained PR comments."
            }
            Self::PrReview => {
                "Shared read classic PAT with public_repo or repo. Use a separate REVIEW_BEE_GITHUB_TOKEN_RW classic PAT with public_repo or repo when publishing maintained PR comments."
            }
            Self::RepoDiscovery => {
                "Classic PAT with public_repo (recommended for public repositories) or repo (private repositories). PatchHive constrains this credential to GitHub read requests."
            }
            Self::RepoHistory => {
                "Classic PAT with public_repo (recommended for public repositories) or repo (private repositories). PatchHive constrains this credential to GitHub read requests."
            }
            Self::ReleaseRead => {
                "Classic PAT with public_repo (recommended for public repositories) or repo (private repositories). PatchHive constrains this credential to release and repository reads."
            }
            Self::SecurityTriage => {
                "Classic PAT with public_repo or repo plus security_events for code-scanning and Dependabot alert reads."
            }
        }
    }

    pub fn ready_check(self) -> StartupCheck {
        let source = if self == Self::AutonomousWrite {
            Some(REPO_REAPER_GITHUB_TOKEN_RW)
        } else {
            github_read_token_source()
        };
        let source_note = source
            .map(|name| format!(" Credential source: {name}."))
            .unwrap_or_default();
        let legacy_note = if self != Self::AutonomousWrite && github_read_token_uses_legacy_name() {
            " This legacy variable name remains supported temporarily; migrate it to PATCHHIVE_GITHUB_TOKEN_RO."
        } else {
            ""
        };
        StartupCheck::ok(format!(
            "{}{source_note}{legacy_note}",
            self.ready_message()
        ))
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
        || lower.contains("patchhive_github_token_ro is not set")
        || lower.contains("repo_reaper_github_token_rw is not set")
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
            .contains("Dependabot alert"));
    }

    #[test]
    fn merge_readiness_separates_identity_from_target_permissions() {
        let message = GitHubPermissionProfile::MergeReadiness.ready_message();

        assert!(message.contains("GitHub accepted the shared read credential"));
        assert!(message.contains("publishing uses its separate write credential"));
        assert!(GitHubPermissionProfile::MergeReadiness
            .recommended_scopes()
            .contains("MERGE_KEEPER_GITHUB_TOKEN_RW"));
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
            "[missing_token]: PATCHHIVE_GITHUB_TOKEN_RO is not set",
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

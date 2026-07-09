use crate::startup::{StartupCheck, StartupCheckLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHubPermissionProfile {
    ActionsRead,
    DependencyTriage,
    MergeReadiness,
    PrReview,
    ReleaseRead,
    SecurityTriage,
}

impl GitHubPermissionProfile {
    pub fn ready_message(self) -> &'static str {
        match self {
            Self::ActionsRead => {
                "GitHub token detected. FlakeSting can read workflow runs and jobs with higher GitHub API rate limits."
            }
            Self::DependencyTriage => {
                "GitHub token is configured. DepTriage can read dependency PRs and Dependabot alerts when this token has access for the target repository."
            }
            Self::MergeReadiness => {
                "GitHub token detected. MergeKeeper can read PR state, review pressure, check health, and publish merge-readiness artifacts."
            }
            Self::PrReview => {
                "GitHub token detected. ReviewBee can fetch PR reviews and maintain a PR comment when requested."
            }
            Self::ReleaseRead => {
                "GitHub token detected. ReleaseSentry can read private repos, Actions history, issues, tags, releases, and changelog files."
            }
            Self::SecurityTriage => {
                "GitHub token is configured. VulnTriage will read code scanning and Dependabot alerts when this token has access for the target repository."
            }
        }
    }

    pub fn missing_message(self) -> &'static str {
        match self {
            Self::ActionsRead => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is not configured. Public-repo scans may still work, but GitHub rate limits will be much tighter."
            }
            Self::DependencyTriage => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is not configured. Public dependency PR scans may still work, but Dependabot alerts and rate limits will be weaker."
            }
            Self::MergeReadiness => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required for GitHub-backed merge readiness checks and GitHub report publishing."
            }
            Self::PrReview => {
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required for GitHub-backed review analysis."
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
            Self::DependencyTriage => {
                "Metadata (read), Pull requests (read), Dependabot alerts (read)."
            }
            Self::MergeReadiness => {
                "Metadata (read), Pull requests (read), Checks (read), Commit statuses (read). Add Checks (write), Commit statuses (write), and Issues (write) for GitHub publishing."
            }
            Self::PrReview => {
                "Metadata (read), Pull requests (read). Add Issues (write) when publishing ReviewBee maintained comments."
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
        StartupCheck::info(self.ready_message())
    }

    pub fn missing_check(self, level: StartupCheckLevel) -> StartupCheck {
        startup_check(level, self.missing_message())
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
    }
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

use serde::{Deserialize, Serialize};

/// Shared execution vocabulary for product-owned validation commands.
///
/// `Disabled` and `Skipped` are deliberately distinct from `Failed`: neither
/// means the candidate was tested and found broken. Only `Passed` is sufficient
/// to publish a non-draft autonomous change.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestExecutionStatus {
    Disabled,
    Skipped,
    Failed,
    Passed,
}

impl TestExecutionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
            Self::Passed => "passed",
        }
    }

    pub fn passed(self) -> bool {
        matches!(self, Self::Passed)
    }

    pub fn should_retry(self) -> bool {
        matches!(self, Self::Failed)
    }

    pub fn requires_draft(self) -> bool {
        !self.passed()
    }
}

#[cfg(test)]
mod tests {
    use super::TestExecutionStatus;

    #[test]
    fn only_passed_validation_allows_a_non_draft_change() {
        for status in [
            TestExecutionStatus::Disabled,
            TestExecutionStatus::Skipped,
            TestExecutionStatus::Failed,
        ] {
            assert!(status.requires_draft());
            assert!(!status.passed());
        }
        assert!(!TestExecutionStatus::Passed.requires_draft());
        assert!(TestExecutionStatus::Passed.passed());
    }

    #[test]
    fn only_executed_failures_are_retryable() {
        assert!(TestExecutionStatus::Failed.should_retry());
        assert!(!TestExecutionStatus::Disabled.should_retry());
        assert!(!TestExecutionStatus::Skipped.should_retry());
        assert!(!TestExecutionStatus::Passed.should_retry());
    }
}

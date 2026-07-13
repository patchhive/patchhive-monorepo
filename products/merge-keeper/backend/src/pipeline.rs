pub mod assessment;
pub mod routes;
pub mod utils;

pub use routes::{
    assess_github_pr, auth_status, capabilities, gen_key, gen_service_token, github_webhook,
    health, history, history_detail, login, overview, rotate_service_token, runs,
    startup_checks_route,
};

#[cfg(test)]
mod tests {
    use super::assessment::{
        apply_repo_memory_signals, apply_trust_gate_signals, current_review_counts,
    };
    use super::utils::{
        actionable_text, diff_changed_paths, mergeability_posture, MergeabilityPosture,
    };
    use crate::models::{RepoMemoryContextPreview, ReviewerState, TrustGateContext};

    #[test]
    fn actionable_text_ignores_pure_praise() {
        assert!(!actionable_text("LGTM, nice work."));
        assert!(actionable_text(
            "Could you add a test for the edge case here?"
        ));
    }

    #[test]
    fn current_review_counts_use_latest_state() {
        let states = vec![
            ReviewerState {
                login: "sam".into(),
                state: "APPROVED".into(),
                submitted_at: "1".into(),
            },
            ReviewerState {
                login: "alex".into(),
                state: "CHANGES_REQUESTED".into(),
                submitted_at: "1".into(),
            },
            ReviewerState {
                login: "lee".into(),
                state: "COMMENTED".into(),
                submitted_at: "1".into(),
            },
        ];

        let (approvals, changes_requested, comment_reviews, requesters) =
            current_review_counts(&states);
        assert_eq!(approvals, 1);
        assert_eq!(changes_requested, 1);
        assert_eq!(comment_reviews, 1);
        assert_eq!(requesters, vec!["alex".to_string()]);
    }

    #[test]
    fn diff_changed_paths_collects_unique_paths() {
        let diff = r#"
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@
+pub fn next() {}
diff --git a/tests/lib.test.rs b/tests/lib.test.rs
--- a/tests/lib.test.rs
+++ b/tests/lib.test.rs
@@
+it("works", () => {})
+++ b/src/lib.rs
"#;

        assert_eq!(
            diff_changed_paths(diff),
            vec!["src/lib.rs".to_string(), "tests/lib.test.rs".to_string()]
        );
    }

    #[test]
    fn mechanically_mergeable_blocked_state_is_a_policy_hold() {
        assert_eq!(
            mergeability_posture(Some(true), "blocked"),
            MergeabilityPosture::PolicyHold
        );
    }

    #[test]
    fn false_or_dirty_mergeability_is_a_conflict() {
        assert_eq!(
            mergeability_posture(Some(false), "blocked"),
            MergeabilityPosture::Conflict
        );
        assert_eq!(
            mergeability_posture(Some(true), "dirty"),
            MergeabilityPosture::Conflict
        );
    }

    #[test]
    fn trust_gate_block_turns_into_blocker() {
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();

        apply_trust_gate_signals(
            &mut blockers,
            &mut warnings,
            Some(&TrustGateContext {
                recommendation: "block".into(),
                summary: "High-risk change.".into(),
                risk_score: 87,
                blocked_findings: 2,
                warning_findings: 0,
                top_findings: vec!["workflow [block]: touches CI".into()],
            }),
        );

        assert_eq!(blockers.len(), 1);
        assert!(warnings.is_empty());
        assert_eq!(blockers[0].key, "trust-gate-block");
    }

    #[test]
    fn repo_memory_only_warns_on_stronger_expectations() {
        let mut warnings = Vec::new();
        apply_repo_memory_signals(
            &mut warnings,
            Some(&RepoMemoryContextPreview {
                summary: "One soft hint.".into(),
                policy_entries: 1,
                pinned_entries: 0,
                top_entries: vec!["tests: reviewers usually ask for coverage here".into()],
                ..RepoMemoryContextPreview::default()
            }),
        );
        assert!(warnings.is_empty());

        apply_repo_memory_signals(
            &mut warnings,
            Some(&RepoMemoryContextPreview {
                summary: "A promoted guardrail matched.".into(),
                failguard_warnings: vec!["Avoid the previous auth regression.".into()],
                ..RepoMemoryContextPreview::default()
            }),
        );
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].key, "failguard-guardrail");
        warnings.clear();

        apply_repo_memory_signals(
            &mut warnings,
            Some(&RepoMemoryContextPreview {
                summary: "Durable repo expectations.".into(),
                policy_entries: 2,
                pinned_entries: 0,
                top_entries: vec!["auth: require regression coverage".into()],
                ..RepoMemoryContextPreview::default()
            }),
        );
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].key, "repo-memory-policy");
    }
}

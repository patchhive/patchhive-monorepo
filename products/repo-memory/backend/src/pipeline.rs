// pipeline.rs - Main module hub for RepoMemory
// Refactored from 2568 lines into modular structure

use std::collections::HashMap;

use crate::models::MemoryEvidence;

// Type aliases used across modules
pub type JsonError = (axum::http::StatusCode, axum::Json<serde_json::Value>);
pub type JsonResult<T> = anyhow::Result<axum::Json<T>, JsonError>;

// Struct definitions used by submodules
#[derive(Clone)]
pub struct PullBundle {
    pub pr: crate::models::GitHubPullRequest,
    pub reviews: Vec<crate::models::GitHubReview>,
    pub comments: Vec<crate::models::GitHubReviewComment>,
    pub files: Vec<crate::models::GitHubPullFile>,
}

#[derive(Default)]
pub struct SignalBucket {
    pub frequency: u32,
    pub evidence: Vec<MemoryEvidence>,
}

#[derive(Default)]
pub struct ReviewerProfileBucket {
    pub total_feedback: u32,
    pub category_counts: HashMap<&'static str, u32>,
    pub path_counts: HashMap<String, u32>,
    pub evidence: Vec<MemoryEvidence>,
}

#[derive(Default)]
pub struct MaintainerProfileBucket {
    pub merged_prs: u32,
    pub source_prs: u32,
    pub source_with_tests: u32,
    pub path_counts: HashMap<String, u32>,
    pub evidence: Vec<MemoryEvidence>,
}

// Module declarations
mod context;
mod diff;
mod failguard;
mod helpers;
mod memory_run;
mod routes;
mod utils;

// Re-export public functions from submodules for backward compatibility
pub use context::disposition_rank;
pub use failguard::{
    capture_failguard_lesson, create_failguard_candidate, dismiss_failguard_candidate,
    failguard_candidates, promote_failguard_candidate,
};
pub use helpers::{build_entry, build_prompt_pack, build_summary};
pub use memory_run::truncate;
pub use routes::{
    auth_status, capabilities, context, curate_memory, gen_key, gen_service_token, health, history,
    history_detail, history_diff, ingest, known_repos, login, memories, overview, prompt_pack,
    rotate_service_token, runs, startup_checks_route,
};
pub use utils::{
    bad_request, internal_error, normalize_disposition, not_found, path_bucket, valid_repo,
    STOPWORDS,
};

#[cfg(test)]
use context::rank_context_entries;
#[cfg(test)]
use failguard::{
    build_failguard_candidate, build_failguard_lesson_run, candidate_to_lesson_request,
};

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        FailGuardCandidatePromoteRequest, FailGuardCandidateRequest, FailGuardLessonRequest,
        MemoryEntry,
    };

    fn sample_entry(kind: &str, title: &str, detail: &str, prompt_line: &str) -> MemoryEntry {
        MemoryEntry {
            id: format!("id-{kind}"),
            memory_ref: format!("ref-{kind}"),
            run_id: "run-1".into(),
            repo: "patchhive/example".into(),
            kind: kind.into(),
            title: title.into(),
            detail: detail.into(),
            prompt_line: prompt_line.into(),
            confidence: 72.0,
            frequency: 3,
            disposition: "signal".into(),
            pinned: false,
            tags: vec![kind.into()],
            evidence: Vec::new(),
            created_at: "2026-04-11T00:00:00Z".into(),
        }
    }

    #[test]
    fn repo_reaper_prefers_maintainer_profiles_when_paths_match() {
        let maintainer = sample_entry(
            "maintainer_profile",
            "Merged patterns from @alex",
            "Recent merged work from @alex clusters in src/reaper.",
            "When touching src/reaper, match the conventions that recently landed in merged work from @alex.",
        );
        let reviewer = sample_entry(
            "reviewer_profile",
            "Review patterns from @sam",
            "Past feedback from @sam repeatedly focused on tests especially around docs/.",
            "Pre-empt the kinds of feedback @sam often gives when touching docs/.",
        );

        let ranked = rank_context_entries(
            &[reviewer, maintainer],
            "repo-reaper",
            &[String::from("src/reaper/fix_worker.rs")],
            "",
            "",
            4,
        );

        assert_eq!(
            ranked.first().map(|entry| entry.kind.as_str()),
            Some("maintainer_profile")
        );
    }

    #[test]
    fn trust_gate_prefers_reviewer_profiles_when_paths_match() {
        let maintainer = sample_entry(
            "maintainer_profile",
            "Merged patterns from @alex",
            "Recent merged work from @alex clusters in src/reaper.",
            "When touching src/reaper, match the conventions that recently landed in merged work from @alex.",
        );
        let reviewer = sample_entry(
            "reviewer_profile",
            "Review patterns from @sam",
            "Past feedback from @sam repeatedly focused on validation especially around src/reaper.",
            "Pre-empt the kinds of feedback @sam often gives when touching src/reaper.",
        );

        let ranked = rank_context_entries(
            &[maintainer, reviewer],
            "trust-gate",
            &[String::from("src/reaper/fix_worker.rs")],
            "",
            "",
            4,
        );

        assert_eq!(
            ranked.first().map(|entry| entry.kind.as_str()),
            Some("reviewer_profile")
        );
    }

    #[test]
    fn pinned_policy_entries_survive_fallback_and_outrank_regular_entries() {
        let mut policy = sample_entry(
            "testing_expectation",
            "Tests are expected for auth changes",
            "Recent fixes around auth nearly always shipped with tests.",
            "Add or update tests when touching auth behavior.",
        );
        policy.disposition = "policy".into();
        policy.pinned = true;

        let regular = sample_entry(
            "review_rule",
            "Use helper builders",
            "The repo prefers shared helper builders for config wiring.",
            "Prefer shared builders over inline config duplication.",
        );

        let ranked = rank_context_entries(&[regular, policy], "trust-gate", &[], "", "", 4);

        assert_eq!(ranked.len(), 2);
        assert_eq!(
            ranked.first().map(|entry| entry.disposition.as_str()),
            Some("policy")
        );
        assert_eq!(ranked.first().map(|entry| entry.pinned), Some(true));
    }

    #[test]
    fn failguard_lesson_builds_policy_failure_memory() {
        let run = build_failguard_lesson_run(
            FailGuardLessonRequest {
                repo: "patchhive/example".into(),
                title: "Webhook secrets must fail closed".into(),
                outcome: "Unsigned webhook could trigger autonomous work.".into(),
                lesson: "Public webhook routes must not accept unsigned payloads.".into(),
                prevention: "Reject webhook delivery when the signing secret is missing.".into(),
                affected_paths: vec!["backend/src/routes/webhook.rs".into()],
                evidence: vec!["Hermes review C2".into()],
                disposition: "policy".into(),
                pinned: true,
            },
            Vec::new(),
        );

        assert_eq!(run.summary.failures, 1);
        assert_eq!(run.entries.len(), 1);
        let entry = &run.entries[0];
        assert_eq!(entry.kind, "failure_pattern");
        assert_eq!(entry.disposition, "policy");
        assert!(entry.pinned);
        assert!(entry.tags.iter().any(|tag| tag == "failguard"));
        assert!(entry
            .evidence
            .iter()
            .any(|item| item.path.as_deref() == Some("backend/src/routes/webhook.rs")));
    }

    #[test]
    fn failguard_lesson_carries_forward_existing_snapshot() {
        let existing = sample_entry(
            "testing_expectation",
            "Tests are expected for auth changes",
            "Recent fixes around auth nearly always shipped with tests.",
            "Add or update tests when touching auth behavior.",
        );

        let run = build_failguard_lesson_run(
            FailGuardLessonRequest {
                repo: "patchhive/example".into(),
                title: "Webhook secrets must fail closed".into(),
                outcome: "Unsigned webhook could trigger autonomous work.".into(),
                lesson: "Public webhook routes must not accept unsigned payloads.".into(),
                prevention: "Reject webhook delivery when the signing secret is missing.".into(),
                affected_paths: vec!["backend/src/routes/webhook.rs".into()],
                evidence: Vec::new(),
                disposition: "policy".into(),
                pinned: true,
            },
            vec![existing],
        );

        assert_eq!(run.entries.len(), 2);
        assert!(run
            .entries
            .iter()
            .any(|entry| entry.kind == "testing_expectation"));
        assert!(run
            .entries
            .iter()
            .any(|entry| entry.title == "FailGuard: Webhook secrets must fail closed"));
    }

    #[test]
    fn failguard_candidate_drafts_reviewable_lesson() {
        let candidate = build_failguard_candidate(FailGuardCandidateRequest {
            repo: "patchhive/example".into(),
            source_type: "TrustGate block".into(),
            source_ref: "review-42".into(),
            title: "Diff touched auth without tests".into(),
            outcome: "TrustGate blocked a generated patch because auth behavior changed without coverage.".into(),
            lesson: String::new(),
            prevention: String::new(),
            affected_paths: vec!["src/auth.rs".into()],
            evidence: vec!["TrustGate block #42".into()],
            confidence: None,
        });

        assert_eq!(candidate.status, "open");
        assert_eq!(candidate.source_type, "trustgate-block");
        assert_eq!(candidate.confidence, 86.0);
        assert!(candidate.lesson.contains("durable guardrail"));
        assert!(candidate.prevention.contains("src/auth.rs"));
        assert!(candidate.evidence.iter().any(|item| item == "review-42"));
    }

    #[test]
    fn failguard_candidate_promotion_allows_operator_edits() {
        let candidate = build_failguard_candidate(FailGuardCandidateRequest {
            repo: "patchhive/example".into(),
            source_type: "repo-reaper-rejection".into(),
            source_ref: "run-7".into(),
            title: "Generated patch skipped webhook signing".into(),
            outcome: "Smith rejected a patch because webhook verification failed open.".into(),
            lesson: "Webhook verification cannot be optional on public routes.".into(),
            prevention: "Reject public webhook requests when signing configuration is absent."
                .into(),
            affected_paths: vec!["backend/src/routes/webhook.rs".into()],
            evidence: vec!["Smith rejection run-7".into()],
            confidence: Some(81.0),
        });

        let lesson = candidate_to_lesson_request(
            &candidate,
            FailGuardCandidatePromoteRequest {
                title: Some("Webhook signing must fail closed".into()),
                prevention: Some("Return 403 when webhook signing is unavailable.".into()),
                disposition: "policy".into(),
                pinned: true,
                ..Default::default()
            },
        );

        assert_eq!(lesson.title, "Webhook signing must fail closed");
        assert_eq!(lesson.outcome, candidate.outcome);
        assert_eq!(lesson.lesson, candidate.lesson);
        assert_eq!(
            lesson.prevention,
            "Return 403 when webhook signing is unavailable."
        );
        assert_eq!(lesson.affected_paths, candidate.affected_paths);
        assert_eq!(lesson.disposition, "policy");
        assert!(lesson.pinned);
    }

    #[test]
    fn suppressed_entries_are_filtered_out_of_context_results() {
        let mut suppressed = sample_entry(
            "failure_pattern",
            "Old flaky pattern",
            "A noisy signal that operators intentionally suppressed.",
            "Ignore this pattern.",
        );
        suppressed.disposition = "suppressed".into();

        let ranked = rank_context_entries(
            &[suppressed],
            "repo-reaper",
            &["src/lib.rs".into()],
            "",
            "",
            4,
        );
        assert!(ranked.is_empty());
    }
}

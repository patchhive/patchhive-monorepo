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
    backfill_promoted_guardrails, capture_failguard_lesson, create_failguard_candidate,
    dismiss_failguard_candidate, failguard_candidates, failguard_guardrails, failguard_matches,
    promote_failguard_candidate,
};
pub use helpers::{build_entry, build_prompt_pack, build_summary, EntryDraft};
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
    compile_failguard_guardrail,
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

    fn sample_issue(number: u32, title: &str, body: &str) -> crate::models::GitHubIssue {
        crate::models::GitHubIssue {
            number,
            title: title.into(),
            body: Some(body.into()),
            ..crate::models::GitHubIssue::default()
        }
    }

    fn sample_bundle(number: u32, files: &[&str]) -> PullBundle {
        PullBundle {
            pr: crate::models::GitHubPullRequest {
                number,
                title: format!("Merged change {number}"),
                html_url: format!("https://github.com/patchhive/example/pull/{number}"),
                ..crate::models::GitHubPullRequest::default()
            },
            reviews: Vec::new(),
            comments: Vec::new(),
            files: files
                .iter()
                .map(|filename| crate::models::GitHubPullFile {
                    filename: (*filename).into(),
                    ..crate::models::GitHubPullFile::default()
                })
                .collect(),
        }
    }

    #[test]
    fn hotspots_count_distinct_pull_requests_and_keep_evidence() {
        let run = memory_run::build_memory_run(
            crate::models::IngestParams {
                repo: "patchhive/example".into(),
                ..crate::models::IngestParams::default()
            },
            vec![
                sample_bundle(
                    1,
                    &[
                        "apps/desktop/controller.tsx",
                        "apps/desktop/panel.tsx",
                        "tests/desktop/controller.test.tsx",
                    ],
                ),
                sample_bundle(
                    2,
                    &[
                        "apps/desktop/state.ts",
                        "apps/desktop/view.tsx",
                        "tests/desktop/state.test.ts",
                    ],
                ),
                sample_bundle(3, &["apps/desktop/routes.ts"]),
            ],
            Vec::new(),
            0,
        )
        .expect("memory run should build");

        let hotspot = run
            .entries
            .iter()
            .find(|entry| entry.kind == "hotspot" && entry.title.contains("apps/desktop"))
            .expect("apps/desktop hotspot should exist");
        assert_eq!(hotspot.frequency, 3);
        assert_eq!(hotspot.evidence.len(), 3);

        let testing = run
            .entries
            .iter()
            .find(|entry| entry.kind == "testing_expectation")
            .expect("testing expectation should exist");
        assert_eq!(testing.frequency, 2);
        assert_eq!(testing.evidence.len(), 2);
    }

    #[test]
    fn reviewer_profiles_keep_the_feedback_that_created_them() {
        let mut bundle = sample_bundle(4, &["src/worker.rs"]);
        bundle.comments = vec![
            crate::models::GitHubReviewComment {
                body: "Please add tests for this edge case.".into(),
                html_url: "https://github.com/patchhive/example/pull/4#discussion-1".into(),
                path: Some("src/worker.rs".into()),
                user: Some(patchhive_github_data::models::GitHubUser {
                    login: "reviewer".into(),
                }),
                ..crate::models::GitHubReviewComment::default()
            },
            crate::models::GitHubReviewComment {
                body: "Reuse the existing shared helper.".into(),
                html_url: "https://github.com/patchhive/example/pull/4#discussion-2".into(),
                path: Some("src/worker.rs".into()),
                user: Some(patchhive_github_data::models::GitHubUser {
                    login: "reviewer".into(),
                }),
                ..crate::models::GitHubReviewComment::default()
            },
        ];

        let run = memory_run::build_memory_run(
            crate::models::IngestParams {
                repo: "patchhive/example".into(),
                ..crate::models::IngestParams::default()
            },
            vec![bundle],
            Vec::new(),
            0,
        )
        .expect("memory run should build");

        let profile = run
            .entries
            .iter()
            .find(|entry| entry.kind == "reviewer_profile")
            .expect("reviewer profile should exist");
        assert_eq!(profile.frequency, 2);
        assert_eq!(profile.evidence.len(), 2);
        assert!(profile
            .evidence
            .iter()
            .all(|evidence| evidence.source_type == "review_feedback"));
    }

    #[test]
    fn prompt_pack_separates_operator_policy_from_inferred_signals() {
        let mut policy = sample_entry(
            "testing_expectation",
            "Behavior changes ship with tests",
            "Tests are expected.",
            "Update tests with behavior changes.",
        );
        policy.disposition = "policy".into();
        policy.pinned = true;
        let signal = sample_entry(
            "review_rule",
            "Prefer existing helpers",
            "Reviewers request shared helpers.",
            "Prefer existing helpers before adding one-off logic.",
        );
        let entries = vec![policy, signal];
        let summary = build_summary(&entries, 3, 2, 1, 0);

        let prompt_pack = build_prompt_pack("patchhive/example", &summary, &entries);

        assert!(prompt_pack.contains("## Operator policies"));
        assert!(prompt_pack.contains("**[Pinned policy]** Update tests"));
        assert!(prompt_pack.contains("## Conventions and review habits"));
        assert_eq!(
            prompt_pack
                .matches("Update tests with behavior changes.")
                .count(),
            1
        );
    }

    #[test]
    fn feature_templates_do_not_become_bug_evidence() {
        let issue = sample_issue(
            1,
            "[Feature]: Add another provider",
            "The proposed response should include better error details.",
        );

        assert!(!memory_run::looks_bug_like(&issue));
    }

    #[test]
    fn strong_failure_language_still_identifies_vague_bug_reports() {
        let issue = sample_issue(
            2,
            "Worker occasionally stops",
            "The process deadlocks after the second background refresh.",
        );

        assert!(memory_run::looks_bug_like(&issue));
    }

    #[test]
    fn repo_name_and_template_words_do_not_become_failure_memories() {
        let run = memory_run::build_memory_run(
            crate::models::IngestParams {
                repo: "NousResearch/hermes-agent".into(),
                ..crate::models::IngestParams::default()
            },
            Vec::new(),
            vec![
                sample_issue(
                    3,
                    "bug: Hermes environment path tool proposed response",
                    "Launch fails.",
                ),
                sample_issue(
                    4,
                    "bug: Hermes environment path tool proposed response",
                    "Connection fails.",
                ),
            ],
            0,
        )
        .expect("memory run should build");

        assert!(run
            .entries
            .iter()
            .all(|entry| entry.kind != "failure_pattern"));
    }

    #[test]
    fn repeated_issue_title_topics_remain_failure_memories() {
        let run = memory_run::build_memory_run(
            crate::models::IngestParams {
                repo: "patchhive/example".into(),
                ..crate::models::IngestParams::default()
            },
            Vec::new(),
            vec![
                sample_issue(5, "bug: sandbox exits early", "The worker crashes."),
                sample_issue(6, "bug: sandbox rejects valid path", "The worker fails."),
            ],
            0,
        )
        .expect("memory run should build");

        assert!(run
            .entries
            .iter()
            .any(|entry| { entry.kind == "failure_pattern" && entry.title.contains("sandbox") }));
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
    fn failguard_candidate_status_preserves_all_filter() {
        assert_eq!(utils::normalize_candidate_status("all"), "all");
        assert_eq!(utils::normalize_candidate_status("dismissed"), "dismissed");
        assert_eq!(utils::normalize_candidate_status("unexpected"), "open");
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
    fn promoted_failguard_lesson_compiles_all_consumer_suggestions() {
        let request = FailGuardLessonRequest {
            repo: "patchhive/example".into(),
            title: "Prevent unsigned webhook dispatch".into(),
            outcome: "An unsigned webhook reached product logic.".into(),
            lesson: "Webhook authentication must fail closed.".into(),
            prevention: "Verify the signature before dispatch.".into(),
            affected_paths: vec!["backend/src/webhook.rs".into()],
            evidence: vec!["run-123".into()],
            disposition: "policy".into(),
            pinned: true,
        };
        let run = build_failguard_lesson_run(request.clone(), Vec::new());
        let entry = run
            .entries
            .iter()
            .find(|entry| entry.kind == "failure_pattern")
            .expect("failure lesson should create an entry");
        let guardrail = compile_failguard_guardrail(&request, entry, "candidate-1");

        assert_eq!(guardrail.suggestions.len(), 4);
        assert_eq!(guardrail.candidate_id, "candidate-1");
        assert!(guardrail.suggestions.iter().any(
            |suggestion| suggestion.consumer == "trust-gate" && suggestion.severity == "block"
        ));
        assert!(guardrail
            .suggestions
            .iter()
            .any(|suggestion| suggestion.consumer == "repo-reaper"
                && suggestion.kind == "preflight-constraint"));
        assert!(guardrail
            .suggestions
            .iter()
            .any(|suggestion| suggestion.consumer == "merge-keeper"));
        assert!(guardrail
            .suggestions
            .iter()
            .any(|suggestion| suggestion.consumer == "release-sentry"));
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

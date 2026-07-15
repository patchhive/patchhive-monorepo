// memory_run.rs - Memory run building and related helpers

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use crate::models::{IngestParams, IngestRecord, MemoryEvidence};

// Use shared helpers from parent module
use super::{
    build_entry, build_prompt_pack, build_summary, path_bucket, EntryDraft,
    MaintainerProfileBucket, ReviewerProfileBucket, SignalBucket, STOPWORDS,
};

pub fn build_memory_run(
    params: IngestParams,
    bundles: Vec<super::PullBundle>,
    issues: Vec<crate::models::GitHubIssue>,
    partial_read_warnings: u32,
) -> Result<IngestRecord> {
    let run_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let repo = params.repo.clone();

    let mut entries = Vec::new();
    let mut review_buckets: HashMap<&'static str, SignalBucket> = HashMap::new();
    let mut hotspot_buckets: HashMap<String, SignalBucket> = HashMap::new();
    let mut file_review_counts: HashMap<String, SignalBucket> = HashMap::new();
    let mut bug_terms: HashMap<String, SignalBucket> = HashMap::new();
    let mut reviewer_profiles: HashMap<String, ReviewerProfileBucket> = HashMap::new();
    let mut maintainer_profiles: HashMap<String, MaintainerProfileBucket> = HashMap::new();
    let mut source_prs = 0u32;
    let mut source_with_tests = 0u32;
    let mut testing_pattern_evidence = Vec::new();
    let mut review_feedback_items = 0u32;
    let repo_tokens = tokenize(&repo);

    for bundle in &bundles {
        let mut touched_source = false;
        let mut touched_tests = false;
        let mut touched_dirs = HashSet::new();
        let author_login = bundle
            .pr
            .user
            .as_ref()
            .map(|user| user.login.trim().to_string())
            .filter(|login| !login.is_empty());

        for file in &bundle.files {
            if is_source_file(&file.filename) {
                touched_source = true;
            }
            if is_test_file(&file.filename) {
                touched_tests = true;
            }

            touched_dirs.insert(path_bucket(&file.filename));
            if let Some(author) = author_login.as_ref() {
                let profile = maintainer_profiles.entry(author.clone()).or_default();
                let path_count = profile
                    .path_counts
                    .entry(path_bucket(&file.filename))
                    .or_insert(0);
                *path_count += 1;
            }
        }

        for dir in touched_dirs {
            let bucket = hotspot_buckets.entry(dir.clone()).or_default();
            bucket.frequency += 1;
            push_evidence(
                &mut bucket.evidence,
                MemoryEvidence {
                    source_type: "merged_pr".into(),
                    title: format!("#{} {}", bundle.pr.number, bundle.pr.title),
                    url: bundle.pr.html_url.clone(),
                    path: Some(dir.clone()),
                    excerpt: format!("Merged PR touched {dir}."),
                },
            );
        }

        if let Some(author) = author_login.as_ref() {
            let profile = maintainer_profiles.entry(author.clone()).or_default();
            profile.merged_prs += 1;
            if touched_source {
                profile.source_prs += 1;
                if touched_tests {
                    profile.source_with_tests += 1;
                }
            }
            push_evidence(
                &mut profile.evidence,
                MemoryEvidence {
                    source_type: "merged_pr".into(),
                    title: format!("#{} {}", bundle.pr.number, bundle.pr.title),
                    url: bundle.pr.html_url.clone(),
                    path: None,
                    excerpt: truncate(
                        &format!(
                            "{} merged with {} changed files.",
                            author,
                            bundle.pr.changed_files.unwrap_or(bundle.files.len() as u32)
                        ),
                        180,
                    ),
                },
            );
        }

        if touched_source {
            source_prs += 1;
            if touched_tests {
                source_with_tests += 1;
                push_evidence(
                    &mut testing_pattern_evidence,
                    MemoryEvidence {
                        source_type: "merged_pr".into(),
                        title: format!("#{} {}", bundle.pr.number, bundle.pr.title),
                        url: bundle.pr.html_url.clone(),
                        path: None,
                        excerpt: "Merged PR changed source files and tests together.".into(),
                    },
                );
            }
        }

        for review in &bundle.reviews {
            if review.state.eq_ignore_ascii_case("commented")
                || review.state.eq_ignore_ascii_case("changes_requested")
                || review.state.eq_ignore_ascii_case("approved")
            {
                if let Some(body) = review.body.as_deref() {
                    review_feedback_items += collect_feedback(
                        &mut review_buckets,
                        &mut reviewer_profiles,
                        &bundle.pr,
                        None,
                        review.html_url.as_deref().unwrap_or(&bundle.pr.html_url),
                        body,
                        review.user.as_ref().map(|user| user.login.as_str()),
                    );
                }
            }
        }

        for comment in &bundle.comments {
            review_feedback_items += collect_feedback(
                &mut review_buckets,
                &mut reviewer_profiles,
                &bundle.pr,
                comment.path.as_deref(),
                &comment.html_url,
                &comment.body,
                comment.user.as_ref().map(|user| user.login.as_str()),
            );

            if let Some(path) = comment.path.as_deref() {
                let entry = file_review_counts.entry(path.to_string()).or_default();
                entry.frequency += 1;
                push_evidence(
                    &mut entry.evidence,
                    MemoryEvidence {
                        source_type: "review_comment".into(),
                        title: format!("#{} {}", bundle.pr.number, bundle.pr.title),
                        url: comment.html_url.clone(),
                        path: Some(path.to_string()),
                        excerpt: truncate(comment.body.trim(), 180),
                    },
                );
            }
        }
    }

    for issue in &issues {
        if !looks_bug_like(issue) {
            continue;
        }
        let title_tokens = tokenize(&issue.title);

        for token in title_tokens {
            if token.len() < 4
                || STOPWORDS.contains(&token.as_str())
                || repo_tokens.contains(&token)
            {
                continue;
            }
            let bucket = bug_terms.entry(token.clone()).or_default();
            bucket.frequency += 1;
            push_evidence(
                &mut bucket.evidence,
                MemoryEvidence {
                    source_type: "issue".into(),
                    title: format!("#{} {}", issue.number, issue.title),
                    url: issue.html_url.clone(),
                    path: None,
                    excerpt: truncate(issue.body.as_deref().unwrap_or(issue.title.as_str()), 180),
                },
            );
        }
    }

    for (key, label, prompt_line, detail_line, tags) in review_bucket_specs() {
        if let Some(bucket) = review_buckets.get(key) {
            if bucket.frequency < 2 {
                continue;
            }
            entries.push(build_entry(
                &run_id,
                &repo,
                &created_at,
                EntryDraft {
                    kind: "review_rule",
                    title: label.into(),
                    detail: format!("{detail_line} Reviewer feedback surfaced this pattern {} times across recent merged PRs.", bucket.frequency),
                    prompt_line: prompt_line.into(),
                    frequency: bucket.frequency,
                    tags,
                    evidence: bucket.evidence.clone(),
                },
            ));
        }
    }

    if source_prs >= 3 && source_with_tests >= 2 {
        let ratio = source_with_tests as f64 / source_prs as f64;
        if ratio >= 0.5 {
            entries.push(build_entry(
                &run_id,
                &repo,
                &created_at,
                EntryDraft {
                    kind: "testing_expectation",
                    title: "Behavior changes usually ship with tests".into(),
                    detail: format!(
                        "{} of the last {} merged PRs that touched source files also updated tests. This repo tends to expect test coverage when behavior changes.",
                        source_with_tests, source_prs
                    ),
                    prompt_line: "When behavior changes or bugs are fixed, update or add tests in the same patch.".into(),
                    frequency: source_with_tests,
                    tags: vec!["tests", "merged-pr-pattern"],
                    evidence: testing_pattern_evidence,
                },
            ));
        }
    }

    let mut hotspot_dirs: Vec<_> = hotspot_buckets.into_iter().collect();
    hotspot_dirs.sort_by(|left, right| {
        right
            .1
            .frequency
            .cmp(&left.1.frequency)
            .then_with(|| left.0.cmp(&right.0))
    });
    for (dir, bucket) in hotspot_dirs
        .into_iter()
        .filter(|(_, bucket)| bucket.frequency >= 2)
        .take(3)
    {
        entries.push(build_entry(
            &run_id,
            &repo,
            &created_at,
            EntryDraft {
                kind: "hotspot",
                title: format!("Recent fixes cluster in {dir}"),
                detail: format!(
                    "Recent merged PRs repeatedly touched {dir}. Treat it as a high-context area and read nearby helpers, tests, and conventions before changing it."
                ),
                prompt_line: format!("Treat {dir} as a high-context area; read nearby code and tests before editing it."),
                frequency: bucket.frequency,
                tags: vec!["hotspot", "paths"],
                evidence: bucket.evidence,
            },
        ));
    }

    let mut review_paths: Vec<_> = file_review_counts.into_iter().collect();
    review_paths.sort_by(|left, right| {
        right
            .1
            .frequency
            .cmp(&left.1.frequency)
            .then_with(|| left.0.cmp(&right.0))
    });
    for (path, bucket) in review_paths
        .into_iter()
        .filter(|(_, bucket)| bucket.frequency >= 2)
        .take(3)
    {
        entries.push(build_entry(
            &run_id,
            &repo,
            &created_at,
            EntryDraft {
                kind: "hotspot",
                title: format!("{path} attracts repeat review churn"),
                detail: format!(
                    "Reviewer comments keep landing on {path}. This file or area likely encodes conventions that agents should read before making edits."
                ),
                prompt_line: format!("Read {path} carefully before editing; this path attracts repeat review feedback."),
                frequency: bucket.frequency,
                tags: vec!["review-churn", "paths"],
                evidence: bucket.evidence,
            },
        ));
    }

    let mut bug_terms: Vec<_> = bug_terms.into_iter().collect();
    bug_terms.sort_by(|left, right| {
        right
            .1
            .frequency
            .cmp(&left.1.frequency)
            .then_with(|| left.0.cmp(&right.0))
    });
    for (term, bucket) in bug_terms
        .into_iter()
        .filter(|(_, bucket)| bucket.frequency >= 2)
        .take(4)
    {
        let topic = failure_topic_label(&term);
        entries.push(build_entry(
            &run_id,
            &repo,
            &created_at,
            EntryDraft {
                kind: "failure_pattern",
                title: format!("Recurring failures mention '{term}'"),
                detail: format!(
                    "Closed bug reports repeatedly mention {term}. RepoMemory is treating that as a repeated failure pattern worth checking before new patches move forward."
                ),
                prompt_line: format!(
                    "Verify behavior and edge cases related to {topic} before finalizing a patch."
                ),
                frequency: bucket.frequency,
                tags: vec!["bugs", "issues", "failure-pattern"],
                evidence: bucket.evidence,
            },
        ));
    }

    let mut reviewer_profiles: Vec<_> = reviewer_profiles.into_iter().collect();
    reviewer_profiles.sort_by(|left, right| {
        right
            .1
            .total_feedback
            .cmp(&left.1.total_feedback)
            .then_with(|| left.0.cmp(&right.0))
    });
    for (reviewer, profile) in reviewer_profiles
        .into_iter()
        .filter(|(_, profile)| profile.total_feedback >= 2)
        .take(4)
    {
        let focus = top_named_counts(&profile.category_counts, 2)
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        let paths = top_string_counts(&profile.path_counts, 2)
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>();
        let focus_text = if focus.is_empty() {
            "general review consistency".into()
        } else {
            focus.join(" and ")
        };
        let path_text = if paths.is_empty() {
            "across the repo".into()
        } else {
            format!("especially around {}", paths.join(" and "))
        };

        entries.push(build_entry(
            &run_id,
            &repo,
            &created_at,
            EntryDraft {
                kind: "reviewer_profile",
                title: format!("Review patterns from @{reviewer}"),
                detail: format!(
                    "Past feedback from @{reviewer} repeatedly focused on {focus_text} {path_text}. RepoMemory is keeping that signature so future patches can pre-empt the same review friction."
                ),
                prompt_line: format!(
                    "Pre-empt the kinds of feedback @{reviewer} often gives around {focus_text} {}.",
                    if paths.is_empty() {
                        "before you ship changes".into()
                    } else {
                        format!("when touching {}", paths.join(" and "))
                    }
                ),
                frequency: profile.total_feedback,
                tags: vec!["reviewer-profile", "feedback-signature"],
                evidence: profile.evidence,
            },
        ));
    }

    let mut maintainer_profiles: Vec<_> = maintainer_profiles.into_iter().collect();
    maintainer_profiles.sort_by(|left, right| {
        right
            .1
            .merged_prs
            .cmp(&left.1.merged_prs)
            .then_with(|| left.0.cmp(&right.0))
    });
    for (author, profile) in maintainer_profiles
        .into_iter()
        .filter(|(_, profile)| profile.merged_prs >= 2)
        .take(4)
    {
        let paths = top_string_counts(&profile.path_counts, 2)
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>();
        let test_ratio = if profile.source_prs > 0 {
            profile.source_with_tests as f64 / profile.source_prs as f64
        } else {
            0.0
        };
        let path_text = if paths.is_empty() {
            "across the repo".into()
        } else {
            paths.join(" and ")
        };
        let test_text = if profile.source_prs >= 2 && test_ratio >= 0.5 {
            "Recent merged work from this author usually pairs source changes with tests."
        } else {
            "Recent merged work from this author mostly signals where accepted patterns keep landing."
        };

        entries.push(build_entry(
            &run_id,
            &repo,
            &created_at,
            EntryDraft {
                kind: "maintainer_profile",
                title: format!("Merged patterns from @{author}"),
                detail: format!(
                    "Recent merged work from @{author} clusters in {path_text}. {test_text}"
                ),
                prompt_line: format!(
                    "When touching {path_text}, match the conventions that recently landed in merged work from @{author}."
                ),
                frequency: profile.merged_prs,
                tags: vec!["maintainer-profile", "merged-history"],
                evidence: profile.evidence,
            },
        ));
    }

    entries.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.frequency.cmp(&left.frequency))
    });

    let summary = build_summary(
        &entries,
        bundles.len() as u32,
        review_feedback_items,
        issues.len() as u32,
        partial_read_warnings,
    );
    let prompt_pack = build_prompt_pack(&repo, &summary, &entries);

    Ok(IngestRecord {
        id: run_id,
        repo,
        created_at,
        params,
        summary,
        prompt_pack,
        entries,
    })
}

// Helper functions

pub fn is_source_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let source_like = [
        ".rs", ".js", ".jsx", ".ts", ".tsx", ".py", ".go", ".java", ".kt", ".rb", ".php", ".c",
        ".cc", ".cpp", ".h", ".hpp",
    ];
    source_like.iter().any(|ext| lower.ends_with(ext)) && !is_test_file(path)
}

pub fn is_test_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("/test")
        || lower.contains("/tests/")
        || lower.contains("__tests__")
        || lower.contains(".spec.")
        || lower.contains(".test.")
}

pub fn looks_bug_like(issue: &crate::models::GitHubIssue) -> bool {
    let title_and_labels = format!(
        "{} {}",
        issue.title,
        issue
            .labels
            .iter()
            .map(|label| label.name.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    )
    .to_ascii_lowercase();

    let explicitly_feature_like = contains_any(
        &title_and_labels,
        &["feature", "enhancement", "proposal", "documentation"],
    );
    let explicitly_bug_like = contains_any(
        &title_and_labels,
        &[
            "bug",
            "regression",
            "panic",
            "crash",
            "timeout",
            "failure",
            "failing",
            "broken",
            "error",
            "race",
            "leak",
        ],
    );
    if explicitly_feature_like && !explicitly_bug_like {
        return false;
    }
    if explicitly_bug_like {
        return true;
    }

    let body = issue
        .body
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    contains_any(
        &body,
        &[
            "regression",
            "panic",
            "crash",
            "timeout",
            "stack trace",
            "exception",
            "race condition",
            "deadlock",
            "data loss",
            "leak",
        ],
    )
}

pub fn tokenize(text: &str) -> HashSet<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| part.len() >= 4)
        .collect()
}

pub fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

pub fn split_feedback_sentences(body: &str) -> Vec<String> {
    body.replace('\r', "\n")
        .split(['\n', '.', '!', '?'])
        .map(str::trim)
        .filter(|part| part.len() >= 18)
        .map(str::to_string)
        .collect()
}

pub fn push_evidence(target: &mut Vec<MemoryEvidence>, evidence: MemoryEvidence) {
    if target.len() < 4 {
        target.push(evidence);
    }
}

pub fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let truncated: String = value.chars().take(limit.saturating_sub(1)).collect();
    format!("{truncated}…")
}

pub fn collect_feedback(
    buckets: &mut HashMap<&'static str, SignalBucket>,
    reviewer_profiles: &mut HashMap<String, ReviewerProfileBucket>,
    pr: &crate::models::GitHubPullRequest,
    path: Option<&str>,
    url: &str,
    body: &str,
    author: Option<&str>,
) -> u32 {
    let mut matched = 0;
    for sentence in split_feedback_sentences(body) {
        let Some((bucket_key, _label)) = classify_feedback(&sentence) else {
            continue;
        };
        let evidence = MemoryEvidence {
            source_type: "review_feedback".into(),
            title: format!("#{} {}", pr.number, pr.title),
            url: url.to_string(),
            path: path.map(str::to_string),
            excerpt: if let Some(author) = author {
                format!("{author}: {}", truncate(&sentence, 180))
            } else {
                truncate(&sentence, 180)
            },
        };
        let bucket = buckets.entry(bucket_key).or_default();
        bucket.frequency += 1;
        push_evidence(&mut bucket.evidence, evidence.clone());
        if let Some(author) = author.filter(|value| !value.trim().is_empty()) {
            let profile = reviewer_profiles.entry(author.to_string()).or_default();
            profile.total_feedback += 1;
            *profile.category_counts.entry(bucket_key).or_insert(0) += 1;
            if let Some(path) = path {
                *profile.path_counts.entry(path_bucket(path)).or_insert(0) += 1;
            }
            push_evidence(&mut profile.evidence, evidence);
        }
        matched += 1;
    }
    matched
}

pub fn review_bucket_specs() -> Vec<(
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Vec<&'static str>,
)> {
    vec![
        (
            "tests",
            "Reviewers repeatedly ask for tests",
            "Add or update tests when behavior changes, bugs are fixed, or risky code is touched.",
            "Repo reviewers regularly ask for stronger test coverage before merge.",
            vec!["tests", "review-feedback"],
        ),
        (
            "helpers",
            "Reviewers prefer existing helpers over one-off logic",
            "Prefer existing helpers, shared utilities, and established abstractions before adding one-off logic.",
            "Review feedback keeps steering changes back toward shared helpers and existing abstractions.",
            vec!["helpers", "conventions"],
        ),
        (
            "validation",
            "Boundary checks and validation matter here",
            "Preserve guard rails, input validation, and edge-case handling around boundaries.",
            "Reviewer feedback repeatedly calls out missing validation, guards, or edge-case handling.",
            vec!["validation", "safety"],
        ),
        (
            "naming",
            "Consistency beats clever naming in this repo",
            "Match existing naming, file placement, and structural conventions before inventing a new pattern.",
            "Reviewer feedback keeps reinforcing local naming and structure conventions.",
            vec!["naming", "style"],
        ),
        (
            "docs",
            "Docs and supporting context are expected alongside behavior changes",
            "Keep docs, comments, or README context in sync when interfaces or behavior change.",
            "Recent reviews repeatedly ask for docs, comments, or supporting context updates.",
            vec!["docs", "maintenance"],
        ),
        (
            "errors",
            "Error handling needs context, not just a happy path",
            "Preserve repo error-handling patterns and include context-rich failures or logging where expected.",
            "Reviewer feedback regularly calls out missing context, logging, or error-handling consistency.",
            vec!["errors", "operability"],
        ),
    ]
}

fn failure_topic_label(term: &str) -> &str {
    match term {
        "windows" => "Windows",
        _ => term,
    }
}

pub fn classify_feedback(sentence: &str) -> Option<(&'static str, &'static str)> {
    let lower = sentence.to_ascii_lowercase();
    if contains_any(&lower, &["test", "coverage", "assert", "spec"]) {
        return Some(("tests", "tests"));
    }
    if contains_any(
        &lower,
        &["helper", "utility", "shared", "common", "existing", "reuse"],
    ) {
        return Some(("helpers", "helpers"));
    }
    if contains_any(
        &lower,
        &[
            "validate",
            "validation",
            "guard",
            "sanitize",
            "check for",
            "edge case",
        ],
    ) {
        return Some(("validation", "validation"));
    }
    if contains_any(
        &lower,
        &[
            "rename",
            "naming",
            "consistent",
            "convention",
            "style",
            "pattern",
        ],
    ) {
        return Some(("naming", "naming"));
    }
    if contains_any(
        &lower,
        &["readme", "docs", "document", "comment", "changelog"],
    ) {
        return Some(("docs", "docs"));
    }
    if contains_any(
        &lower,
        &["error", "logging", "log ", "context", "fallback", "retry"],
    ) {
        return Some(("errors", "errors"));
    }
    None
}

pub fn category_label(key: &str) -> String {
    match key {
        "tests" => "tests".into(),
        "helpers" => "shared helpers".into(),
        "validation" => "validation".into(),
        "naming" => "naming and structure".into(),
        "docs" => "docs and supporting context".into(),
        "errors" => "error handling".into(),
        _ => key.to_string(),
    }
}

pub fn top_named_counts(counts: &HashMap<&'static str, u32>, limit: usize) -> Vec<(String, u32)> {
    let mut items = counts
        .iter()
        .map(|(name, count)| (category_label(name), *count))
        .collect::<Vec<_>>();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    items.truncate(limit);
    items
}

pub fn top_string_counts(counts: &HashMap<String, u32>, limit: usize) -> Vec<(String, u32)> {
    let mut items = counts
        .iter()
        .map(|(name, count)| (name.clone(), *count))
        .collect::<Vec<_>>();
    items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    items.truncate(limit);
    items
}

// review.rs — Core diff review engine

use chrono::Utc;
use patchhive_product_core::repo_memory::{fetch_repo_memory_context, RepoMemoryContextRequest};
use tracing::warn;
use uuid::Uuid;

use crate::models::{
    FileAssessment, GitHubReviewContext, RepoRuleSet, ReviewMetricSummary, ReviewResult,
};

use super::types::{
    clamp_score, is_docs_only_path, is_generated_path, limit_examples, make_finding,
    matching_patterns, parse_diff, path_policy_note, summarize_diff_for_memory,
    summarize_review_task,
};

fn count_phrase(count: u32, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

fn decision_adjusted_score(raw_score: u32, recommendation: &str) -> u32 {
    match recommendation {
        "block" => raw_score.max(70),
        "warn" => raw_score.max(25),
        _ => raw_score,
    }
}

fn credential_prefix_match(line: &str, prefix: &str, minimum_payload: usize) -> bool {
    let lower_line = line.to_ascii_lowercase();
    let lower_prefix = prefix.to_ascii_lowercase();
    lower_line.match_indices(&lower_prefix).any(|(index, _)| {
        let boundary_ok = index == 0
            || lower_line[..index]
                .chars()
                .next_back()
                .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_');
        if !boundary_ok {
            return false;
        }
        lower_line[index + lower_prefix.len()..]
            .chars()
            .take_while(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
            })
            .count()
            >= minimum_payload
    })
}

fn blocked_term_matches(line: &str, term: &str) -> bool {
    match term.trim().to_ascii_lowercase().as_str() {
        "sk-" | "ghp_" | "github_pat_" | "akia" => credential_prefix_match(line, term, 8),
        _ => line
            .to_ascii_lowercase()
            .contains(&term.to_ascii_lowercase()),
    }
}

fn marker_term_matches(line: &str, marker: &str) -> bool {
    line.match_indices(marker).any(|(index, _)| {
        let before_ok = index == 0
            || line[..index]
                .chars()
                .next_back()
                .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_');
        let after_index = index + marker.len();
        let after_ok = after_index == line.len()
            || line[after_index..]
                .chars()
                .next()
                .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_');
        before_ok && after_ok
    })
}

fn hardcoded_sensitive_value_matches(line: &str, term: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    ['=', ':'].into_iter().any(|delimiter| {
        let Some((left, right)) = lower.split_once(delimiter) else {
            return false;
        };
        if !left.contains(term) {
            return false;
        }
        let value = right.trim_start();
        matches!(value.chars().next(), Some('"' | '\'' | '`'))
            && value
                .trim_matches(|character| matches!(character, '"' | '\'' | '`' | ' ' | '\t'))
                .len()
                >= 3
    })
}

fn suspicious_term_matches(line: &str, term: &str) -> bool {
    match term.trim() {
        "TODO" | "FIXME" => marker_term_matches(line, term),
        value
            if matches!(
                value.to_ascii_lowercase().as_str(),
                "token" | "secret" | "password"
            ) =>
        {
            hardcoded_sensitive_value_matches(line, &value.to_ascii_lowercase())
        }
        _ => line
            .to_ascii_lowercase()
            .contains(&term.to_ascii_lowercase()),
    }
}

pub async fn review_diff(
    client: &reqwest::Client,
    repo: &str,
    diff: &str,
    ai_source: &str,
    rules: RepoRuleSet,
    source_kind: &str,
    github_context: Option<GitHubReviewContext>,
) -> ReviewResult {
    let patches = parse_diff(diff);
    let changed_paths: Vec<String> = patches.iter().map(|patch| patch.path.clone()).collect();
    let diff_summary = summarize_diff_for_memory(&patches);
    let task_summary = summarize_review_task(repo, ai_source, source_kind, github_context.as_ref());
    let repo_memory_context = match fetch_repo_memory_context(
        client,
        &RepoMemoryContextRequest {
            repo: repo.to_string(),
            consumer: "trust-gate".into(),
            changed_paths: changed_paths.clone(),
            task_summary,
            diff_summary,
            limit: 5,
        },
    )
    .await
    {
        Ok(context) => context,
        Err(err) => {
            warn!("RepoMemory context lookup failed for {repo}: {err}");
            None
        }
    };
    let files_changed = patches.len() as u32;
    let additions = patches.iter().map(|patch| patch.additions).sum::<u32>();
    let deletions = patches.iter().map(|patch| patch.deletions).sum::<u32>();

    let mut findings = Vec::new();
    let mut files = Vec::new();
    let mut blocked_path_hits = Vec::new();
    let mut warn_path_hits = Vec::new();
    let mut blocked_term_hits = Vec::new();
    let mut suspicious_term_hits = Vec::new();
    let mut policy_hits = Vec::new();
    let mut generated_hits = Vec::new();
    let mut tests_changed = 0u32;
    let mut source_files_changed = 0u32;
    let mut generated_files = 0u32;
    let mut test_required_paths = Vec::new();
    let mut sensitive_code_changes = 0u32;

    for patch in patches {
        let generated = is_generated_path(&patch.path);
        let docs_only = is_docs_only_path(&patch.path);
        let path_policy = path_policy_note(&patch.path).unwrap_or("").to_string();
        let touches_test_path = !matching_patterns(&patch.path, &rules.test_paths).is_empty();
        let requires_tests = !generated
            && !docs_only
            && !touches_test_path
            && !matching_patterns(&patch.path, &rules.require_test_for_paths).is_empty();

        if touches_test_path {
            tests_changed += 1;
        }
        if generated {
            generated_files += 1;
            generated_hits.push(patch.path.clone());
        }
        if requires_tests {
            source_files_changed += 1;
            test_required_paths.push(patch.path.clone());
        }

        let mut status = "safe".to_string();
        let mut reasons = Vec::new();
        let mut matched_rules = Vec::new();

        let blocked_paths = matching_patterns(&patch.path, &rules.blocked_paths);
        if !blocked_paths.is_empty() {
            status = "block".into();
            reasons.push("Touches a blocked path".to_string());
            matched_rules.extend(blocked_paths.iter().map(|p| format!("blocked path: {p}")));
            blocked_path_hits.push(format!("{} ({})", patch.path, blocked_paths.join(", ")));
        }

        let warn_paths = matching_patterns(&patch.path, &rules.warn_paths);
        if !warn_paths.is_empty() {
            if status != "block" {
                status = "warn".into();
            }
            reasons.push("Touches a sensitive path".to_string());
            matched_rules.extend(warn_paths.iter().map(|p| format!("warn path: {p}")));
            warn_path_hits.push(format!("{} ({})", patch.path, warn_paths.join(", ")));
            sensitive_code_changes += 1;
        }

        for term in &rules.blocked_terms {
            let matches: Vec<_> = patch
                .added_lines
                .iter()
                .filter(|line| blocked_term_matches(line, term))
                .take(2)
                .cloned()
                .collect();
            if !matches.is_empty() {
                status = "block".into();
                reasons.push(format!("Added blocked term '{term}'"));
                matched_rules.push(format!("blocked term: {term}"));
                blocked_term_hits.push(format!("{} -> {}", patch.path, matches.join(" | ")));
            }
        }

        for term in &rules.suspicious_terms {
            let matches: Vec<_> = patch
                .added_lines
                .iter()
                .filter(|line| suspicious_term_matches(line, term))
                .take(2)
                .cloned()
                .collect();
            if !matches.is_empty() {
                if status == "safe" {
                    status = "warn".into();
                }
                reasons.push(format!("Added suspicious term '{term}'"));
                matched_rules.push(format!("suspicious term: {term}"));
                suspicious_term_hits.push(format!("{} -> {}", patch.path, matches.join(" | ")));
            }
        }

        if !path_policy.is_empty() {
            policy_hits.push(format!("{} — {}", patch.path, path_policy));
        }

        let mut summary_parts = Vec::new();
        if !reasons.is_empty() {
            summary_parts.push(reasons.join(" "));
        }
        if !path_policy.is_empty() {
            summary_parts.push(path_policy.clone());
        }
        if generated {
            summary_parts.push(
                "Likely generated or lockfile output; review it together with the source of truth that produced it.".into(),
            );
        }

        let summary = if summary_parts.is_empty() {
            "No immediate rule hits in this file.".to_string()
        } else {
            summary_parts.join(" ")
        };

        files.push(FileAssessment {
            path: patch.path,
            status,
            additions: patch.additions,
            deletions: patch.deletions,
            matched_rules,
            summary,
            generated,
            path_policy,
        });
    }

    files.sort_by_key(|file| match file.status.as_str() {
        "block" => 0,
        "warn" => 1,
        _ => 2,
    });

    if !blocked_path_hits.is_empty() {
        findings.push(make_finding(
            "blocked_paths",
            "Blocked file paths",
            "block",
            "The diff touches file areas that should not move forward without explicit review.",
            limit_examples(blocked_path_hits.clone(), 6),
        ));
    }
    if !warn_path_hits.is_empty() {
        findings.push(make_finding(
            "warn_paths",
            "Sensitive file paths",
            "warn",
            "The diff touches file areas that deserve extra scrutiny.",
            limit_examples(warn_path_hits, 6),
        ));
    }
    if !policy_hits.is_empty() {
        findings.push(make_finding(
            "path_policy", "Path-specific policy notes",
            if !blocked_path_hits.is_empty() { "block" } else { "warn" },
            "Some touched files sit on boundaries where even small edits can have outsized impact on trust, runtime, or data safety.",
            limit_examples(policy_hits, 6),
        ));
    }
    if !blocked_term_hits.is_empty() {
        findings.push(make_finding(
            "blocked_terms",
            "Blocked added content",
            "block",
            "The diff appears to add secret-like or explicitly banned content.",
            limit_examples(blocked_term_hits, 6),
        ));
    }
    if !suspicious_term_hits.is_empty() {
        findings.push(make_finding(
            "suspicious_terms",
            "Suspicious added content",
            "warn",
            "The diff adds lines that often correlate with fragile or risky changes.",
            limit_examples(suspicious_term_hits, 6),
        ));
    }

    if files_changed > rules.max_files {
        let severity = if files_changed > rules.max_files.saturating_mul(2) {
            "block"
        } else {
            "warn"
        };
        findings.push(make_finding(
            "scope_files",
            "Scope exceeds file budget",
            severity,
            format!(
                "This diff changes {files_changed} files, above the repo rule limit of {}.",
                rules.max_files
            ),
            vec![format!("{files_changed} changed files")],
        ));
    }
    if additions > rules.max_additions {
        let severity = if additions > rules.max_additions.saturating_mul(2) {
            "block"
        } else {
            "warn"
        };
        findings.push(make_finding(
            "scope_additions",
            "Additions exceed budget",
            severity,
            format!(
                "This diff adds {additions} lines, above the repo rule limit of {}.",
                rules.max_additions
            ),
            vec![format!("{additions} added lines")],
        ));
    }
    if deletions > rules.max_deletions {
        let severity = if deletions > rules.max_deletions.saturating_mul(2) {
            "block"
        } else {
            "warn"
        };
        findings.push(make_finding(
            "scope_deletions",
            "Deletions exceed budget",
            severity,
            format!(
                "This diff deletes {deletions} lines, above the repo rule limit of {}.",
                rules.max_deletions
            ),
            vec![format!("{deletions} deleted lines")],
        ));
    }
    if generated_files > 0 {
        let severity = if generated_files >= 3 && source_files_changed == 0 {
            "block"
        } else {
            "warn"
        };
        let detail = if source_files_changed == 0 {
            "The diff changes generated artifacts or lockfiles without touching likely source files. Review the true source of change before moving forward."
        } else {
            "The diff includes generated artifacts or lockfiles. Review them alongside the code or configuration that produced them."
        };
        findings.push(make_finding(
            "generated_files",
            "Generated files changed",
            severity,
            detail,
            limit_examples(generated_hits, 6),
        ));
    }

    let missing_tests = !test_required_paths.is_empty() && tests_changed == 0;
    if missing_tests {
        let severity = if sensitive_code_changes > 0
            || source_files_changed > 2
            || additions > 140
            || deletions > 80
        {
            "block"
        } else {
            "warn"
        };
        findings.push(make_finding(
            "missing_tests",
            "Code changes without tests",
            severity,
            "The diff touches code paths that normally deserve tests, but no test files changed.",
            limit_examples(test_required_paths.clone(), 6),
        ));
    }

    let risky_files = files
        .iter()
        .filter(|f| f.status == "warn" || f.status == "block")
        .count() as u32;
    if (files_changed > rules.max_files
        || additions > rules.max_additions
        || deletions > rules.max_deletions)
        && risky_files >= 3
    {
        findings.push(make_finding(
            "large_risky_diff", "Large diff with concentrated risk", "block",
            "This patch is both large and concentrated in sensitive areas. TrustGate should not treat it like a normal bounded AI patch.",
            vec![
                format!("{} changed", count_phrase(files_changed, "file", "files")),
                count_phrase(risky_files, "risky file", "risky files"),
                format!("{additions} additions / {deletions} deletions"),
            ],
        ));
    }

    if let Some(context) = repo_memory_context.as_ref() {
        if !context.guardrails.is_empty() {
            let severity = if context
                .guardrails
                .iter()
                .any(|guardrail| guardrail.severity == "block")
            {
                "block"
            } else {
                "warn"
            };
            findings.push(make_finding(
                "failguard_policy",
                "FailGuard promoted guardrails",
                severity,
                "A human-promoted FailGuard lesson matched this diff and compiled into TrustGate policy evidence.",
                context
                    .guardrails
                    .iter()
                    .take(4)
                    .map(|guardrail| {
                        format!(
                            "{} — {}{}",
                            guardrail.title,
                            guardrail.instruction,
                            if guardrail.matched_paths.is_empty() {
                                String::new()
                            } else {
                                format!(" Matched: {}.", guardrail.matched_paths.join(", "))
                            }
                        )
                    })
                    .collect(),
            ));
        }

        if missing_tests {
            let memory_testing_entries = context
                .entries
                .iter()
                .filter(|e| e.kind == "testing_expectation" || e.tags.iter().any(|t| t == "tests"))
                .take(4)
                .collect::<Vec<_>>();
            let memory_testing = memory_testing_entries
                .iter()
                .map(|e| format!("{} — {}", e.title, e.prompt_line))
                .collect::<Vec<_>>();
            if !memory_testing.is_empty() {
                findings.push(make_finding(
                    "repo_memory_tests", "RepoMemory test expectations",
                    if sensitive_code_changes > 0 || memory_testing_entries.iter().any(|e| e.pinned || e.disposition == "policy") { "block" } else { "warn" },
                    "RepoMemory found prior evidence that this repo expects tests for changes like these.",
                    memory_testing,
                ));
            }
        }

        let hotspot_entries = context
            .entries
            .iter()
            .filter(|e| e.kind == "hotspot" && !e.matched_paths.is_empty())
            .map(|e| format!("{} -> {}", e.matched_paths.join(", "), e.prompt_line))
            .take(4)
            .collect::<Vec<_>>();
        if !hotspot_entries.is_empty() {
            findings.push(make_finding(
                "repo_memory_hotspots", "RepoMemory hotspots", "warn",
                "RepoMemory says this diff touches high-context areas that have attracted repeat fixes or review churn.",
                hotspot_entries,
            ));
        }

        let failure_entries = context
            .entries
            .iter()
            .filter(|e| e.kind == "failure_pattern")
            .filter(|entry| {
                !context
                    .guardrails
                    .iter()
                    .any(|guardrail| guardrail.memory_ref == entry.memory_ref)
            })
            .map(|e| format!("{} — {}", e.title, e.prompt_line))
            .take(3)
            .collect::<Vec<_>>();
        let failure_is_curated = context
            .entries
            .iter()
            .filter(|e| e.kind == "failure_pattern")
            .filter(|entry| {
                !context
                    .guardrails
                    .iter()
                    .any(|guardrail| guardrail.memory_ref == entry.memory_ref)
            })
            .any(|e| e.pinned || e.disposition == "policy");
        if !failure_entries.is_empty() {
            findings.push(make_finding(
                "repo_memory_failures",
                "RepoMemory failure patterns",
                if failure_is_curated && sensitive_code_changes > 0 {
                    "block"
                } else {
                    "warn"
                },
                "RepoMemory found recurring historical failures that look relevant to this review.",
                failure_entries,
            ));
        }
    }

    let blocked_findings = findings.iter().filter(|f| f.severity == "block").count() as u32;
    let warning_findings = findings.iter().filter(|f| f.severity == "warn").count() as u32;

    let recommendation = if blocked_findings > 0 {
        "block"
    } else if warning_findings > 0 {
        "warn"
    } else {
        "safe"
    };

    let raw_risk_score = clamp_score(
        blocked_findings as usize * 34
            + warning_findings as usize * 11
            + risky_files as usize * 8
            + generated_files as usize * 4
            + source_files_changed as usize * 3
            + usize::from(missing_tests) * 12,
    );
    let risk_score = decision_adjusted_score(raw_risk_score, recommendation);

    let summary = match recommendation {
        "block" => format!(
            "Block this diff for now. TrustGate found {} and {}.",
            count_phrase(blocked_findings, "blocking issue", "blocking issues"),
            count_phrase(warning_findings, "warning", "warnings"),
        ),
        "warn" => format!(
            "Review closely before merge. TrustGate found {} across {}.",
            count_phrase(warning_findings, "warning", "warnings"),
            count_phrase(risky_files, "risky file", "risky files"),
        ),
        _ => format!(
            "This diff looks safe against the current repo rules. {} changed with no active warnings.",
            count_phrase(files_changed, "file", "files"),
        ),
    };

    ReviewResult {
        id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        repo: repo.to_string(),
        ai_source: ai_source.to_string(),
        recommendation: recommendation.into(),
        risk_score,
        summary,
        metrics: ReviewMetricSummary {
            files_changed,
            additions,
            deletions,
            tests_changed,
            risky_files,
            blocked_findings,
            warning_findings,
            generated_files,
            source_files_changed,
        },
        files,
        findings,
        rules,
        diff: diff.to_string(),
        source_kind: source_kind.into(),
        github: github_context,
        github_report: None,
        repo_memory_context,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        blocked_term_matches, count_phrase, decision_adjusted_score, suspicious_term_matches,
    };

    #[test]
    fn count_phrase_uses_singular_only_for_one() {
        assert_eq!(count_phrase(1, "file", "files"), "1 file");
        assert_eq!(count_phrase(0, "file", "files"), "0 files");
        assert_eq!(count_phrase(2, "file", "files"), "2 files");
    }

    #[test]
    fn short_secret_prefixes_do_not_match_ordinary_words() {
        assert!(!blocked_term_matches(
            "import { TaskPanel } from './task-panel';",
            "sk-"
        ));
        assert!(!blocked_term_matches("const task_id = 'task-123';", "sk-"));
        assert!(!blocked_term_matches("const makia = true;", "AKIA"));
    }

    #[test]
    fn credential_like_values_still_match_blocked_prefixes() {
        assert!(blocked_term_matches("token = 'sk-proj-1234567890'", "sk-"));
        assert!(blocked_term_matches(
            "GITHUB_TOKEN=ghp_1234567890abcdef",
            "ghp_"
        ));
        assert!(blocked_term_matches(
            "token=github_pat_1234567890abcdef",
            "github_pat_"
        ));
        assert!(blocked_term_matches(
            "AWS_ACCESS_KEY_ID=AKIA1234567890ABCD",
            "AKIA"
        ));
    }

    #[test]
    fn todo_markers_do_not_match_domain_status_values() {
        assert!(!suspicious_term_matches(
            "const VISIBLE_STATUSES = ['ready', 'todo', 'blocked'] as const;",
            "TODO"
        ));
        assert!(suspicious_term_matches("// TODO: handle the error", "TODO"));
        assert!(suspicious_term_matches("# FIXME remove fallback", "FIXME"));
    }

    #[test]
    fn sensitive_identifiers_require_hardcoded_values() {
        assert!(!suspicious_term_matches(
            "from web_server import _SESSION_TOKEN",
            "token"
        ));
        assert!(!suspicious_term_matches(
            "client.headers[name] = _SESSION_TOKEN",
            "token"
        ));
        assert!(suspicious_term_matches("password = 'not-safe'", "password"));
        assert!(suspicious_term_matches(
            "\"token\": \"example-value\"",
            "token"
        ));
    }

    #[test]
    fn decision_severity_establishes_score_floors() {
        assert_eq!(decision_adjusted_score(43, "block"), 70);
        assert_eq!(decision_adjusted_score(92, "block"), 92);
        assert_eq!(decision_adjusted_score(11, "warn"), 25);
        assert_eq!(decision_adjusted_score(40, "warn"), 40);
        assert_eq!(decision_adjusted_score(9, "safe"), 9);
    }
}

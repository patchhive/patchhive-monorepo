use std::collections::BTreeSet;

use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};
use chrono::Utc;
use glob::Pattern;
use serde_json::json;
use uuid::Uuid;

use crate::{
    db,
    models::{
        FileAssessment, RepoRuleSet, ReviewFinding, ReviewMetricSummary, ReviewRequest, ReviewResult,
    },
};

type ApiError = (StatusCode, Json<serde_json::Value>);

#[derive(Debug, Default)]
struct FilePatch {
    path: String,
    additions: u32,
    deletions: u32,
    added_lines: Vec<String>,
}

fn api_error(status: StatusCode, message: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": message.into() })))
}

fn matches_rule(value: &str, pattern: &str) -> bool {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.contains('*') || trimmed.contains('?') || trimmed.contains('[') {
        return Pattern::new(trimmed)
            .map(|compiled| compiled.matches(value))
            .unwrap_or_else(|_| value.to_lowercase().contains(&trimmed.to_lowercase()));
    }

    value.to_lowercase().contains(&trimmed.to_lowercase())
}

fn matching_patterns<'a>(value: &str, patterns: &'a [String]) -> Vec<&'a str> {
    patterns
        .iter()
        .map(String::as_str)
        .filter(|pattern| matches_rule(value, pattern))
        .collect()
}

fn parse_diff(diff: &str) -> Vec<FilePatch> {
    let mut files = Vec::new();
    let mut current: Option<FilePatch> = None;
    let mut fragment_counter = 1u32;

    let flush = |files: &mut Vec<FilePatch>, current: &mut Option<FilePatch>| {
        if let Some(mut patch) = current.take() {
            if patch.path.trim().is_empty() {
                patch.path = format!("diff-fragment-{}", files.len() + 1);
            }
            files.push(patch);
        }
    };

    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            flush(&mut files, &mut current);
            let path = line
                .split_whitespace()
                .nth(3)
                .map(|value| value.trim_start_matches("b/").to_string())
                .unwrap_or_else(|| format!("diff-fragment-{fragment_counter}"));
            fragment_counter += 1;
            current = Some(FilePatch {
                path,
                ..FilePatch::default()
            });
            continue;
        }

        if line.starts_with("+++ b/") {
            let path = line.trim_start_matches("+++ b/").trim().to_string();
            if let Some(current_patch) = current.as_mut() {
                current_patch.path = path;
            } else {
                current = Some(FilePatch {
                    path,
                    ..FilePatch::default()
                });
            }
            continue;
        }

        if !line.trim().is_empty() && current.is_none() {
            current = Some(FilePatch {
                path: format!("diff-fragment-{fragment_counter}"),
                ..FilePatch::default()
            });
            fragment_counter += 1;
        }

        if let Some(current_patch) = current.as_mut() {
            if line.starts_with('+') && !line.starts_with("+++") {
                current_patch.additions += 1;
                current_patch
                    .added_lines
                    .push(line.trim_start_matches('+').to_string());
            } else if line.starts_with('-') && !line.starts_with("---") {
                current_patch.deletions += 1;
            }
        }
    }

    flush(&mut files, &mut current);
    files
}

fn clamp_score(value: usize) -> u32 {
    value.min(100) as u32
}

fn make_finding(
    key: &str,
    label: &str,
    severity: &str,
    detail: impl Into<String>,
    evidence: Vec<String>,
) -> ReviewFinding {
    ReviewFinding {
        key: key.into(),
        label: label.into(),
        severity: severity.into(),
        detail: detail.into(),
        evidence,
    }
}

fn review_diff(repo: &str, diff: &str, ai_source: &str, rules: RepoRuleSet) -> ReviewResult {
    let patches = parse_diff(diff);
    let files_changed = patches.len() as u32;
    let additions = patches.iter().map(|patch| patch.additions).sum::<u32>();
    let deletions = patches.iter().map(|patch| patch.deletions).sum::<u32>();
    let tests_changed = patches
        .iter()
        .filter(|patch| !matching_patterns(&patch.path, &rules.test_paths).is_empty())
        .count() as u32;

    let mut findings = Vec::new();
    let mut files = Vec::new();
    let mut blocked_path_hits = Vec::new();
    let mut warn_path_hits = Vec::new();
    let mut blocked_term_hits = Vec::new();
    let mut suspicious_term_hits = Vec::new();

    for patch in patches {
        let mut status = "safe".to_string();
        let mut reasons = Vec::new();
        let mut matched_rules = Vec::new();

        let blocked_paths = matching_patterns(&patch.path, &rules.blocked_paths);
        if !blocked_paths.is_empty() {
            status = "block".into();
            reasons.push("Touches a blocked path".to_string());
            matched_rules.extend(
                blocked_paths
                    .iter()
                    .map(|pattern| format!("blocked path: {pattern}")),
            );
            blocked_path_hits.push(format!("{} ({})", patch.path, blocked_paths.join(", ")));
        }

        let warn_paths = matching_patterns(&patch.path, &rules.warn_paths);
        if !warn_paths.is_empty() {
            if status != "block" {
                status = "warn".into();
            }
            reasons.push("Touches a sensitive path".to_string());
            matched_rules.extend(
                warn_paths
                    .iter()
                    .map(|pattern| format!("warn path: {pattern}")),
            );
            warn_path_hits.push(format!("{} ({})", patch.path, warn_paths.join(", ")));
        }

        for term in &rules.blocked_terms {
            let matches = patch
                .added_lines
                .iter()
                .filter(|line| line.to_lowercase().contains(&term.to_lowercase()))
                .take(2)
                .cloned()
                .collect::<Vec<_>>();
            if !matches.is_empty() {
                status = "block".into();
                reasons.push(format!("Added blocked term '{term}'"));
                matched_rules.push(format!("blocked term: {term}"));
                blocked_term_hits.push(format!("{} -> {}", patch.path, matches.join(" | ")));
            }
        }

        for term in &rules.suspicious_terms {
            let matches = patch
                .added_lines
                .iter()
                .filter(|line| line.to_lowercase().contains(&term.to_lowercase()))
                .take(2)
                .cloned()
                .collect::<Vec<_>>();
            if !matches.is_empty() {
                if status == "safe" {
                    status = "warn".into();
                }
                reasons.push(format!("Added suspicious term '{term}'"));
                matched_rules.push(format!("suspicious term: {term}"));
                suspicious_term_hits.push(format!("{} -> {}", patch.path, matches.join(" | ")));
            }
        }

        let summary = if reasons.is_empty() {
            "No immediate rule hits in this file.".to_string()
        } else {
            reasons.join(" ")
        };

        files.push(FileAssessment {
            path: patch.path,
            status,
            additions: patch.additions,
            deletions: patch.deletions,
            matched_rules,
            summary,
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
            blocked_path_hits.into_iter().take(6).collect(),
        ));
    }

    if !warn_path_hits.is_empty() {
        findings.push(make_finding(
            "warn_paths",
            "Sensitive file paths",
            "warn",
            "The diff touches file areas that deserve extra scrutiny.",
            warn_path_hits.into_iter().take(6).collect(),
        ));
    }

    if !blocked_term_hits.is_empty() {
        findings.push(make_finding(
            "blocked_terms",
            "Blocked added content",
            "block",
            "The diff appears to add secret-like or explicitly banned content.",
            blocked_term_hits.into_iter().take(6).collect(),
        ));
    }

    if !suspicious_term_hits.is_empty() {
        findings.push(make_finding(
            "suspicious_terms",
            "Suspicious added content",
            "warn",
            "The diff adds lines that often correlate with fragile or risky changes.",
            suspicious_term_hits.into_iter().take(6).collect(),
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

    let needs_tests = files.iter().any(|file| {
        !matching_patterns(&file.path, &rules.require_test_for_paths).is_empty()
            && matching_patterns(&file.path, &rules.test_paths).is_empty()
    });
    if needs_tests && tests_changed == 0 {
        let evidence = files
            .iter()
            .filter(|file| {
                !matching_patterns(&file.path, &rules.require_test_for_paths).is_empty()
                    && matching_patterns(&file.path, &rules.test_paths).is_empty()
            })
            .map(|file| file.path.clone())
            .take(6)
            .collect::<Vec<_>>();

        findings.push(make_finding(
            "missing_tests",
            "Code changes without tests",
            "warn",
            "The diff touches code paths that normally deserve tests, but no test files changed.",
            evidence,
        ));
    }

    let blocked_findings = findings
        .iter()
        .filter(|finding| finding.severity == "block")
        .count() as u32;
    let warning_findings = findings
        .iter()
        .filter(|finding| finding.severity == "warn")
        .count() as u32;
    let risky_files = files
        .iter()
        .filter(|file| file.status == "warn" || file.status == "block")
        .count() as u32;

    let recommendation = if blocked_findings > 0 {
        "block"
    } else if warning_findings > 0 {
        "warn"
    } else {
        "safe"
    };

    let risk_score = clamp_score(
        blocked_findings as usize * 35
            + warning_findings as usize * 12
            + risky_files as usize * 8
            + usize::from(needs_tests && tests_changed == 0) * 10,
    );

    let summary = match recommendation {
        "block" => format!(
            "Block this diff for now. TrustGate found {blocked_findings} blocking issues and {warning_findings} warnings."
        ),
        "warn" => format!(
            "Review closely before merge. TrustGate found {warning_findings} warnings across {risky_files} risky files."
        ),
        _ => format!(
            "This diff looks safe against the current repo rules. {} files changed with no active warnings.",
            files_changed
        ),
    };

    ReviewResult {
        id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        repo: repo.to_string(),
        ai_source: if ai_source.trim().is_empty() {
            "unknown".into()
        } else {
            ai_source.trim().to_string()
        },
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
        },
        files,
        findings,
        rules,
        diff: diff.to_string(),
    }
}

pub async fn review(Json(body): Json<ReviewRequest>) -> Result<Json<ReviewResult>, ApiError> {
    let Some(repo) = db::normalize_repo_name(&body.repo) else {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "TrustGate expects repos in owner/repo format.",
        ));
    };

    if body.diff.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Paste a unified diff before running TrustGate.",
        ));
    }

    let mut rules = if let Some(mut rules) = body.rules {
        rules.repo = repo.clone();
        rules
    } else if let Some(saved) = db::get_rules(&repo)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
    {
        saved
    } else {
        let mut defaults = RepoRuleSet::default();
        defaults.repo = repo.clone();
        defaults
    };

    rules.repo = repo.clone();

    let review = review_diff(&repo, &body.diff, &body.ai_source, rules);
    db::save_review(&review)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(review))
}

pub async fn history() -> Result<Json<serde_json::Value>, ApiError> {
    let reviews = db::list_reviews()
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(json!({ "reviews": reviews })))
}

pub async fn history_detail(Path(id): Path<String>) -> Result<Json<ReviewResult>, ApiError> {
    let review = db::get_review(&id)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    match review {
        Some(review) => Ok(Json(review)),
        None => Err(api_error(StatusCode::NOT_FOUND, "TrustGate review not found.")),
    }
}

pub fn unique_repos(reviews: &[crate::models::ReviewHistoryItem]) -> usize {
    reviews
        .iter()
        .map(|review| review.repo.clone())
        .collect::<BTreeSet<_>>()
        .len()
}

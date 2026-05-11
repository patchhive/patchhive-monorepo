// types.rs — Shared types and helper functions for diff analysis

use glob::Pattern;

use crate::models::ReviewFinding;

pub type ApiError = (axum::http::StatusCode, axum::Json<serde_json::Value>);

pub fn api_error(status: axum::http::StatusCode, message: impl Into<String>) -> ApiError {
    (
        status,
        axum::Json(serde_json::json!({ "error": message.into() })),
    )
}

#[derive(Debug, Default)]
pub struct FilePatch {
    pub path: String,
    pub additions: u32,
    pub deletions: u32,
    pub added_lines: Vec<String>,
}

pub fn matches_rule(value: &str, pattern: &str) -> bool {
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

pub fn matching_patterns<'a>(value: &str, patterns: &'a [String]) -> Vec<&'a str> {
    patterns
        .iter()
        .map(String::as_str)
        .filter(|pattern| matches_rule(value, pattern))
        .collect()
}

pub fn parse_diff(diff: &str) -> Vec<FilePatch> {
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

pub fn clamp_score(value: usize) -> u32 {
    value.min(100) as u32
}

pub fn make_finding(
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

pub fn is_generated_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with("package-lock.json")
        || lower.ends_with("pnpm-lock.yaml")
        || lower.ends_with("yarn.lock")
        || lower.ends_with("cargo.lock")
        || lower.contains("/dist/")
        || lower.contains("/build/")
        || lower.contains("/coverage/")
        || lower.contains("/generated/")
        || lower.ends_with(".snap")
        || lower.ends_with(".min.js")
        || lower.ends_with(".min.css")
        || lower.ends_with(".pb.go")
}

pub fn is_docs_only_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".md")
        || lower.starts_with("docs/")
        || lower.contains("/docs/")
        || lower.ends_with("changelog")
        || lower.ends_with("license")
}

pub fn path_policy_note(path: &str) -> Option<&'static str> {
    let lower = path.to_lowercase();
    if lower.contains(".github/workflows/") {
        Some("Workflow edits can change CI behavior, release automation, or secret exposure.")
    } else if lower.contains("auth/") || lower.contains("permission") {
        Some("Auth and permission changes deserve extra scrutiny because small mistakes can broaden access.")
    } else if lower.contains("billing") {
        Some("Billing paths affect money movement and should be reviewed with policy and test coverage in mind.")
    } else if lower.contains("terraform/")
        || lower.contains("infra/")
        || lower.contains("dockerfile")
        || lower.contains("docker-compose")
    {
        Some("Infra/runtime changes can alter deployment, networking, or secret handling beyond the diff itself.")
    } else if lower.contains("migration") || lower.ends_with("schema.sql") {
        Some("Schema and migration changes can have irreversible data impact if they move forward too casually.")
    } else {
        None
    }
}

pub fn limit_examples(items: Vec<String>, limit: usize) -> Vec<String> {
    items.into_iter().take(limit).collect()
}

pub fn normalize_ai_source(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.into()
    } else {
        value.trim().to_string()
    }
}

pub fn short_text(value: &str, limit: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_string();
    }
    let mut out = trimmed
        .chars()
        .take(limit.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

pub fn summarize_diff_for_memory(patches: &[FilePatch]) -> String {
    patches
        .iter()
        .take(6)
        .map(|patch| {
            let lines = patch
                .added_lines
                .iter()
                .take(2)
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" | ");
            if lines.is_empty() {
                patch.path.clone()
            } else {
                format!("{}: {}", patch.path, lines)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn summarize_review_task(
    repo: &str,
    ai_source: &str,
    source_kind: &str,
    github_context: Option<&crate::models::GitHubReviewContext>,
) -> String {
    if let Some(context) = github_context {
        format!(
            "Review {} diff for {} PR #{} {} from {} to {}.",
            ai_source,
            repo,
            context.pr_number,
            context.pr_title,
            context.head_ref,
            context.base_ref,
        )
    } else {
        format!("Review {ai_source} diff for {repo} from {source_kind}.")
    }
}

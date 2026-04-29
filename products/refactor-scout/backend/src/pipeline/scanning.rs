use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, Result};
use chrono::Utc;
use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

use crate::models::{RefactorOpportunity, RefactorScanResult};

use super::analysis::*;

pub(crate) const MAX_SCAN_FILES: u32 = 1_500;
pub(crate) const MAX_RETURNED_OPPORTUNITIES: usize = 60;
pub(crate) const MAX_FILE_BYTES: u64 = 350_000;
pub(crate) const LONG_FILE_THRESHOLD: usize = 320;
pub(crate) const LONG_FUNCTION_THRESHOLD: usize = 60;
pub(crate) const REPEATED_LITERAL_MIN_LEN: usize = 12;
pub(crate) const REPEATED_LITERAL_MIN_REPEATS: u32 = 3;
pub(crate) const MAX_WARNINGS: usize = 12;

static RUST_FN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)")
        .expect("rust function regex should compile")
});
static PY_FN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)").expect("python function regex should compile")
});
static JS_FN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)")
        .expect("javascript function regex should compile")
});
static JS_ARROW_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*(?:async\s*)?(?:\([^)]*\)|[A-Za-z_$][A-Za-z0-9_$]*)\s*=>")
        .expect("javascript arrow regex should compile")
});
static GO_FN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*func\s+(?:\([^)]+\)\s*)?([A-Za-z_][A-Za-z0-9_]*)")
        .expect("go function regex should compile")
});
static DOUBLE_QUOTED_LITERAL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""([^"\\]|\\.){12,}""#).expect("double-quoted literal regex should compile")
});
static SINGLE_QUOTED_LITERAL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"'([^'\\]|\\.){12,}'"#).expect("single-quoted literal regex should compile")
});

pub(crate) struct ScanArtifacts {
    pub(crate) opportunities: Vec<RefactorOpportunity>,
    pub(crate) warnings: Vec<String>,
    pub(crate) files_scanned: u32,
    pub(crate) files_skipped: u32,
    pub(crate) limit_hit: bool,
}

pub fn build_scan_result(
    state: &crate::state::AppState,
    repo_path: &str,
    max_files: u32,
) -> Result<RefactorScanResult> {
    let root = resolve_scan_root(repo_path, state)?;
    let repo_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(repo_path)
        .to_string();

    let mut artifacts = scan_repo(&root, max_files)?;
    if artifacts.limit_hit {
        push_warning(
            &mut artifacts.warnings,
            format!(
                "Scan stopped after {max_files} supported files. Raise max files if this repo regularly pushes the cap."
            ),
        );
    }

    artifacts.opportunities.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| safety_rank(&right.safety).cmp(&safety_rank(&left.safety)))
            .then_with(|| left.path.cmp(&right.path))
    });
    artifacts.opportunities.truncate(MAX_RETURNED_OPPORTUNITIES);

    let metrics = build_metrics(
        artifacts.files_scanned,
        artifacts.files_skipped,
        &artifacts.opportunities,
    );
    let summary = build_summary(&repo_name, &metrics, artifacts.opportunities.first());

    Ok(RefactorScanResult {
        id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        repo_path: root.display().to_string(),
        repo_name,
        summary,
        metrics,
        opportunities: artifacts.opportunities,
        warnings: artifacts.warnings,
    })
}

pub(crate) fn resolve_scan_root(
    repo_path: &str,
    state: &crate::state::AppState,
) -> Result<std::path::PathBuf> {
    use std::fs;

    let candidate = std::path::PathBuf::from(repo_path);
    let canonical = fs::canonicalize(&candidate)
        .map_err(|err| anyhow!("Could not access `{repo_path}`: {err}"))?;
    if !canonical.is_dir() {
        return Err(anyhow!("`{}` is not a directory.", canonical.display()));
    }

    let allowed_roots = state.resolved_allowed_roots();
    if allowed_roots.is_empty() {
        return Err(anyhow!(
            "RefactorScout has no readable allowed roots configured. Set REFACTOR_SCOUT_ALLOWED_ROOTS first."
        ));
    }
    if !crate::state::path_within_allowed_roots(&canonical, &allowed_roots) {
        return Err(anyhow!(
            "`{}` is outside the configured allowed roots.",
            canonical.display()
        ));
    }

    Ok(canonical)
}

fn scan_repo(root: &Path, max_files: u32) -> Result<ScanArtifacts> {
    use std::fs;

    let mut opportunities = Vec::new();
    let mut warnings = Vec::new();
    let mut files_scanned = 0;
    let mut files_skipped = 0;
    let mut limit_hit = false;

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(should_descend)
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                files_skipped += 1;
                push_warning(&mut warnings, format!("Could not walk one path: {err}"));
                continue;
            }
        };

        if !entry.file_type().is_file() || !supported_source(entry.path()) {
            continue;
        }

        if files_scanned >= max_files {
            limit_hit = true;
            break;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                files_skipped += 1;
                push_warning(
                    &mut warnings,
                    format!(
                        "Could not read metadata for {}: {err}",
                        entry.path().display()
                    ),
                );
                continue;
            }
        };

        if metadata.len() > MAX_FILE_BYTES {
            files_skipped += 1;
            push_warning(
                &mut warnings,
                format!(
                    "Skipped {} because it is larger than {} KB.",
                    entry.path().display(),
                    MAX_FILE_BYTES / 1024
                ),
            );
            continue;
        }

        let content = match fs::read_to_string(entry.path()) {
            Ok(content) => content,
            Err(err) => {
                files_skipped += 1;
                push_warning(
                    &mut warnings,
                    format!(
                        "Skipped {} because it is not readable text: {err}",
                        entry.path().display()
                    ),
                );
                continue;
            }
        };

        let relative_path = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .display()
            .to_string();
        let language = language_for_path(entry.path());
        opportunities.extend(analyze_file(&relative_path, language, &content));
        files_scanned += 1;
    }

    Ok(ScanArtifacts {
        opportunities,
        warnings,
        files_scanned,
        files_skipped,
        limit_hit,
    })
}

pub(crate) fn analyze_file(path: &str, language: &str, content: &str) -> Vec<RefactorOpportunity> {
    let lines = content.lines().collect::<Vec<_>>();
    let mut opportunities = Vec::new();

    if lines.len() > LONG_FILE_THRESHOLD {
        opportunities.push(large_file_opportunity(path, language, lines.len()));
    }

    opportunities.extend(long_function_opportunities(path, language, &lines));
    if let Some(opportunity) = repeated_literal_opportunity(path, language, content) {
        opportunities.push(opportunity);
    }

    opportunities
}

fn large_file_opportunity(path: &str, language: &str, line_count: usize) -> RefactorOpportunity {
    let score = 54 + ((line_count.saturating_sub(LONG_FILE_THRESHOLD) as u32) / 8).min(36);
    RefactorOpportunity {
        id: Uuid::new_v4().to_string(),
        kind: "large_file".into(),
        title: "Split oversized file".into(),
        summary: format!(
            "`{path}` is {} lines long, which is a strong signal that one cohesive slice could be extracted without changing behavior.",
            line_count
        ),
        path: path.into(),
        language: language.into(),
        score,
        safety: "medium".into(),
        effort: "medium".into(),
        line_start: 1,
        line_end: line_count as u32,
        suggestion: "Start by extracting one helper cluster or domain slice behind the current public surface so the file gets smaller without changing callers.".into(),
        evidence: vec![
            format!("{line_count} total lines"),
            "Oversized modules are often the safest first cut for incremental refactors.".into(),
        ],
    }
}

fn long_function_opportunities(
    path: &str,
    language: &str,
    lines: &[&str],
) -> Vec<RefactorOpportunity> {
    let mut opportunities = Vec::new();

    for start in 0..lines.len() {
        let Some(name) = function_name_for_line(language, lines[start]) else {
            continue;
        };

        let end = match language {
            "python" => python_function_end(lines, start),
            "rust" | "javascript" | "typescript" | "go" => match brace_function_end(lines, start) {
                Some(end) => end,
                None => continue,
            },
            _ => continue,
        };

        let line_count = end.saturating_sub(start) + 1;
        if line_count <= LONG_FUNCTION_THRESHOLD {
            continue;
        }

        let score = 58 + ((line_count.saturating_sub(LONG_FUNCTION_THRESHOLD) as u32) / 2).min(34);
        opportunities.push(RefactorOpportunity {
            id: Uuid::new_v4().to_string(),
            kind: "long_function".into(),
            title: format!("Extract helper from `{name}`"),
            summary: format!(
                "`{name}` in `{path}` spans {} lines ({}-{}), which usually means there is at least one validation, formatting, or branching step worth extracting.",
                line_count,
                start + 1,
                end + 1
            ),
            path: path.into(),
            language: language.into(),
            score,
            safety: "medium".into(),
            effort: "medium".into(),
            line_start: (start + 1) as u32,
            line_end: (end + 1) as u32,
            suggestion: "Keep the current function signature stable and extract one internal phase into a named helper first. That usually buys readability without widening the refactor blast radius.".into(),
            evidence: vec![
                format!("{line_count} lines in one function"),
                format!("Detected in {language} code"),
            ],
        });
    }

    opportunities
}

fn repeated_literal_opportunity(
    path: &str,
    language: &str,
    content: &str,
) -> Option<RefactorOpportunity> {
    let mut literals: HashMap<String, (u32, usize)> = HashMap::new();
    let literal_matchers: &[&Lazy<Regex>] = if language == "rust" {
        &[&DOUBLE_QUOTED_LITERAL_RE]
    } else {
        &[&DOUBLE_QUOTED_LITERAL_RE, &SINGLE_QUOTED_LITERAL_RE]
    };

    for matcher in literal_matchers {
        for matched in matcher.find_iter(content) {
            let literal = matched.as_str().trim_matches(&['"', '\''][..]).trim();
            if should_ignore_literal(literal) {
                continue;
            }

            let entry = literals
                .entry(literal.to_string())
                .or_insert((0, matched.start()));
            entry.0 += 1;
        }
    }

    let (literal, (count, first_offset)) = literals
        .into_iter()
        .filter(|(literal, (count, _))| {
            literal.len() >= REPEATED_LITERAL_MIN_LEN && *count >= REPEATED_LITERAL_MIN_REPEATS
        })
        .max_by(|left, right| {
            let left_score = left.1 .0 as usize * left.0.len();
            let right_score = right.1 .0 as usize * right.0.len();
            left_score.cmp(&right_score)
        })?;

    let line = 1 + content[..first_offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count() as u32;
    let preview = literal_preview(&literal);
    let score = 68 + (count.saturating_sub(REPEATED_LITERAL_MIN_REPEATS) * 7).min(18);

    Some(RefactorOpportunity {
        id: Uuid::new_v4().to_string(),
        kind: "repeated_literal".into(),
        title: "Extract repeated string literal".into(),
        summary: format!(
            "`{path}` repeats the string `{preview}` {} times, which is usually a low-risk extract-constant cleanup.",
            count
        ),
        path: path.into(),
        language: language.into(),
        score,
        safety: "high".into(),
        effort: "low".into(),
        line_start: line,
        line_end: line,
        suggestion: "Lift the repeated literal into a named constant close to its usage site first. If the meaning stays clear, promote it to a shared module later.".into(),
        evidence: vec![
            format!("{count} repeated occurrences"),
            "Repeated literals are usually one of the safest refactor entry points.".into(),
        ],
    })
}

fn function_name_for_line(language: &str, line: &str) -> Option<String> {
    let capture = match language {
        "rust" => RUST_FN_RE.captures(line),
        "python" => PY_FN_RE.captures(line),
        "javascript" | "typescript" => JS_FN_RE
            .captures(line)
            .or_else(|| JS_ARROW_RE.captures(line)),
        "go" => GO_FN_RE.captures(line),
        _ => None,
    }?;

    Some(capture.get(1)?.as_str().to_string())
}

fn brace_function_end(lines: &[&str], start: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut saw_body = false;

    for (offset, line) in lines[start..].iter().enumerate() {
        for ch in line.chars() {
            if ch == '{' {
                saw_body = true;
                depth += 1;
            } else if ch == '}' && saw_body {
                depth -= 1;
            }
        }

        if saw_body && depth <= 0 {
            return Some(start + offset);
        }
    }

    if saw_body {
        Some(lines.len().saturating_sub(1))
    } else {
        None
    }
}

fn python_function_end(lines: &[&str], start: usize) -> usize {
    let mut body_indent = None;
    let mut end = start;

    for index in (start + 1)..lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            end = index;
            continue;
        }

        if body_indent.is_none() {
            if trimmed.starts_with('#') {
                continue;
            }
            body_indent = Some(leading_indent(line));
            end = index;
            continue;
        }

        let indent = leading_indent(line);
        if indent < body_indent.unwrap_or(0) && !trimmed.starts_with('#') {
            break;
        }
        end = index;
    }

    end
}

fn leading_indent(line: &str) -> usize {
    line.chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .count()
}

fn supported_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("rs" | "py" | "js" | "jsx" | "ts" | "tsx" | "go")
    )
}

fn language_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust",
        Some("py") => "python",
        Some("ts" | "tsx") => "typescript",
        Some("go") => "go",
        _ => "javascript",
    }
}

fn should_descend(entry: &DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return true;
    }

    let Some(name) = entry.file_name().to_str() else {
        return false;
    };

    !matches!(
        name,
        ".git"
            | ".next"
            | ".turbo"
            | ".venv"
            | "build"
            | "coverage"
            | "dist"
            | "node_modules"
            | "target"
            | "vendor"
    )
}

fn should_ignore_literal(literal: &str) -> bool {
    let trimmed = literal.trim();
    trimmed.is_empty()
        || trimmed.len() < REPEATED_LITERAL_MIN_LEN
        || trimmed.contains("${")
        || trimmed.contains('{')
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("./")
        || trimmed.starts_with("../")
}

fn literal_preview(literal: &str) -> String {
    let sanitized = literal.replace('`', "'");
    if sanitized.len() <= 48 {
        sanitized
    } else {
        format!("{}...", &sanitized[..45])
    }
}

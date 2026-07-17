use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Output, Stdio},
    time::Duration,
};

use anyhow::{anyhow, Result};
use chrono::Utc;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::process::Command;
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
const DEFAULT_CLONE_TIMEOUT_SECS: u64 = 120;
const MAX_CLONE_ERROR_BYTES: usize = 600;

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
pub(crate) struct ScanArtifacts {
    pub(crate) opportunities: Vec<RefactorOpportunity>,
    pub(crate) warnings: Vec<String>,
    pub(crate) files_scanned: u32,
    pub(crate) files_skipped: u32,
    pub(crate) limit_hit: bool,
}

#[derive(Default)]
struct BraceScanState {
    block_comment_depth: u32,
    string_delimiter: Option<char>,
    escaped: bool,
    rust_raw_hashes: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitHubRepoTarget {
    owner: String,
    repo: String,
}

impl GitHubRepoTarget {
    pub(crate) fn label(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }

    fn clone_url(&self) -> String {
        format!("https://github.com/{}/{}.git", self.owner, self.repo)
    }
}

struct TemporaryClone {
    path: PathBuf,
}

impl TemporaryClone {
    fn new(target: &GitHubRepoTarget) -> Result<Self> {
        let parent = std::env::temp_dir().join("refactor-scout-clones");
        std::fs::create_dir_all(&parent).map_err(|err| {
            anyhow!(
                "Could not create temporary clone directory `{}`: {err}",
                parent.display()
            )
        })?;

        Ok(Self {
            path: parent.join(format!(
                "{}-{}-{}",
                sanitize_clone_segment(&target.owner),
                sanitize_clone_segment(&target.repo),
                Uuid::new_v4()
            )),
        })
    }
}

impl Drop for TemporaryClone {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub async fn build_scan_result_for_input(
    state: &crate::state::AppState,
    repo_path: &str,
    max_files: u32,
) -> Result<RefactorScanResult> {
    if !should_use_local_scan(repo_path) {
        if let Some(target) = parse_github_repo_target(repo_path) {
            return build_github_scan_result(target, max_files).await;
        }
    }

    build_scan_result(state, repo_path, max_files)
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
    build_scan_result_from_root(&root, root.display().to_string(), repo_name, max_files)
}

async fn build_github_scan_result(
    target: GitHubRepoTarget,
    max_files: u32,
) -> Result<RefactorScanResult> {
    let clone = TemporaryClone::new(&target)?;
    clone_github_repo(&target, &clone.path).await?;
    let label = target.label();
    build_scan_result_from_root(&clone.path, label.clone(), label, max_files)
}

fn build_scan_result_from_root(
    root: &Path,
    repo_path: String,
    repo_name: String,
    max_files: u32,
) -> Result<RefactorScanResult> {
    let mut artifacts = scan_repo(root, max_files)?;
    if artifacts.limit_hit {
        push_warning(
            &mut artifacts.warnings,
            format!(
                "Scan stopped after {max_files} supported files. Raise max files if this repo regularly pushes the cap."
            ),
        );
    }

    artifacts.opportunities.sort_by(|left, right| {
        safety_rank(&right.safety)
            .cmp(&safety_rank(&left.safety))
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| left.path.cmp(&right.path))
    });
    let mut metrics = build_metrics(
        artifacts.files_scanned,
        artifacts.files_skipped,
        &artifacts.opportunities,
    );
    artifacts.opportunities.truncate(MAX_RETURNED_OPPORTUNITIES);
    metrics.returned_opportunities = artifacts.opportunities.len() as u32;
    metrics.opportunities_truncated = metrics.returned_opportunities < metrics.opportunities;
    if metrics.opportunities_truncated {
        push_warning(
            &mut artifacts.warnings,
            format!(
                "RefactorScout found {} candidates and returned the highest-priority {} to keep this result bounded.",
                metrics.opportunities, metrics.returned_opportunities
            ),
        );
    }
    let summary = build_summary(&repo_name, &metrics, artifacts.opportunities.first());

    Ok(RefactorScanResult {
        id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        repo_path,
        repo_name,
        summary,
        metrics,
        opportunities: artifacts.opportunities,
        warnings: artifacts.warnings,
    })
}

async fn clone_github_repo(target: &GitHubRepoTarget, destination: &Path) -> Result<()> {
    let timeout_secs = clone_timeout_secs();
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg("credential.helper=")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--single-branch")
        .arg(target.clone_url())
        .arg(destination)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let child = command.spawn().map_err(|err| {
        anyhow!(
            "Could not start git clone for `{}`: {err}. Install git or scan a local path instead.",
            target.label()
        )
    })?;

    let output = tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
        .await
        .map_err(|_| {
            anyhow!(
                "Timed out cloning `{}` after {timeout_secs} seconds.",
                target.label()
            )
        })?
        .map_err(|err| anyhow!("Could not run git clone for `{}`: {err}", target.label()))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Could not clone `{}` from GitHub: {}",
            target.label(),
            command_output_summary(&output)
        ));
    }

    Ok(())
}

fn command_output_summary(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let text = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("git exited with status {}", output.status)
    };
    if text.chars().count() <= MAX_CLONE_ERROR_BYTES {
        text
    } else {
        format!(
            "{}...",
            text.chars().take(MAX_CLONE_ERROR_BYTES).collect::<String>()
        )
    }
}

fn clone_timeout_secs() -> u64 {
    std::env::var("REFACTOR_SCOUT_CLONE_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_CLONE_TIMEOUT_SECS)
}

fn should_use_local_scan(repo_path: &str) -> bool {
    let trimmed = repo_path.trim();
    let path = Path::new(trimmed);
    path.exists() || path.is_absolute() || trimmed.starts_with('.') || trimmed.starts_with('~')
}

pub(crate) fn parse_github_repo_target(input: &str) -> Option<GitHubRepoTarget> {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    if let Some(path) = trimmed.strip_prefix("https://github.com/") {
        return parse_github_path(path);
    }
    if let Some(path) = trimmed.strip_prefix("http://github.com/") {
        return parse_github_path(path);
    }
    if let Some(path) = trimmed.strip_prefix("github.com/") {
        return parse_github_path(path);
    }
    if let Some(path) = trimmed.strip_prefix("git@github.com:") {
        return parse_github_path(path);
    }

    parse_github_path(trimmed)
}

fn parse_github_path(path: &str) -> Option<GitHubRepoTarget> {
    if path.contains('?') || path.contains('#') || path.contains(':') {
        return None;
    }

    let cleaned = path.trim().trim_matches('/').trim_end_matches(".git");
    let mut parts = cleaned.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if parts.next().is_some() || !valid_github_segment(owner) || !valid_github_segment(repo) {
        return None;
    }

    Some(GitHubRepoTarget {
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

fn valid_github_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.len() <= 100
        && segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn sanitize_clone_segment(segment: &str) -> String {
    segment
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect()
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
            "rust" | "javascript" | "typescript" | "go" => {
                match brace_function_end(lines, start, language) {
                    Some(end) => end,
                    None => continue,
                }
            }
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

    for (literal, offset) in source_string_literals(content, language) {
        let literal = literal.trim();
        if should_ignore_literal(literal) {
            continue;
        }

        let entry = literals.entry(literal.to_string()).or_insert((0, offset));
        entry.0 += 1;
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

fn source_string_literals(content: &str, language: &str) -> Vec<(String, usize)> {
    let bytes = content.as_bytes();
    let mut literals = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if language == "python" && bytes[index] == b'#' {
            index = line_end(bytes, index);
            continue;
        }
        if language != "python" && bytes.get(index..index + 2) == Some(b"//") {
            index = line_end(bytes, index);
            continue;
        }
        if language != "python" && bytes.get(index..index + 2) == Some(b"/*") {
            index = block_comment_end(bytes, index + 2);
            continue;
        }

        if language == "rust" {
            if let Some((content_start, hashes)) = rust_raw_string_open(bytes, index) {
                if let Some((content_end, after)) =
                    rust_raw_string_bounds(bytes, content_start, hashes)
                {
                    literals.push((content[content_start..content_end].to_string(), index));
                    index = after;
                    continue;
                }
            }
        }

        let delimiter = match (language, bytes[index]) {
            ("rust", b'"') | ("go", b'"') => Some(b'"'),
            ("javascript" | "typescript" | "python", b'"') => Some(b'"'),
            ("javascript" | "typescript" | "python", b'\'') => Some(b'\''),
            ("javascript" | "typescript" | "go", b'`') => Some(b'`'),
            _ => None,
        };
        let Some(delimiter) = delimiter else {
            index += 1;
            continue;
        };

        let triple_quoted = language == "python"
            && bytes.get(index..index + 3) == Some(&[delimiter, delimiter, delimiter]);
        let content_start = index + if triple_quoted { 3 } else { 1 };
        let allow_escapes = !(language == "go" && delimiter == b'`');
        let Some((content_end, after)) = quoted_string_bounds(
            bytes,
            content_start,
            delimiter,
            triple_quoted,
            allow_escapes,
        ) else {
            index += 1;
            continue;
        };

        let literal = &content[content_start..content_end];
        let dynamic_template = matches!(language, "javascript" | "typescript")
            && delimiter == b'`'
            && literal.contains("${");
        let python_f_string =
            language == "python" && python_string_prefix(content, index).contains('f');
        if !triple_quoted && !dynamic_template && !python_f_string {
            literals.push((literal.to_string(), index));
        }
        index = after;
    }

    literals
}

fn line_end(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|offset| start + offset + 1)
        .unwrap_or(bytes.len())
}

fn block_comment_end(bytes: &[u8], start: usize) -> usize {
    let mut depth = 1u32;
    let mut index = start;
    while index < bytes.len() {
        if bytes.get(index..index + 2) == Some(b"/*") {
            depth += 1;
            index += 2;
            continue;
        }
        if bytes.get(index..index + 2) == Some(b"*/") {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return index;
            }
            continue;
        }
        index += 1;
    }
    bytes.len()
}

fn rust_raw_string_open(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut cursor = start;
    if bytes.get(cursor) == Some(&b'b') {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'r') {
        return None;
    }
    cursor += 1;

    let mut hashes = 0;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }
    (bytes.get(cursor) == Some(&b'"')).then_some((cursor + 1, hashes))
}

fn rust_raw_string_bounds(
    bytes: &[u8],
    content_start: usize,
    hashes: usize,
) -> Option<(usize, usize)> {
    let mut cursor = content_start;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && (0..hashes).all(|offset| bytes.get(cursor + 1 + offset) == Some(&b'#'))
        {
            return Some((cursor, cursor + hashes + 1));
        }
        cursor += 1;
    }
    None
}

fn quoted_string_bounds(
    bytes: &[u8],
    content_start: usize,
    delimiter: u8,
    triple_quoted: bool,
    allow_escapes: bool,
) -> Option<(usize, usize)> {
    let mut cursor = content_start;
    while cursor < bytes.len() {
        if allow_escapes && bytes[cursor] == b'\\' {
            cursor = (cursor + 2).min(bytes.len());
            continue;
        }
        if triple_quoted {
            if bytes.get(cursor..cursor + 3) == Some(&[delimiter, delimiter, delimiter]) {
                return Some((cursor, cursor + 3));
            }
        } else if bytes[cursor] == delimiter {
            return Some((cursor, cursor + 1));
        }
        cursor += 1;
    }
    None
}

fn python_string_prefix(content: &str, quote_start: usize) -> String {
    let prefix_start = content[..quote_start]
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_alphabetic())
        .last()
        .map(|(index, _)| index)
        .unwrap_or(quote_start);
    content[prefix_start..quote_start].to_ascii_lowercase()
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

fn brace_function_end(lines: &[&str], start: usize, language: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut saw_body = false;
    let mut state = BraceScanState::default();

    for (offset, line) in lines[start..].iter().enumerate() {
        let (opens, closes) = structural_braces(line, language, &mut state);
        saw_body |= opens > 0;
        depth += opens as i32;
        if saw_body {
            depth -= closes as i32;
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

fn structural_braces(line: &str, language: &str, state: &mut BraceScanState) -> (u32, u32) {
    let chars = line.chars().collect::<Vec<_>>();
    let mut opens = 0;
    let mut closes = 0;
    let mut index = 0;

    while index < chars.len() {
        if let Some(hashes) = state.rust_raw_hashes {
            if chars[index] == '"' && raw_string_closes(&chars, index, hashes) {
                state.rust_raw_hashes = None;
                index += hashes + 1;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(delimiter) = state.string_delimiter {
            let ch = chars[index];
            if state.escaped {
                state.escaped = false;
            } else if ch == '\\' && (delimiter != '`' || language != "go") {
                state.escaped = true;
            } else if ch == delimiter {
                state.string_delimiter = None;
            }
            index += 1;
            continue;
        }

        if state.block_comment_depth > 0 {
            if pair_at(&chars, index, '/', '*') {
                state.block_comment_depth += 1;
                index += 2;
                continue;
            }
            if pair_at(&chars, index, '*', '/') {
                state.block_comment_depth -= 1;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if pair_at(&chars, index, '/', '/') {
            break;
        }
        if pair_at(&chars, index, '/', '*') {
            state.block_comment_depth = 1;
            index += 2;
            continue;
        }

        if language == "rust" {
            if let Some((hashes, prefix_len)) = rust_raw_string_start(&chars, index) {
                state.rust_raw_hashes = Some(hashes);
                index += prefix_len;
                continue;
            }
            if chars[index] == '\'' {
                if let Some(end) = rust_char_literal_end(&chars, index) {
                    index = end + 1;
                    continue;
                }
            }
        }

        let ch = chars[index];
        let quoted = ch == '"'
            || (ch == '\'' && language != "rust")
            || (ch == '`' && matches!(language, "javascript" | "typescript" | "go"));
        if quoted {
            state.string_delimiter = Some(ch);
            state.escaped = false;
        } else if ch == '{' {
            opens += 1;
        } else if ch == '}' {
            closes += 1;
        }
        index += 1;
    }

    state.escaped = false;
    (opens, closes)
}

fn pair_at(chars: &[char], index: usize, first: char, second: char) -> bool {
    chars.get(index) == Some(&first) && chars.get(index + 1) == Some(&second)
}

fn rust_raw_string_start(chars: &[char], index: usize) -> Option<(usize, usize)> {
    if chars.get(index) != Some(&'r') {
        return None;
    }

    let mut cursor = index + 1;
    let mut hashes = 0;
    while chars.get(cursor) == Some(&'#') {
        hashes += 1;
        cursor += 1;
    }
    (chars.get(cursor) == Some(&'"')).then_some((hashes, cursor - index + 1))
}

fn raw_string_closes(chars: &[char], quote_index: usize, hashes: usize) -> bool {
    (0..hashes).all(|offset| chars.get(quote_index + 1 + offset) == Some(&'#'))
}

fn rust_char_literal_end(chars: &[char], start: usize) -> Option<usize> {
    match (chars.get(start + 1), chars.get(start + 2)) {
        (Some('\\'), Some('u')) if chars.get(start + 3) == Some(&'{') => {
            let closing_brace = chars[start + 4..]
                .iter()
                .position(|ch| *ch == '}')
                .map(|offset| start + 4 + offset)?;
            (chars.get(closing_brace + 1) == Some(&'\'')).then_some(closing_brace + 1)
        }
        (Some('\\'), Some(_)) if chars.get(start + 3) == Some(&'\'') => Some(start + 3),
        (Some(_), Some('\'')) => Some(start + 2),
        _ => None,
    }
}

fn python_function_end(lines: &[&str], start: usize) -> usize {
    let mut body_indent = None;
    let mut end = start;

    for (index, line) in lines.iter().enumerate().skip(start + 1) {
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
            | ".vite"
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

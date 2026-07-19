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
use super::scan_hygiene::{
    classify_function_shape, classify_literal_context, coherent_contract_usage,
    control_flow_markers, is_generated_source, is_non_actionable_style_literal,
    is_style_literal_usage, is_template_placeholder, FunctionShape, LiteralContext,
};

pub(crate) const MAX_SCAN_FILES: u32 = 1_500;
pub(crate) const MAX_FILE_BYTES: u64 = 350_000;
pub(crate) const LONG_FILE_THRESHOLD: usize = 320;
pub(crate) const LONG_FUNCTION_THRESHOLD: usize = 60;
pub(crate) const REPEATED_LITERAL_MIN_LEN: usize = 12;
pub(crate) const REPEATED_LITERAL_MIN_REPEATS: u32 = 3;
pub(crate) const MAX_WARNINGS: usize = 12;
const DEFAULT_CLONE_TIMEOUT_SECS: u64 = 120;
const MAX_CLONE_ERROR_BYTES: usize = 600;
const SUPPORT_CODE_SCORE_PENALTY: u32 = 10;

static RUST_FN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)")
        .expect("rust function regex should compile")
});
static RUST_MOD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+[A-Za-z_][A-Za-z0-9_]*")
        .expect("rust module regex should compile")
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
    if let Some(target) = github_repo_target_for_input(repo_path) {
        return build_github_scan_result(target, max_files).await;
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
            .then_with(|| opportunity_span(right).cmp(&opportunity_span(left)))
            .then_with(|| left.path.cmp(&right.path))
    });
    let metrics = build_metrics(
        artifacts.files_scanned,
        artifacts.files_skipped,
        &artifacts.opportunities,
    );
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
        trigger_type: "operator".into(),
        schedule_name: None,
        target_selection_mode: patchhive_product_core::contract::TargetSelectionMode::Direct,
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

pub(crate) fn should_use_local_scan(repo_path: &str) -> bool {
    let trimmed = repo_path.trim();
    let path = Path::new(trimmed);
    path.exists() || path.is_absolute() || trimmed.starts_with('.') || trimmed.starts_with('~')
}

pub(crate) fn github_repo_target_for_input(input: &str) -> Option<GitHubRepoTarget> {
    (!should_use_local_scan(input))
        .then(|| parse_github_repo_target(input))
        .flatten()
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
        .sort_by_file_name()
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
    if is_generated_source(path, content) {
        return Vec::new();
    }

    let lines = content.lines().collect::<Vec<_>>();
    let inline_test_ranges = if language == "rust" {
        rust_inline_test_module_ranges(content)
    } else {
        Vec::new()
    };
    let inline_test_line_ranges = byte_ranges_to_line_ranges(content, &inline_test_ranges);
    let measured_lines = lines
        .len()
        .saturating_sub(lines_in_ranges(&inline_test_line_ranges));
    let mut opportunities = Vec::new();

    if measured_lines > LONG_FILE_THRESHOLD {
        opportunities.push(large_file_opportunity(
            path,
            language,
            measured_lines,
            lines.len(),
        ));
    }

    opportunities.extend(long_function_opportunities(
        path,
        language,
        &lines,
        &inline_test_line_ranges,
    ));
    if let Some(opportunity) =
        repeated_literal_opportunity(path, language, content, &inline_test_ranges)
    {
        opportunities.push(opportunity);
    }

    if is_test_or_fixture_path(path) {
        for opportunity in &mut opportunities {
            opportunity.score = opportunity.score.saturating_sub(SUPPORT_CODE_SCORE_PENALTY);
            if opportunity.safety == "high" {
                opportunity.safety = "medium".into();
            }
            opportunity.evidence.push(
                "Test or fixture code remains a closer-review candidate rather than high-confidence production refactor evidence.".into(),
            );
        }
    }

    opportunities
}

fn opportunity_span(opportunity: &RefactorOpportunity) -> u32 {
    opportunity
        .line_end
        .saturating_sub(opportunity.line_start)
        .saturating_add(1)
}

fn bounded_score_bonus(excess: usize, max_bonus: u32, half_saturation: usize) -> u32 {
    if excess == 0 {
        return 0;
    }

    ((excess as u64 * max_bonus as u64) / (excess + half_saturation) as u64) as u32
}

fn adjusted_score(score: u32, adjustment: i32) -> u32 {
    if adjustment.is_negative() {
        score.saturating_sub(adjustment.unsigned_abs())
    } else {
        score.saturating_add(adjustment as u32).min(94)
    }
}

fn large_file_opportunity(
    path: &str,
    language: &str,
    measured_lines: usize,
    total_lines: usize,
) -> RefactorOpportunity {
    let score = 50 + bounded_score_bonus(measured_lines - LONG_FILE_THRESHOLD, 31, 180);
    let excluded_test_lines = total_lines.saturating_sub(measured_lines);
    let mut evidence = vec![
        format!("{measured_lines} measured non-test lines"),
        "File size is a review signal, not proof that a cohesive extraction exists.".into(),
    ];
    if excluded_test_lines > 0 {
        evidence.push(format!(
            "{excluded_test_lines} inline test-module lines excluded from measurement"
        ));
    }

    RefactorOpportunity {
        id: Uuid::new_v4().to_string(),
        kind: "large_file".into(),
        title: "Review oversized file boundary".into(),
        summary: format!(
            "`{path}` contains {measured_lines} measured non-test lines. Its size makes it a review candidate, but the scanner cannot infer a safe split from line count alone."
        ),
        path: path.into(),
        language: language.into(),
        score,
        safety: "medium".into(),
        effort: "medium".into(),
        line_start: 1,
        line_end: total_lines as u32,
        suggestion: "Review module responsibilities and change history first. Propose a boundary only when one responsibility can be isolated with focused tests and a stable public surface.".into(),
        evidence,
    }
}

fn long_function_opportunities(
    path: &str,
    language: &str,
    lines: &[&str],
    excluded_line_ranges: &[(usize, usize)],
) -> Vec<RefactorOpportunity> {
    let mut opportunities = Vec::new();
    let eligible_starts = function_start_eligibility(lines, language);

    for start in 0..lines.len() {
        if line_is_in_ranges(start, excluded_line_ranges) || !eligible_starts[start] {
            continue;
        }
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

        let body = &lines[start..=end];
        let shape = classify_function_shape(language, &name, lines, start, end);
        let control_markers = control_flow_markers(body);
        if !shape.has_review_evidence(line_count, control_markers) {
            continue;
        }
        let base_score = 52 + bounded_score_bonus(line_count - LONG_FUNCTION_THRESHOLD, 31, 120);
        let score = adjusted_score(
            base_score,
            shape.score_adjustment() + control_markers.min(8) as i32,
        );
        let (title, summary, suggestion, evidence) = match shape {
            FunctionShape::EmbeddedStylesheet => (
                format!("Review embedded stylesheet boundary in `{name}`"),
                format!(
                    "`{name}` in `{path}` contains a {line_count}-line embedded stylesheet ({}-{}). It is declarative styling, not a complex-function finding.",
                    start + 1,
                    end + 1
                ),
                "Review whether the stylesheet belongs in a product stylesheet or CSS module. Preserve selector scope and loading behavior if it moves.".into(),
                vec![
                    format!("{line_count} lines in one embedded stylesheet"),
                    "Classified as declarative styling rather than branching application logic.".into(),
                ],
            ),
            FunctionShape::DeclarativeJsx => (
                format!("Review large declarative component `{name}`"),
                format!(
                    "`{name}` in `{path}` spans {line_count} lines ({}-{}), but most of the body is declarative JSX rather than control-flow complexity.",
                    start + 1,
                    end + 1
                ),
                "Review component responsibilities and rendering boundaries. Extract only when a child surface has a stable purpose, inputs, and focused tests.".into(),
                vec![
                    format!("{line_count} lines in one JSX component"),
                    format!("{control_markers} control-flow markers"),
                    "Classified as declarative JSX; line count receives a lower score.".into(),
                ],
            ),
            FunctionShape::SqlSchema => (
                format!("Review schema setup boundary in `{name}`"),
                format!(
                    "`{name}` in `{path}` spans {line_count} lines ({}-{}), primarily in SQL or schema declarations rather than branching application logic.",
                    start + 1,
                    end + 1
                ),
                "Review migration ownership and transactional boundaries before splitting declarations. Keep schema ordering and rollback behavior explicit.".into(),
                vec![
                    format!("{line_count} lines in one schema-oriented function"),
                    "Classified as SQL/schema declarations; line count receives a lower score.".into(),
                ],
            ),
            FunctionShape::LookupTable => (
                format!("Review declarative table in `{name}`"),
                format!(
                    "`{name}` in `{path}` spans {line_count} lines ({}-{}), with a table or match-heavy shape rather than dense branching logic.",
                    start + 1,
                    end + 1
                ),
                "Review whether the table is easier to validate in place or as named data. Do not extract it solely to reduce the function's line count.".into(),
                vec![
                    format!("{line_count} lines in one table-oriented function"),
                    "Classified as a declarative lookup/table; line count receives a lower score.".into(),
                ],
            ),
            FunctionShape::ComplexLogic => (
                format!("Review responsibilities in `{name}`"),
                format!(
                    "`{name}` in `{path}` spans {line_count} lines ({}-{}) with {control_markers} control-flow markers. Review it for separable responsibilities before proposing an extraction.",
                    start + 1,
                    end + 1
                ),
                "Map the function's phases and tests first. Extract one named responsibility only when its inputs, outputs, and failure behavior are clear.".into(),
                vec![
                    format!("{line_count} lines in one function"),
                    format!("{control_markers} control-flow markers"),
                    format!("Detected in {language} code"),
                ],
            ),
        };
        opportunities.push(RefactorOpportunity {
            id: Uuid::new_v4().to_string(),
            kind: "long_function".into(),
            title,
            summary,
            path: path.into(),
            language: language.into(),
            score,
            safety: "medium".into(),
            effort: "medium".into(),
            line_start: (start + 1) as u32,
            line_end: (end + 1) as u32,
            suggestion,
            evidence,
        });
    }

    opportunities
}

fn function_start_eligibility(lines: &[&str], language: &str) -> Vec<bool> {
    let mut state = BraceScanState::default();
    let mut eligible = Vec::with_capacity(lines.len());

    for line in lines {
        eligible.push(
            state.block_comment_depth == 0
                && state.string_delimiter.is_none()
                && state.rust_raw_hashes.is_none(),
        );
        structural_braces(line, language, &mut state);
    }

    eligible
}

fn repeated_literal_opportunity(
    path: &str,
    language: &str,
    content: &str,
    inline_test_ranges: &[(usize, usize)],
) -> Option<RefactorOpportunity> {
    let mut literals: HashMap<String, (u32, usize, Vec<usize>)> = HashMap::new();

    for (literal, offset) in source_string_literals(content, language) {
        if is_non_extractable_literal_context(content, language, offset, inline_test_ranges) {
            continue;
        }

        let literal = literal.trim();
        if should_ignore_literal(literal) {
            continue;
        }

        let entry = literals
            .entry(literal.to_string())
            .or_insert_with(|| (0, offset, Vec::new()));
        entry.0 += 1;
        entry.2.push(offset);
    }

    let (literal, (count, first_offset, offsets)) = literals
        .into_iter()
        .filter(|(literal, (count, _, offsets))| {
            if literal.len() < REPEATED_LITERAL_MIN_LEN || *count < REPEATED_LITERAL_MIN_REPEATS {
                return false;
            }

            if repeated_validation_pattern(content, offsets) {
                return true;
            }

            match classify_literal_context(literal, content, offsets) {
                LiteralContext::Contract => true,
                LiteralContext::General => *count >= 4,
                LiteralContext::UiCopy => *count >= 5,
            }
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
    let validation_pattern = repeated_validation_pattern(content, &offsets);
    let literal_context = classify_literal_context(&literal, content, &offsets);
    let machine_readable_error_contract =
        validation_pattern && machine_readable_contract_value(&literal);
    let (kind, title, summary, suggestion, context_evidence, score, safety) =
        if machine_readable_error_contract {
            (
                "repeated_validation",
                "Review repeated error contract",
                format!(
                    "`{path}` repeats the machine-readable error value `{preview}` {count} times in similar error paths. Review whether those paths share one error contract before centralizing it."
                ),
                "Compare the error semantics and consumers first. If the value is one stable contract, centralize it while preserving the exact machine-readable value and adding focused tests.",
                "The repeated value is a machine-readable identifier used in similar error paths."
                    .to_string(),
                76 + bounded_score_bonus(count.saturating_sub(3) as usize, 12, 4),
                "high",
            )
        } else if validation_pattern {
            (
            "repeated_validation",
            "Review repeated validation boundary",
            format!(
                "`{path}` repeats the validation message `{preview}` {count} times in similar error paths. Review whether those paths enforce one contract before consolidating them."
            ),
            "Compare the guards and error semantics first. If they enforce the same contract, consolidate the boundary while preserving exact behavior and adding focused tests.",
            "The repeated message appears in similar validation or error-return paths."
                .to_string(),
            76 + bounded_score_bonus(count.saturating_sub(3) as usize, 12, 4),
            "high",
        )
        } else if literal_context == LiteralContext::Contract {
            let coherent_usage = coherent_contract_usage(&literal, content, &offsets);
            let safety = if count >= 5 && coherent_usage.is_some() {
                "high"
            } else {
                "medium"
            };
            let context_evidence = coherent_usage.map_or_else(
            || {
                "The literal looks contract-shaped, but its surrounding usage does not establish one shared ownership boundary.".to_string()
            },
            |role| format!("Every occurrence has the same {role} role."),
        );
            (
            "repeated_literal",
            "Review repeated contract literal",
            format!(
                "`{path}` repeats the contract-shaped string `{preview}` {count} times. Review whether every occurrence represents the same protocol, configuration, route, or machine-readable value."
            ),
            "Compare the consumers first. Introduce a named constant only when all occurrences must evolve together.",
            context_evidence,
            62 + bounded_score_bonus(count.saturating_sub(3) as usize, 18, 5),
            safety,
        )
        } else if literal_context == LiteralContext::UiCopy {
            (
            "repeated_literal",
            "Review repeated interface copy",
            format!(
                "`{path}` repeats the interface text `{preview}` {count} times. Shared wording may be intentional, but occurrence count alone does not justify a constant."
            ),
            "Check whether the text is one product concept or separate labels that may diverge. Share it only when synchronized wording is a real requirement.",
            "Most occurrences appear in labels, titles, placeholders, or other interface copy."
                .to_string(),
            44 + bounded_score_bonus(count.saturating_sub(3) as usize, 13, 7),
            "medium",
        )
        } else {
            (
            "repeated_literal",
            "Review repeated string usage",
            format!(
                "`{path}` repeats the string `{preview}` {count} times. Review semantic ownership before deciding whether a shared constant improves the code."
            ),
            "Compare the surrounding responsibilities first. Keep the values inline when proximity is clearer; share them only when they represent one concept.",
            "The occurrences are real, but their surrounding contexts do not prove shared ownership."
                .to_string(),
            50 + bounded_score_bonus(count.saturating_sub(3) as usize, 16, 6),
            "medium",
        )
        };

    let mut evidence = vec![format!("{count} repeated occurrences"), context_evidence];
    if safety == "medium" {
        evidence.push(
            "Confirm the occurrences represent the same concept before sharing one constant."
                .into(),
        );
    }

    Some(RefactorOpportunity {
        id: Uuid::new_v4().to_string(),
        kind: kind.into(),
        title: title.into(),
        summary,
        path: path.into(),
        language: language.into(),
        score,
        safety: safety.into(),
        effort: "low".into(),
        line_start: line,
        line_end: line,
        suggestion: suggestion.into(),
        evidence,
    })
}

fn is_non_extractable_literal_context(
    content: &str,
    language: &str,
    offset: usize,
    inline_test_ranges: &[(usize, usize)],
) -> bool {
    (language == "rust"
        && (is_inside_rust_attribute(content, offset)
            || inline_test_ranges
                .iter()
                .any(|(start, end)| (*start..*end).contains(&offset))))
        || (matches!(language, "javascript" | "typescript")
            && is_style_literal_usage(content, offset))
}

fn is_inside_rust_attribute(content: &str, offset: usize) -> bool {
    let prefix = &content[..offset];
    let Some(attribute_start) = prefix.rfind("#[") else {
        return false;
    };
    prefix
        .rfind(']')
        .is_none_or(|attribute_end| attribute_start > attribute_end)
}

fn rust_inline_test_module_ranges(content: &str) -> Vec<(usize, usize)> {
    let lines = content.lines().collect::<Vec<_>>();
    let mut line_offsets = vec![0usize];
    line_offsets.extend(
        content
            .match_indices('\n')
            .map(|(newline_offset, _)| newline_offset + 1),
    );
    let mut ranges = Vec::new();

    for cfg_line in 0..lines.len() {
        if lines[cfg_line].trim() != "#[cfg(test)]" {
            continue;
        }

        let mut module_line = None;
        for (index, line) in lines.iter().enumerate().skip(cfg_line + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("#[") {
                continue;
            }
            if RUST_MOD_RE.is_match(line) {
                module_line = Some(index);
            }
            break;
        }
        let Some(module_line) = module_line else {
            continue;
        };

        let Some(end_line) = brace_function_end(&lines, module_line, "rust") else {
            continue;
        };
        let start = line_offsets[cfg_line];
        let end = line_offsets
            .get(end_line + 1)
            .copied()
            .unwrap_or(content.len());
        ranges.push((start, end));
    }

    ranges
}

fn byte_ranges_to_line_ranges(
    content: &str,
    byte_ranges: &[(usize, usize)],
) -> Vec<(usize, usize)> {
    byte_ranges
        .iter()
        .map(|(start, end)| {
            let start_line = content[..*start]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count();
            let mut end_line = content[..*end]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count();
            if *end == content.len() && !content.ends_with('\n') {
                end_line += 1;
            }
            (start_line, end_line)
        })
        .collect()
}

fn lines_in_ranges(ranges: &[(usize, usize)]) -> usize {
    ranges
        .iter()
        .map(|(start, end)| end.saturating_sub(*start))
        .sum()
}

fn line_is_in_ranges(line: usize, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| (*start..*end).contains(&line))
}

fn repeated_validation_pattern(content: &str, offsets: &[usize]) -> bool {
    let validation_hits = offsets
        .iter()
        .filter(|offset| {
            let context = nearby_source_lines(content, **offset).to_ascii_lowercase();
            [
                "return err",
                "anyhow!",
                "bail!",
                "ensure!",
                "ok_or",
                "raise ",
                "throw ",
                "panic!",
            ]
            .iter()
            .any(|marker| context.contains(marker))
        })
        .count();

    validation_hits >= REPEATED_LITERAL_MIN_REPEATS as usize && validation_hits * 2 >= offsets.len()
}

fn machine_readable_contract_value(literal: &str) -> bool {
    !literal.is_empty()
        && !literal.chars().any(char::is_whitespace)
        && literal
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._:/-".contains(character))
        && literal
            .chars()
            .any(|character| matches!(character, '_' | '-' | ':' | '/'))
}

fn nearby_source_lines(content: &str, offset: usize) -> &str {
    let current_start = content[..offset]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let previous_start = current_start
        .checked_sub(1)
        .and_then(|index| content[..index].rfind('\n'))
        .map(|index| index + 1)
        .unwrap_or(0);
    let current_end = content[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(content.len());
    &content[previous_start..current_end]
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
            if bytes[index] == b'\'' {
                if let Some(after) = rust_char_literal_after(bytes, index) {
                    index = after;
                    continue;
                }
            }
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

fn rust_char_literal_after(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'\'') {
        return None;
    }

    let content_start = start + 1;
    let mut cursor = content_start;
    if bytes.get(cursor) == Some(&b'\\') {
        cursor += 1;
        if bytes.get(cursor) == Some(&b'u') && bytes.get(cursor + 1) == Some(&b'{') {
            cursor += 2;
            while bytes.get(cursor).is_some_and(|byte| *byte != b'}') {
                cursor += 1;
            }
            if bytes.get(cursor) != Some(&b'}') {
                return None;
            }
            cursor += 1;
        } else {
            cursor += 1;
        }
    } else {
        let text = std::str::from_utf8(bytes.get(content_start..)?).ok()?;
        cursor += text.chars().next()?.len_utf8();
    }

    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
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

    !is_ignored_directory(name)
}

fn is_ignored_directory(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".cache"
            | ".git"
            | ".gradle"
            | ".mypy_cache"
            | ".next"
            | ".nox"
            | ".nuxt"
            | ".output"
            | ".parcel-cache"
            | ".pnpm-store"
            | ".pytest_cache"
            | ".ruff_cache"
            | ".svelte-kit"
            | ".tox"
            | ".turbo"
            | ".venv"
            | ".vite"
            | ".yarn"
            | "__pycache__"
            | "bower_components"
            | "build"
            | "coverage"
            | "dist"
            | "node_modules"
            | "out"
            | "storybook-static"
            | "target"
            | "vendor"
    )
}

fn is_test_or_fixture_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let filename = normalized.rsplit('/').next().unwrap_or(&normalized);

    normalized.starts_with("tests/")
        || normalized.starts_with("test/")
        || normalized.starts_with("fixtures/")
        || normalized.contains("/tests/")
        || normalized.contains("/test/")
        || normalized.contains("/__tests__/")
        || normalized.contains("/fixtures/")
        || filename.contains(".test.")
        || filename.contains(".spec.")
        || (filename.starts_with("test_") && filename.ends_with(".py"))
        || filename.ends_with("_test.py")
        || filename.ends_with("_test.go")
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
        || is_non_actionable_style_literal(trimmed)
        || is_template_placeholder(trimmed)
}

fn literal_preview(literal: &str) -> String {
    let sanitized = literal.replace('`', "'");
    if sanitized.len() <= 48 {
        sanitized
    } else {
        format!("{}...", &sanitized[..45])
    }
}

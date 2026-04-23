use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use axum::{
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use once_cell::sync::Lazy;
use patchhive_product_core::contract;
use patchhive_product_core::startup::count_errors;
use regex::Regex;
use serde_json::json;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

use crate::{
    auth::{
        auth_enabled, generate_and_save_key, generate_and_save_service_token,
        service_auth_enabled, service_token_generation_allowed, verify_token,
    },
    db,
    models::{
        HistoryItem, OverviewPayload, RefactorOpportunity, RefactorScanResult, ScanMetrics,
        ScanRequest,
    },
    state::{path_within_allowed_roots, AppState},
    STARTUP_CHECKS,
};

type ApiError = (StatusCode, Json<serde_json::Value>);
type JsonResult<T> = Result<Json<T>, ApiError>;

pub async fn capabilities() -> Json<contract::ProductCapabilities> {
    Json(contract::capabilities(
        "refactor-scout",
        "RefactorScout",
        vec![contract::action(
            "scan_local_repo",
            "Scan local repo",
            "POST",
            "/scan/local",
            "Surface safe refactor opportunities from an allowed local repository path.",
            true,
        )],
        vec![
            contract::link("overview", "Overview", "/overview"),
            contract::link("history", "History", "/history"),
        ],
    ))
}

pub async fn runs() -> Json<contract::ProductRunsResponse> {
    Json(contract::runs_from_history(
        "refactor-scout",
        db::history(30),
    ))
}

const MAX_SCAN_FILES: u32 = 1_500;
const MAX_RETURNED_OPPORTUNITIES: usize = 60;
const MAX_FILE_BYTES: u64 = 350_000;
const LONG_FILE_THRESHOLD: usize = 320;
const LONG_FUNCTION_THRESHOLD: usize = 60;
const REPEATED_LITERAL_MIN_LEN: usize = 12;
const REPEATED_LITERAL_MIN_REPEATS: u32 = 3;
const MAX_WARNINGS: usize = 12;

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

#[derive(serde::Deserialize)]
pub struct LoginBody {
    api_key: String,
}

struct ScanArtifacts {
    opportunities: Vec<RefactorOpportunity>,
    warnings: Vec<String>,
    files_scanned: u32,
    files_skipped: u32,
    limit_hit: bool,
}

pub async fn auth_status() -> Json<serde_json::Value> {
    Json(crate::auth::auth_status_payload())
}

pub async fn login(Json(body): Json<LoginBody>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth_enabled() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if !verify_token(&body.api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(
        json!({"ok": true, "auth_enabled": true, "auth_configured": true}),
    ))
}

pub async fn gen_key(
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if auth_enabled() {
        return Err(patchhive_product_core::auth::auth_already_configured_error());
    }
    if !crate::auth::bootstrap_request_allowed(&headers) {
        return Err(patchhive_product_core::auth::bootstrap_localhost_required_error());
    }
    let key = generate_and_save_key()
        .map_err(|err| patchhive_product_core::auth::key_generation_failed_error(&err))?;
    Ok(Json(
        json!({"api_key": key, "message": "Store this — it won't be shown again"}),
    ))
}

pub async fn gen_service_token(
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, patchhive_product_core::auth::JsonApiError> {
    if service_auth_enabled() {
        return Err(patchhive_product_core::auth::service_auth_already_configured_error());
    }
    if !service_token_generation_allowed(&headers) {
        return Err(patchhive_product_core::auth::service_token_generation_forbidden_error());
    }
    let token = generate_and_save_service_token()
        .map_err(|err| patchhive_product_core::auth::service_token_generation_failed_error(&err))?;
    Ok(Json(json!({
        "service_token": token,
        "message": "Store this for HiveCore or other PatchHive service callers — it won't be shown again"
    })))
}

pub async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();
    let counts = db::overview_counts();

    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "RefactorScout by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "scan_count": counts.scans,
        "repo_count": counts.repos,
        "opportunity_count": counts.opportunities,
        "high_safety_count": counts.high_safety,
        "allowed_roots": state.allowed_root_labels(),
        "remote_fs_enabled": state.remote_fs_enabled,
        "mode": "local-refactor-scout",
    }))
}

pub async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

pub async fn overview(State(state): State<AppState>) -> Json<OverviewPayload> {
    let counts = db::overview_counts();
    Json(OverviewPayload {
        product: "RefactorScout by PatchHive".into(),
        tagline: "Surface safe, high-value refactors before code quality drift turns expensive."
            .into(),
        scan_count: counts.scans,
        repo_count: counts.repos,
        opportunity_count: counts.opportunities,
        high_safety_count: counts.high_safety,
        large_file_count: counts.large_file_count,
        long_function_count: counts.long_function_count,
        repeated_literal_count: counts.repeated_literal_count,
        last_repo: counts.last_repo,
        allowed_roots: state.allowed_root_labels(),
        remote_fs_enabled: state.remote_fs_enabled,
    })
}

pub async fn history() -> Json<Vec<HistoryItem>> {
    Json(db::history(30))
}

pub async fn history_detail(AxumPath(id): AxumPath<String>) -> JsonResult<RefactorScanResult> {
    db::get_scan(&id)
        .map(Json)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "RefactorScout scan not found"))
}

pub async fn scan_local_repo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ScanRequest>,
) -> JsonResult<RefactorScanResult> {
    if !scan_request_allowed(&headers, state.remote_fs_enabled) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Filesystem scans are limited to localhost callers unless REFACTOR_SCOUT_ALLOW_REMOTE_FS=true.",
        ));
    }

    let repo_path = request.repo_path.trim();
    if repo_path.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Repository path is required.",
        ));
    }

    let max_files = request.max_files.clamp(25, MAX_SCAN_FILES);
    let result = build_scan_result(&state, repo_path, max_files)
        .map_err(|err| api_error(StatusCode::BAD_REQUEST, err.to_string()))?;

    db::save_scan(&result)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(result))
}

fn api_error(status: StatusCode, error: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": error.into() })))
}

fn build_scan_result(
    state: &AppState,
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

fn resolve_scan_root(repo_path: &str, state: &AppState) -> Result<PathBuf> {
    let candidate = PathBuf::from(repo_path);
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
    if !path_within_allowed_roots(&canonical, &allowed_roots) {
        return Err(anyhow!(
            "`{}` is outside the configured allowed roots.",
            canonical.display()
        ));
    }

    Ok(canonical)
}

fn scan_repo(root: &Path, max_files: u32) -> Result<ScanArtifacts> {
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

fn analyze_file(path: &str, language: &str, content: &str) -> Vec<RefactorOpportunity> {
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
    let literal_matchers: &[&Regex] = if language == "rust" {
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

fn build_metrics(
    files_scanned: u32,
    files_skipped: u32,
    opportunities: &[RefactorOpportunity],
) -> ScanMetrics {
    let mut metrics = ScanMetrics {
        files_scanned,
        files_skipped,
        opportunities: opportunities.len() as u32,
        ..ScanMetrics::default()
    };

    for opportunity in opportunities {
        match opportunity.safety.as_str() {
            "high" => metrics.high_safety += 1,
            _ => metrics.medium_safety += 1,
        }

        match opportunity.kind.as_str() {
            "large_file" => metrics.large_file_count += 1,
            "long_function" => metrics.long_function_count += 1,
            "repeated_literal" => metrics.repeated_literal_count += 1,
            _ => {}
        }
    }

    metrics
}

fn build_summary(
    repo_name: &str,
    metrics: &ScanMetrics,
    top: Option<&RefactorOpportunity>,
) -> String {
    if metrics.opportunities == 0 {
        return format!(
            "RefactorScout did not find clear low-risk refactor candidates in `{repo_name}` within the current scan limits."
        );
    }

    let mut summary = format!(
        "RefactorScout found {} candidate{} across {} scanned file{}. {} high-safety lead{}, {} medium-safety lead{}.",
        metrics.opportunities,
        plural_suffix(metrics.opportunities),
        metrics.files_scanned,
        plural_suffix(metrics.files_scanned),
        metrics.high_safety,
        plural_suffix(metrics.high_safety),
        metrics.medium_safety,
        plural_suffix(metrics.medium_safety),
    );

    if let Some(top) = top {
        summary.push_str(&format!(" Strongest lead: {}.", top.summary));
    }

    summary
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

fn safety_rank(safety: &str) -> u8 {
    match safety {
        "high" => 2,
        _ => 1,
    }
}

fn plural_suffix(value: u32) -> &'static str {
    if value == 1 {
        ""
    } else {
        "s"
    }
}

fn push_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < MAX_WARNINGS {
        warnings.push(warning);
    }
}

fn scan_request_allowed(headers: &HeaderMap, remote_fs_enabled: bool) -> bool {
    if remote_fs_enabled {
        return true;
    }

    let mut saw_browser_local_hint = false;

    for header in ["origin", "referer"] {
        if let Some(value) = headers.get(header).and_then(|value| value.to_str().ok()) {
            if !local_endpoint(value) {
                return false;
            }
            saw_browser_local_hint = true;
        }
    }

    if saw_browser_local_hint {
        return true;
    }

    if let Some(value) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        let client = value.split(',').next().unwrap_or("").trim();
        if !matches!(client, "" | "127.0.0.1" | "::1" | "[::1]") {
            return false;
        }
    }

    headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .map(local_endpoint)
        .unwrap_or(false)
}

fn local_endpoint(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    let without_scheme = trimmed
        .split("://")
        .nth(1)
        .unwrap_or(trimmed)
        .split('/')
        .next()
        .unwrap_or(trimmed)
        .trim();

    let host = without_scheme
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(':')
        .next()
        .unwrap_or(without_scheme)
        .trim();

    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

#[cfg(test)]
mod tests {
    use super::{analyze_file, build_summary, resolve_scan_root};
    use crate::{models::ScanMetrics, state::AppState};
    use std::{fs, path::PathBuf};

    #[test]
    fn analyze_file_surfaces_large_file_and_long_function() {
        let mut source = String::from("fn huge_function() {\n");
        for line in 0..85 {
            source.push_str(&format!("    println!(\"line {line}\");\n"));
        }
        source.push_str("}\n");
        for _ in 0..260 {
            source.push_str("// filler\n");
        }

        let opportunities = analyze_file("src/lib.rs", "rust", &source);
        assert!(opportunities.iter().any(|item| item.kind == "large_file"));
        assert!(opportunities
            .iter()
            .any(|item| item.kind == "long_function"));
    }

    #[test]
    fn analyze_file_surfaces_repeated_literal_candidate() {
        let source = r#"
const A = "service unavailable while syncing billing customers";
const B = "service unavailable while syncing billing customers";
const C = "service unavailable while syncing billing customers";
"#;

        let opportunities = analyze_file("src/client.ts", "typescript", source);
        assert!(opportunities
            .iter()
            .any(|item| item.kind == "repeated_literal" && item.safety == "high"));
    }

    #[test]
    fn build_summary_handles_empty_scan_cleanly() {
        let summary = build_summary("example", &ScanMetrics::default(), None);
        assert!(summary.contains("did not find clear low-risk refactor candidates"));
    }

    #[test]
    fn resolve_scan_root_rejects_paths_outside_allowed_roots() {
        let base =
            std::env::temp_dir().join(format!("refactor-scout-test-{}", uuid::Uuid::new_v4()));
        let allowed = base.join("allowed");
        let outside = base.join("outside");
        fs::create_dir_all(&allowed).expect("allowed dir should exist");
        fs::create_dir_all(&outside).expect("outside dir should exist");

        let state = AppState {
            allowed_roots: vec![allowed.clone()],
            remote_fs_enabled: false,
        };

        let err = resolve_scan_root(outside.to_str().expect("utf8 path"), &state)
            .expect_err("outside root should be rejected");
        assert!(err
            .to_string()
            .contains("outside the configured allowed roots"));

        fs::remove_dir_all(PathBuf::from(base)).ok();
    }
}

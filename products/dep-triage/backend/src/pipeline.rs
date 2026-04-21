use std::collections::{BTreeSet, HashMap};

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use patchhive_product_core::contract;
use patchhive_product_core::startup::count_errors;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{auth_enabled, generate_and_save_key, verify_token},
    db, github,
    models::{
        DependencyAlertRef, DependencyPullRef, DependencyTriageItem, HistoryItem, OverviewPayload,
        ScanRequest, TriageMetrics, TriageScanResult,
    },
    state::AppState,
    STARTUP_CHECKS,
};

type ApiError = (StatusCode, Json<serde_json::Value>);
type JsonResult<T> = Result<Json<T>, ApiError>;

#[derive(serde::Deserialize)]
pub struct LoginBody {
    api_key: String,
}

pub async fn capabilities() -> Json<contract::ProductCapabilities> {
    Json(contract::capabilities(
        "dep-triage",
        "DepTriage",
        vec![contract::action(
            "scan_github_dependencies",
            "Scan GitHub dependencies",
            "POST",
            "/scan/github/dependencies",
            "Rank dependency PRs and alerts into update, watch, and ignore decisions.",
            true,
        )],
        vec![
            contract::link("overview", "Overview", "/overview"),
            contract::link("history", "History", "/history"),
        ],
    ))
}

pub async fn runs() -> Json<contract::ProductRunsResponse> {
    Json(contract::runs_from_history("dep-triage", db::history(30)))
}

#[derive(Default)]
struct Builder {
    package_name: String,
    ecosystem: String,
    strongest_update_kind: String,
    runtime_labels: BTreeSet<String>,
    manifests: BTreeSet<String>,
    changed_paths: BTreeSet<String>,
    stale_days: u32,
    pull_requests: Vec<DependencyPullRef>,
    alerts: Vec<DependencyAlertRef>,
    reasons: BTreeSet<String>,
    evidence: Vec<String>,
    highest_severity: String,
}

struct PullAnalysis {
    package_name: String,
    ecosystem: String,
    update_kind: String,
    runtime_impact: String,
    stale_days: u32,
    source_tool: String,
    manifest_paths: Vec<String>,
    changed_paths: Vec<String>,
    from_version: String,
    to_version: String,
    summary_reason: String,
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

pub async fn health() -> Json<serde_json::Value> {
    let errors = STARTUP_CHECKS
        .get()
        .map(|checks| count_errors(checks))
        .unwrap_or(0);
    let db_ok = db::health_check();
    let counts = db::overview_counts();

    Json(json!({
        "status": if errors > 0 || !db_ok { "degraded" } else { "ok" },
        "version": "0.1.0",
        "product": "DepTriage by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "github_ready": github::github_token_configured(),
        "scan_count": counts.scans,
        "repo_count": counts.repos,
        "tracked_item_count": counts.tracked_items,
        "update_now_count": counts.update_now,
        "watch_count": counts.watch,
        "ignore_count": counts.ignore_for_now,
        "mode": "dependency-triage",
    }))
}

pub async fn startup_checks_route() -> Json<serde_json::Value> {
    Json(json!({"checks": STARTUP_CHECKS.get().cloned().unwrap_or_default()}))
}

pub async fn overview() -> Json<OverviewPayload> {
    Json(db::overview())
}

pub async fn history() -> Json<Vec<HistoryItem>> {
    Json(db::history(30))
}

pub async fn history_detail(Path(id): Path<String>) -> JsonResult<TriageScanResult> {
    db::get_scan(&id)
        .map(Json)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "DepTriage scan not found"))
}

pub async fn scan_github_dependencies(
    State(state): State<AppState>,
    Json(request): Json<ScanRequest>,
) -> JsonResult<TriageScanResult> {
    let repo = request.repo.trim();
    if !valid_repo(repo) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Repository must be in owner/name format.",
        ));
    }

    let pr_limit = request.pr_limit.clamp(5, 60);
    let result = build_scan_result(&state, repo, pr_limit, request.include_alerts)
        .await
        .map_err(upstream_error)?;

    db::save_scan(&result)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(result))
}

fn api_error(status: StatusCode, error: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": error.into() })))
}

fn upstream_error(error: anyhow::Error) -> ApiError {
    api_error(StatusCode::BAD_GATEWAY, error.to_string())
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

async fn build_scan_result(
    state: &AppState,
    repo: &str,
    pr_limit: u32,
    include_alerts: bool,
) -> Result<TriageScanResult> {
    let pulls = github::fetch_pull_requests(&state.http, repo, pr_limit).await?;
    let mut warnings = Vec::new();
    let mut dependency_pull_analyses = Vec::new();

    for pr in &pulls {
        let files = match github::fetch_pull_files(&state.http, repo, pr.number).await {
            Ok(files) => files,
            Err(err) => {
                warnings.push(format!(
                    "Could not inspect changed files for PR #{}: {}",
                    pr.number, err
                ));
                continue;
            }
        };

        if !looks_like_dependency_pr(pr, &files) {
            continue;
        }

        if let Some(analysis) = analyze_pull(pr, &files) {
            dependency_pull_analyses.push((pr.clone(), analysis));
        }
    }

    let alerts = if include_alerts {
        match github::fetch_dependabot_alerts(&state.http, repo, 100).await {
            Ok(alerts) => alerts,
            Err(err) => {
                warnings.push(format!(
                    "Dependabot alerts could not be read for {repo}: {err}"
                ));
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let mut builders: HashMap<String, Builder> = HashMap::new();

    for (pr, analysis) in dependency_pull_analyses {
        let key = triage_key(&analysis.ecosystem, &analysis.package_name, Some(pr.number));
        let builder = builders.entry(key).or_default();
        if builder.package_name.is_empty() {
            builder.package_name = analysis.package_name.clone();
        }
        if builder.ecosystem.is_empty() {
            builder.ecosystem = analysis.ecosystem.clone();
        }
        builder.strongest_update_kind =
            strongest_update_kind(&builder.strongest_update_kind, &analysis.update_kind).into();
        builder
            .runtime_labels
            .insert(analysis.runtime_impact.clone());
        builder.stale_days = builder.stale_days.max(analysis.stale_days);
        builder
            .manifests
            .extend(analysis.manifest_paths.iter().cloned());
        builder
            .changed_paths
            .extend(analysis.changed_paths.iter().cloned());
        builder.reasons.insert(analysis.summary_reason.clone());
        push_evidence(
            &mut builder.evidence,
            format!(
                "PR #{} · {}{}",
                pr.number,
                pr.title,
                if analysis.from_version.is_empty() || analysis.to_version.is_empty() {
                    String::new()
                } else {
                    format!(" ({} → {})", analysis.from_version, analysis.to_version)
                }
            ),
        );
        builder.pull_requests.push(DependencyPullRef {
            number: pr.number,
            title: pr.title.clone(),
            html_url: pr.html_url.clone(),
            updated_at: pr.updated_at.clone(),
            author: pr
                .user
                .as_ref()
                .map(|user| user.login.clone())
                .unwrap_or_default(),
            source_tool: analysis.source_tool,
            from_version: analysis.from_version,
            to_version: analysis.to_version,
            update_kind: analysis.update_kind,
            manifest_paths: analysis.manifest_paths,
            changed_paths: analysis.changed_paths,
        });
    }

    for alert in alerts {
        let package_name = if !alert.security_vulnerability.package.name.trim().is_empty() {
            alert.security_vulnerability.package.name.trim().to_string()
        } else {
            format!("alert-{}", alert.number)
        };
        let ecosystem = if !alert
            .security_vulnerability
            .package
            .ecosystem
            .trim()
            .is_empty()
        {
            alert
                .security_vulnerability
                .package
                .ecosystem
                .trim()
                .to_string()
        } else {
            "unknown".into()
        };

        let key = triage_key(&ecosystem, &package_name, Some(alert.number));
        let builder = builders.entry(key).or_default();
        if builder.package_name.is_empty() {
            builder.package_name = package_name.clone();
        }
        if builder.ecosystem.is_empty() {
            builder.ecosystem = ecosystem.clone();
        }
        builder.highest_severity =
            strongest_severity(&builder.highest_severity, alert_severity(&alert)).into();
        builder
            .runtime_labels
            .insert(runtime_for_ecosystem(&ecosystem).into());
        builder.reasons.insert(format!(
            "Open {} severity alert is attached to this dependency.",
            alert_severity(&alert)
        ));
        push_evidence(
            &mut builder.evidence,
            format!(
                "Dependabot alert #{} · {}",
                alert.number,
                alert.security_advisory.summary.trim()
            ),
        );
        builder.alerts.push(DependencyAlertRef {
            number: alert.number,
            package_name,
            ecosystem,
            severity: alert_severity(&alert).into(),
            summary: alert.security_advisory.summary.trim().to_string(),
            html_url: alert.html_url,
            created_at: alert.created_at,
            vulnerable_version_range: alert.security_vulnerability.vulnerable_version_range,
            first_patched_version: alert
                .security_vulnerability
                .first_patched_version
                .map(|value| value.identifier)
                .unwrap_or_default(),
        });
    }

    let mut items = builders
        .into_values()
        .map(finalize_item)
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        recommendation_priority(&right.recommendation)
            .cmp(&recommendation_priority(&left.recommendation))
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| right.stale_days.cmp(&left.stale_days))
            .then_with(|| left.package_name.cmp(&right.package_name))
    });

    let metrics = build_metrics(pulls.len() as u32, items.as_slice());
    let summary = build_summary(repo, &metrics, items.first());

    Ok(TriageScanResult {
        id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        repo: repo.to_string(),
        summary,
        metrics,
        items,
        warnings,
    })
}

fn build_metrics(scanned_pull_requests: u32, items: &[DependencyTriageItem]) -> TriageMetrics {
    let mut metrics = TriageMetrics {
        scanned_pull_requests,
        dependency_pull_requests: items
            .iter()
            .map(|item| item.pull_requests.len() as u32)
            .sum(),
        open_alerts: items.iter().map(|item| item.alerts.len() as u32).sum(),
        tracked_items: items.len() as u32,
        ..TriageMetrics::default()
    };

    for item in items {
        match item.recommendation.as_str() {
            "update_now" => metrics.update_now += 1,
            "watch" => metrics.watch += 1,
            _ => metrics.ignore_for_now += 1,
        }
        if matches!(item.runtime_impact.as_str(), "runtime" | "mixed") {
            metrics.runtime_updates += 1;
        }
        if item.update_kind == "major" {
            metrics.major_updates += 1;
        }
    }

    metrics
}

fn build_summary(
    repo: &str,
    metrics: &TriageMetrics,
    top: Option<&DependencyTriageItem>,
) -> String {
    if metrics.tracked_items == 0 {
        return format!(
            "DepTriage did not find open dependency PRs or security alerts that need ranking in `{repo}` right now."
        );
    }

    let mut summary = format!(
        "DepTriage ranked {} dependency item{} for `{repo}`: {} update now, {} watch, {} ignore for now.",
        metrics.tracked_items,
        if metrics.tracked_items == 1 { "" } else { "s" },
        metrics.update_now,
        metrics.watch,
        metrics.ignore_for_now,
    );

    if let Some(top) = top {
        summary.push_str(&format!(
            " Highest urgency: {} ({}, {}).",
            top.package_name,
            top.recommendation.replace('_', " "),
            top.summary
        ));
    }

    summary
}

fn finalize_item(builder: Builder) -> DependencyTriageItem {
    let recommendation = recommend_item(&builder);
    let score = score_item(&builder);
    let runtime_impact = finalize_runtime_impact(&builder.runtime_labels);
    let update_kind = if builder.strongest_update_kind.is_empty() {
        if !builder.alerts.is_empty() {
            "security".into()
        } else {
            "unknown".into()
        }
    } else {
        builder.strongest_update_kind.clone()
    };

    let source = match (builder.pull_requests.is_empty(), builder.alerts.is_empty()) {
        (false, false) => "pull request + alert",
        (false, true) => "pull request",
        (true, false) => "alert",
        (true, true) => "unknown",
    }
    .to_string();

    let mut reasons = builder.reasons.into_iter().collect::<Vec<_>>();
    if !builder.highest_severity.is_empty() {
        reasons.push(format!(
            "Security severity for this dependency is {}.",
            builder.highest_severity
        ));
    }
    if builder.pull_requests.len() > 1 || builder.alerts.len() > 1 {
        reasons.push(
            "Multiple PRs or alerts are pointing at the same dependency, so the pressure is starting to compound.".into(),
        );
    }
    if builder.stale_days >= 14 {
        reasons.push(format!(
            "This update has been sitting open for about {} day{}.",
            builder.stale_days,
            if builder.stale_days == 1 { "" } else { "s" }
        ));
    }

    let summary = build_item_summary(
        &builder.package_name,
        &recommendation,
        &update_kind,
        &runtime_impact,
        &builder.highest_severity,
        builder.pull_requests.len() as u32,
        builder.alerts.len() as u32,
    );

    DependencyTriageItem {
        key: triage_key(&builder.ecosystem, &builder.package_name, None),
        package_name: builder.package_name,
        ecosystem: if builder.ecosystem.is_empty() {
            "unknown".into()
        } else {
            builder.ecosystem
        },
        recommendation,
        score,
        update_kind,
        runtime_impact,
        source,
        summary,
        reasons,
        manifests: builder.manifests.into_iter().collect(),
        changed_paths: builder.changed_paths.into_iter().collect(),
        stale_days: builder.stale_days,
        pull_requests: builder.pull_requests,
        alerts: builder.alerts,
        evidence: builder.evidence,
    }
}

fn build_item_summary(
    package_name: &str,
    recommendation: &str,
    update_kind: &str,
    runtime_impact: &str,
    severity: &str,
    prs: u32,
    alerts: u32,
) -> String {
    let mut parts = Vec::new();
    if !severity.is_empty() {
        parts.push(format!("{severity} severity alert"));
    }
    if update_kind != "unknown" && update_kind != "security" {
        parts.push(format!("{update_kind} version jump"));
    }
    if !runtime_impact.is_empty() && runtime_impact != "unknown" {
        parts.push(format!("{runtime_impact} impact"));
    }
    if prs > 1 {
        parts.push(format!("{prs} overlapping PRs"));
    }
    if alerts > 1 {
        parts.push(format!("{alerts} overlapping alerts"));
    }

    let driver = if parts.is_empty() {
        "notable dependency movement".into()
    } else {
        parts.join(", ")
    };

    format!(
        "{} is currently marked `{}` because DepTriage saw {}.",
        package_name,
        recommendation.replace('_', " "),
        driver
    )
}

fn score_item(builder: &Builder) -> u32 {
    let mut score = 0;
    score += match builder.highest_severity.as_str() {
        "critical" => 70,
        "high" => 56,
        "moderate" => 34,
        "medium" => 34,
        "low" => 18,
        _ => 0,
    };
    score += match builder.strongest_update_kind.as_str() {
        "major" => 24,
        "minor" => 12,
        "patch" => 5,
        _ => 8,
    };

    let runtime = finalize_runtime_impact(&builder.runtime_labels);
    score += match runtime.as_str() {
        "runtime" => 14,
        "mixed" => 10,
        "ci" => 4,
        "tooling" => 2,
        _ => 0,
    };

    score += match builder.stale_days {
        days if days >= 30 => 12,
        days if days >= 14 => 7,
        days if days >= 7 => 3,
        _ => 0,
    };

    if builder.pull_requests.len() > 1 {
        score += 8;
    }
    if builder.alerts.len() > 1 {
        score += 6;
    }
    if builder.manifests.len() > 3 {
        score += 4;
    }

    score.min(100)
}

fn recommend_item(builder: &Builder) -> String {
    let score = score_item(builder);
    let runtime = finalize_runtime_impact(&builder.runtime_labels);
    let severity = builder.highest_severity.as_str();
    let update_kind = builder.strongest_update_kind.as_str();

    if matches!(severity, "critical" | "high") {
        return "update_now".into();
    }
    if score >= 55 {
        return "update_now".into();
    }
    if score >= 36 || (update_kind == "major" && matches!(runtime.as_str(), "runtime" | "mixed")) {
        return "watch".into();
    }
    "ignore_for_now".into()
}

fn analyze_pull(
    pr: &github::GitHubPullRequest,
    files: &[github::GitHubPullFile],
) -> Option<PullAnalysis> {
    let manifest_paths = files
        .iter()
        .filter_map(|file| dependency_manifest(&file.filename).map(|_| file.filename.clone()))
        .collect::<Vec<_>>();
    let changed_paths = files
        .iter()
        .map(|file| file.filename.clone())
        .take(12)
        .collect::<Vec<_>>();
    let package_name = infer_package_name(pr.title.as_str(), pr.body.as_deref(), files)?;
    let ecosystem = infer_ecosystem(
        &manifest_paths,
        pr.title.as_str(),
        pr.body.as_deref(),
        &package_name,
    );
    let update_kind = infer_update_kind(pr.title.as_str(), pr.body.as_deref());
    let runtime_impact = infer_runtime_impact(
        &manifest_paths,
        pr.title.as_str(),
        pr.body.as_deref(),
        &package_name,
    );
    let source_tool = infer_source_tool(pr.title.as_str(), pr.body.as_deref());
    let (from_version, to_version) = parse_versions(pr.title.as_str(), pr.body.as_deref());
    let stale_days = stale_days(&pr.updated_at);
    let summary_reason = if source_tool == "dependabot" || source_tool == "renovate" {
        format!(
            "This looks like a bot-managed {} update touching {} manifest{}.",
            update_kind,
            manifest_paths.len(),
            if manifest_paths.len() == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "This {} dependency update touches {} manifest{} and still needs a triage call.",
            update_kind,
            manifest_paths.len(),
            if manifest_paths.len() == 1 { "" } else { "s" }
        )
    };

    Some(PullAnalysis {
        package_name,
        ecosystem,
        update_kind,
        runtime_impact,
        stale_days,
        source_tool,
        manifest_paths,
        changed_paths,
        from_version,
        to_version,
        summary_reason,
    })
}

fn looks_like_dependency_pr(
    pr: &github::GitHubPullRequest,
    files: &[github::GitHubPullFile],
) -> bool {
    let title = pr.title.to_ascii_lowercase();
    let body = pr.body.clone().unwrap_or_default().to_ascii_lowercase();
    let keyword_hit = [
        "dependabot",
        "renovate",
        "dependency",
        "dependencies",
        "chore(deps",
        "bump ",
        "update ",
        "upgrade ",
    ]
    .iter()
    .any(|needle| title.contains(needle) || body.contains(needle));

    let dependency_files = files
        .iter()
        .filter(|file| dependency_manifest(&file.filename).is_some())
        .count();
    let non_dependency_files = files
        .iter()
        .filter(|file| !is_ignorable_dependency_change(&file.filename))
        .count()
        .saturating_sub(dependency_files);

    keyword_hit || (dependency_files > 0 && non_dependency_files == 0)
}

fn dependency_manifest(path: &str) -> Option<(&'static str, &'static str)> {
    let normalized = path.replace('\\', "/");
    let name = normalized.rsplit('/').next().unwrap_or(normalized.as_str());

    if normalized.starts_with(".github/workflows/") {
        return Some(("github-actions", "ci"));
    }

    match name {
        "package.json" | "package-lock.json" | "yarn.lock" | "pnpm-lock.yaml" | "bun.lock"
        | "bun.lockb" => Some(("npm", "runtime")),
        "Cargo.toml" | "Cargo.lock" => Some(("cargo", "runtime")),
        "requirements.txt"
        | "requirements-dev.txt"
        | "requirements-test.txt"
        | "pyproject.toml"
        | "poetry.lock"
        | "Pipfile"
        | "Pipfile.lock"
        | "uv.lock"
        | "setup.py"
        | "setup.cfg" => Some(("python", "runtime")),
        "Gemfile" | "Gemfile.lock" => Some(("ruby", "runtime")),
        "go.mod" | "go.sum" => Some(("gomod", "runtime")),
        "pom.xml" => Some(("maven", "runtime")),
        "build.gradle" | "build.gradle.kts" | "gradle.properties" | "gradle.lockfile" => {
            Some(("gradle", "runtime"))
        }
        "composer.json" | "composer.lock" => Some(("composer", "runtime")),
        "mix.exs" | "mix.lock" => Some(("hex", "runtime")),
        "Directory.Packages.props" | "packages.config" => Some(("nuget", "runtime")),
        _ => None,
    }
}

fn is_ignorable_dependency_change(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    dependency_manifest(&normalized).is_some()
        || normalized.ends_with("README.md")
        || normalized.ends_with("CHANGELOG.md")
        || normalized.starts_with("docs/")
}

fn infer_package_name(
    title: &str,
    body: Option<&str>,
    files: &[github::GitHubPullFile],
) -> Option<String> {
    parse_package_name(title)
        .or_else(|| body.and_then(parse_package_name))
        .or_else(|| {
            files.iter().find_map(|file| {
                let name = file.filename.rsplit('/').next().unwrap_or("");
                if name == "package.json" || name == "Cargo.toml" || name == "pyproject.toml" {
                    Some(name.to_string())
                } else {
                    None
                }
            })
        })
}

fn parse_package_name(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    for marker in ["bump ", "update ", "upgrade "] {
        if let Some(index) = lower.find(marker) {
            let remainder = &text[index + marker.len()..];
            let trimmed = remainder
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_start_matches("dependency ")
                .trim_start_matches("dependencies ")
                .trim_start_matches("deps ")
                .trim_start_matches("the ")
                .trim_start();
            let token = trimmed
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c: char| {
                    matches!(c, '`' | '"' | '\'' | ':' | ',' | ';' | '(' | ')')
                });
            if !token.is_empty() && !matches!(token, "from" | "to") {
                return Some(token.to_string());
            }
        }
    }
    None
}

fn parse_versions(title: &str, body: Option<&str>) -> (String, String) {
    parse_versions_from_text(title)
        .or_else(|| body.and_then(parse_versions_from_text))
        .unwrap_or_default()
}

fn parse_versions_from_text(text: &str) -> Option<(String, String)> {
    let lower = text.to_ascii_lowercase();
    let from_index = lower.find(" from ")?;
    let to_index = lower[from_index + 6..].find(" to ")? + from_index + 6;
    let from = clean_version_token(text[from_index + 6..to_index].trim());
    let to = clean_version_token(text[to_index + 4..].split_whitespace().next().unwrap_or(""));
    if from.is_empty() || to.is_empty() {
        None
    } else {
        Some((from, to))
    }
}

fn clean_version_token(raw: &str) -> String {
    raw.trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | ',' | '.' | ')' | '(' | ';'))
        .to_string()
}

fn infer_ecosystem(
    manifests: &[String],
    title: &str,
    body: Option<&str>,
    package_name: &str,
) -> String {
    for manifest in manifests {
        if let Some((ecosystem, _)) = dependency_manifest(manifest) {
            return ecosystem.to_string();
        }
    }

    let haystack = format!(
        "{} {} {}",
        title.to_ascii_lowercase(),
        body.unwrap_or_default().to_ascii_lowercase(),
        package_name.to_ascii_lowercase()
    );
    if haystack.contains("cargo") || haystack.contains("crates.io") {
        "cargo".into()
    } else if haystack.contains("pip") || haystack.contains("python") || haystack.contains("poetry")
    {
        "python".into()
    } else if haystack.contains("npm") || haystack.contains("pnpm") || haystack.contains("yarn") {
        "npm".into()
    } else if haystack.contains("github action") || haystack.contains("actions/") {
        "github-actions".into()
    } else {
        "unknown".into()
    }
}

fn infer_update_kind(title: &str, body: Option<&str>) -> String {
    let (from_version, to_version) = parse_versions(title, body);
    if from_version.is_empty() || to_version.is_empty() {
        return "unknown".into();
    }

    match compare_versions(&from_version, &to_version) {
        Some(kind) => kind,
        None => "unknown".into(),
    }
}

fn compare_versions(from: &str, to: &str) -> Option<String> {
    let from_parts = version_parts(from)?;
    let to_parts = version_parts(to)?;

    if to_parts.0 != from_parts.0 {
        Some("major".into())
    } else if to_parts.1 != from_parts.1 {
        Some("minor".into())
    } else if to_parts.2 != from_parts.2 {
        Some("patch".into())
    } else {
        None
    }
}

fn version_parts(value: &str) -> Option<(u32, u32, u32)> {
    let trimmed = value.trim().trim_start_matches('v');
    let mut captured = String::new();
    for ch in trimmed.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            captured.push(ch);
        } else if !captured.is_empty() {
            break;
        }
    }
    if captured.is_empty() {
        return None;
    }

    let mut parts = captured.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    Some((major, minor, patch))
}

fn infer_source_tool(title: &str, body: Option<&str>) -> String {
    let haystack = format!(
        "{} {}",
        title.to_ascii_lowercase(),
        body.unwrap_or_default().to_ascii_lowercase()
    );
    if haystack.contains("dependabot") {
        "dependabot".into()
    } else if haystack.contains("renovate") {
        "renovate".into()
    } else {
        "manual".into()
    }
}

fn infer_runtime_impact(
    manifests: &[String],
    title: &str,
    body: Option<&str>,
    package_name: &str,
) -> String {
    let mut labels = BTreeSet::new();

    for manifest in manifests {
        if let Some((_, impact)) = dependency_manifest(manifest) {
            labels.insert(impact.to_string());
        }
    }

    let haystack = format!(
        "{} {} {}",
        title.to_ascii_lowercase(),
        body.unwrap_or_default().to_ascii_lowercase(),
        package_name.to_ascii_lowercase()
    );

    if haystack.contains("deps-dev")
        || haystack.contains("dev dependency")
        || haystack.contains("dev-dependency")
        || is_tooling_package(package_name)
    {
        labels.insert("tooling".into());
    }
    if haystack.contains("github-action") || haystack.contains("actions/") {
        labels.insert("ci".into());
    }

    finalize_runtime_impact(&labels)
}

fn finalize_runtime_impact(labels: &BTreeSet<String>) -> String {
    if labels.is_empty() {
        return "unknown".into();
    }
    if labels.len() == 1 {
        return labels
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| "unknown".into());
    }
    if labels.contains("runtime") && (labels.contains("tooling") || labels.contains("ci")) {
        "mixed".into()
    } else if labels.contains("ci") {
        "ci".into()
    } else if labels.contains("tooling") {
        "tooling".into()
    } else {
        labels
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| "unknown".into())
    }
}

fn is_tooling_package(package_name: &str) -> bool {
    let value = package_name.to_ascii_lowercase();
    [
        "@types/",
        "eslint",
        "prettier",
        "vitest",
        "jest",
        "ts-jest",
        "rollup",
        "vite",
        "webpack",
        "babel",
        "cypress",
        "playwright",
        "pytest",
        "ruff",
        "mypy",
        "black",
        "tox",
        "nox",
        "cargo-nextest",
        "clippy",
        "rustfmt",
        "actions/",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

fn stale_days(updated_at: &str) -> u32 {
    DateTime::parse_from_rfc3339(updated_at)
        .map(|value| {
            let now = Utc::now();
            let updated = value.with_timezone(&Utc);
            (now - updated).num_days().max(0) as u32
        })
        .unwrap_or(0)
}

fn triage_key(ecosystem: &str, package_name: &str, fallback: Option<u32>) -> String {
    let eco = sanitize_key_part(ecosystem);
    let pkg = sanitize_key_part(package_name);
    if !eco.is_empty() && !pkg.is_empty() {
        format!("{eco}:{pkg}")
    } else if let Some(fallback) = fallback {
        format!("unknown:{fallback}")
    } else {
        "unknown".into()
    }
}

fn sanitize_key_part(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn strongest_update_kind<'a>(left: &'a str, right: &'a str) -> &'a str {
    if update_rank(right) > update_rank(left) {
        right
    } else {
        left
    }
}

fn update_rank(value: &str) -> u8 {
    match value {
        "major" => 4,
        "minor" => 3,
        "patch" => 2,
        "security" => 1,
        _ => 0,
    }
}

fn strongest_severity<'a>(left: &'a str, right: &'a str) -> &'a str {
    if severity_rank(right) > severity_rank(left) {
        right
    } else {
        left
    }
}

fn severity_rank(value: &str) -> u8 {
    match value {
        "critical" => 5,
        "high" => 4,
        "moderate" | "medium" => 3,
        "low" => 2,
        "" => 0,
        _ => 1,
    }
}

fn alert_severity(alert: &github::GitHubDependabotAlert) -> &str {
    let vuln = alert.security_vulnerability.severity.trim();
    if !vuln.is_empty() {
        vuln
    } else {
        alert.security_advisory.severity.trim()
    }
}

fn runtime_for_ecosystem(ecosystem: &str) -> &str {
    if ecosystem == "github-actions" {
        "ci"
    } else {
        "runtime"
    }
}

fn recommendation_priority(value: &str) -> u8 {
    match value {
        "update_now" => 3,
        "watch" => 2,
        _ => 1,
    }
}

fn push_evidence(evidence: &mut Vec<String>, line: String) {
    if line.trim().is_empty() || evidence.iter().any(|item| item == &line) {
        return;
    }
    if evidence.len() < 8 {
        evidence.push(line);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compare_versions, infer_runtime_impact, looks_like_dependency_pr, parse_package_name,
    };
    use crate::github::{GitHubPullFile, GitHubPullRequest};

    #[test]
    fn compares_semver_jumps() {
        assert_eq!(compare_versions("1.2.3", "2.0.0").as_deref(), Some("major"));
        assert_eq!(compare_versions("1.2.3", "1.3.0").as_deref(), Some("minor"));
        assert_eq!(compare_versions("1.2.3", "1.2.4").as_deref(), Some("patch"));
    }

    #[test]
    fn parses_dependabot_package_name() {
        assert_eq!(
            parse_package_name("build(deps): bump reqwest from 0.11.0 to 0.12.0").as_deref(),
            Some("reqwest")
        );
        assert_eq!(
            parse_package_name("chore: update dependency react to v19").as_deref(),
            Some("react")
        );
    }

    #[test]
    fn detects_dependency_prs_from_titles_and_files() {
        let pr = GitHubPullRequest {
            title: "build(deps): bump tokio from 1.42 to 1.43".into(),
            ..GitHubPullRequest::default()
        };
        let files = vec![GitHubPullFile {
            filename: "backend/Cargo.toml".into(),
            ..GitHubPullFile::default()
        }];
        assert!(looks_like_dependency_pr(&pr, &files));

        let runtime = infer_runtime_impact(
            &[String::from("backend/Cargo.toml")],
            &pr.title,
            None,
            "tokio",
        );
        assert_eq!(runtime, "runtime");
    }
}

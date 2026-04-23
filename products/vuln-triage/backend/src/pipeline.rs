use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use patchhive_product_core::contract;
use patchhive_product_core::startup::count_errors;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{
        auth_enabled, generate_and_save_key, generate_and_save_service_token,
        service_auth_enabled, service_token_generation_allowed, verify_token,
    },
    db, github,
    models::{
        HistoryItem, OverviewPayload, ScanRequest, VulnMetrics, VulnScanResult,
        VulnerabilityFinding,
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
        "vuln-triage",
        "VulnTriage",
        vec![contract::action(
            "scan_github_findings",
            "Scan GitHub findings",
            "POST",
            "/scan/github/findings",
            "Rank code scanning and dependency alerts into a practical security queue.",
            true,
        )],
        vec![
            contract::link("overview", "Overview", "/overview"),
            contract::link("history", "History", "/history"),
        ],
    ))
}

pub async fn runs() -> Json<contract::ProductRunsResponse> {
    Json(contract::runs_from_history("vuln-triage", db::history(30)))
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
        "product": "VulnTriage by PatchHive",
        "auth_enabled": auth_enabled(),
        "config_errors": errors,
        "db_ok": db_ok,
        "db_path": db::db_path(),
        "github_ready": github::github_token_configured(),
        "scan_count": counts.scans,
        "repo_count": counts.repos,
        "tracked_finding_count": counts.tracked_findings,
        "fix_now_count": counts.fix_now,
        "plan_next_count": counts.plan_next,
        "watch_count": counts.watch,
        "mode": "security-triage",
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

pub async fn history_detail(Path(id): Path<String>) -> JsonResult<VulnScanResult> {
    db::get_scan(&id)
        .map(Json)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "VulnTriage scan not found"))
}

pub async fn scan_github_findings(
    State(state): State<AppState>,
    Json(request): Json<ScanRequest>,
) -> JsonResult<VulnScanResult> {
    let repo = request.repo.trim();
    if !valid_repo(repo) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Repository must be in owner/name format.",
        ));
    }

    let result = build_scan_result(
        &state,
        repo,
        request.include_code_scanning,
        request.include_dependency_alerts,
    )
    .await;

    let result = match result {
        Ok(result) => result,
        Err(err) => return Err(api_error(StatusCode::BAD_GATEWAY, err)),
    };

    db::save_scan(&result)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(result))
}

fn api_error(status: StatusCode, error: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": error.into() })))
}

async fn build_scan_result(
    state: &AppState,
    repo: &str,
    include_code_scanning: bool,
    include_dependency_alerts: bool,
) -> Result<VulnScanResult, String> {
    let mut warnings = Vec::new();
    let mut findings = Vec::new();

    if include_code_scanning {
        match github::fetch_code_scanning_alerts(&state.http, repo, 100).await {
            Ok(alerts) => {
                findings.extend(alerts.into_iter().map(code_scanning_to_finding));
            }
            Err(err) => warnings.push(format!(
                "Code scanning alerts could not be read for {repo}: {err}"
            )),
        }
    }

    if include_dependency_alerts {
        match github::fetch_dependabot_alerts(&state.http, repo, 100).await {
            Ok(alerts) => {
                findings.extend(alerts.into_iter().map(dependabot_to_finding));
            }
            Err(err) => warnings.push(format!(
                "Dependency alerts could not be read for {repo}: {err}"
            )),
        }
    }

    findings.sort_by(|left, right| {
        recommendation_rank(&right.recommendation)
            .cmp(&recommendation_rank(&left.recommendation))
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| severity_rank(&right.severity).cmp(&severity_rank(&left.severity)))
            .then_with(|| left.title.cmp(&right.title))
    });

    let metrics = build_metrics(&findings);
    let summary = build_summary(repo, &metrics, findings.first());

    Ok(VulnScanResult {
        id: Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        repo: repo.to_string(),
        summary,
        metrics,
        findings,
        warnings,
    })
}

fn build_metrics(findings: &[VulnerabilityFinding]) -> VulnMetrics {
    let mut metrics = VulnMetrics {
        code_scanning_alerts: findings
            .iter()
            .filter(|item| item.source == "code_scanning")
            .count() as u32,
        dependency_alerts: findings
            .iter()
            .filter(|item| item.source == "dependency_alert")
            .count() as u32,
        tracked_findings: findings.len() as u32,
        ..VulnMetrics::default()
    };

    for finding in findings {
        match finding.recommendation.as_str() {
            "fix_now" => metrics.fix_now += 1,
            "plan_next" => metrics.plan_next += 1,
            _ => metrics.watch += 1,
        }
        if matches!(
            finding.reachability.as_str(),
            "public surface" | "runtime path" | "runtime dependency"
        ) {
            metrics.runtime_exposed += 1;
        }
        if finding.owner_hint != "repo maintainers" {
            metrics.owner_scoped += 1;
        }
    }

    metrics
}

fn build_summary(repo: &str, metrics: &VulnMetrics, top: Option<&VulnerabilityFinding>) -> String {
    if metrics.tracked_findings == 0 {
        return format!(
            "VulnTriage did not find open security alerts worth ranking in `{repo}` right now."
        );
    }

    let mut summary = format!(
        "VulnTriage ranked {} finding{} for `{repo}`: {} fix now, {} plan next, {} watch.",
        metrics.tracked_findings,
        if metrics.tracked_findings == 1 {
            ""
        } else {
            "s"
        },
        metrics.fix_now,
        metrics.plan_next,
        metrics.watch,
    );
    if let Some(top) = top {
        summary.push_str(&format!(" Highest urgency: {}.", top.title));
    }
    summary
}

fn code_scanning_to_finding(alert: github::GitHubCodeScanningAlert) -> VulnerabilityFinding {
    let location = location_label(
        &alert.most_recent_instance.location.path,
        alert.most_recent_instance.location.start_line,
    );
    let severity = code_scanning_severity(&alert);
    let reachability = code_scanning_reachability(
        &alert.most_recent_instance.location.path,
        &alert.most_recent_instance.classifications,
    );
    let owner_hint = owner_hint_for_path(&alert.most_recent_instance.location.path);
    let score = score_code_scanning(
        &severity,
        &reachability,
        &alert.most_recent_instance.classifications,
    );
    let recommendation = recommend(score, &severity);
    let title = if !alert.rule.name.trim().is_empty() {
        alert.rule.name.clone()
    } else if !alert.rule.id.trim().is_empty() {
        alert.rule.id.clone()
    } else {
        format!("Code scanning alert #{}", alert.number)
    };

    let mut identifiers = vec![format!("alert:{}", alert.number)];
    if !alert.rule.id.trim().is_empty() {
        identifiers.push(alert.rule.id.clone());
    }
    identifiers.extend(
        alert
            .rule
            .tags
            .iter()
            .filter(|tag| tag.contains("cwe"))
            .cloned(),
    );

    let mut evidence = Vec::new();
    push_evidence(
        &mut evidence,
        format!("{} at {}", alert.tool.name, location),
    );
    push_evidence(
        &mut evidence,
        alert.most_recent_instance.message.text.clone(),
    );
    if !alert.most_recent_instance.ref_.trim().is_empty() {
        push_evidence(
            &mut evidence,
            format!("Observed on {}", alert.most_recent_instance.ref_),
        );
    }

    VulnerabilityFinding {
        key: format!("code-scanning:{}", alert.number),
        source: "code_scanning".into(),
        recommendation,
        severity: severity.clone(),
        score,
        title,
        summary: if !alert.rule.description.trim().is_empty() {
            alert.rule.description
        } else if !alert.most_recent_instance.message.text.trim().is_empty() {
            alert.most_recent_instance.message.text.clone()
        } else {
            "GitHub code scanning flagged a live security finding.".into()
        },
        owner_hint: owner_hint.into(),
        location,
        package_name: String::new(),
        ecosystem: String::new(),
        reachability: reachability.into(),
        next_action: format!(
            "Inspect {} with the {} owners, validate exploitability, and decide whether to patch immediately or add a bounded mitigation.",
            alert.most_recent_instance.location.path.trim(),
            owner_hint
        ),
        tool_name: alert.tool.name,
        html_url: alert.html_url,
        created_at: if !alert.updated_at.trim().is_empty() {
            alert.updated_at
        } else {
            alert.created_at
        },
        identifiers,
        evidence,
        references: Vec::new(),
    }
}

fn dependabot_to_finding(alert: github::GitHubDependabotAlert) -> VulnerabilityFinding {
    let package_name = if !alert.dependency.package.name.trim().is_empty() {
        alert.dependency.package.name.clone()
    } else {
        alert.security_vulnerability.package.name.clone()
    };
    let ecosystem = if !alert.dependency.package.ecosystem.trim().is_empty() {
        alert.dependency.package.ecosystem.clone()
    } else {
        alert.security_vulnerability.package.ecosystem.clone()
    };
    let severity = dependabot_severity(&alert).to_string();
    let reachability =
        dependency_reachability(&alert.dependency.manifest_path, &alert.dependency.scope)
            .to_string();
    let owner_hint = owner_hint_for_path(&alert.dependency.manifest_path);
    let score = score_dependency_alert(&alert, &severity, &reachability);
    let recommendation = recommend(score, &severity);
    let summary = alert.security_advisory.summary.clone();
    let title = format!(
        "{} vulnerability in {}",
        severity.to_ascii_uppercase(),
        package_name
    );

    let mut identifiers = Vec::new();
    if !alert.security_advisory.ghsa_id.trim().is_empty() {
        identifiers.push(alert.security_advisory.ghsa_id.clone());
    }
    if !alert.security_advisory.cve_id.trim().is_empty() {
        identifiers.push(alert.security_advisory.cve_id.clone());
    }
    identifiers.extend(
        alert
            .security_advisory
            .cwes
            .iter()
            .map(|cwe| cwe.cwe_id.clone())
            .filter(|value| !value.trim().is_empty()),
    );

    let mut evidence = Vec::new();
    push_evidence(
        &mut evidence,
        format!(
            "{} scope in {}",
            if alert.dependency.scope.trim().is_empty() {
                "unknown".into()
            } else {
                alert.dependency.scope.clone()
            },
            alert.dependency.manifest_path
        ),
    );
    if !alert
        .security_vulnerability
        .vulnerable_version_range
        .trim()
        .is_empty()
    {
        push_evidence(
            &mut evidence,
            format!(
                "vulnerable range {}",
                alert.security_vulnerability.vulnerable_version_range
            ),
        );
    }
    if let Some(first_patched) = alert.security_vulnerability.first_patched_version.as_ref() {
        if !first_patched.identifier.trim().is_empty() {
            push_evidence(
                &mut evidence,
                format!("first patched version {}", first_patched.identifier),
            );
        }
    }
    if let Some(epss) = alert.security_advisory.epss.as_ref() {
        if epss.percentage > 0.0 {
            push_evidence(&mut evidence, format!("epss {:.3}", epss.percentage));
        }
    }

    VulnerabilityFinding {
        key: format!("dependency-alert:{}", alert.number),
        source: "dependency_alert".into(),
        recommendation,
        severity,
        score,
        title,
        summary,
        owner_hint: owner_hint.into(),
        location: alert.dependency.manifest_path.clone(),
        package_name,
        ecosystem,
        reachability,
        next_action: dependency_next_action(&alert, owner_hint),
        tool_name: "Dependabot".into(),
        html_url: alert.html_url,
        created_at: if !alert.updated_at.trim().is_empty() {
            alert.updated_at
        } else {
            alert.created_at
        },
        identifiers,
        evidence,
        references: alert
            .security_advisory
            .references
            .into_iter()
            .map(|item| item.url)
            .filter(|value| !value.trim().is_empty())
            .take(6)
            .collect(),
    }
}

fn dependency_next_action(alert: &github::GitHubDependabotAlert, owner_hint: &str) -> String {
    let target = alert
        .security_vulnerability
        .first_patched_version
        .as_ref()
        .map(|item| item.identifier.clone())
        .filter(|value| !value.trim().is_empty());

    if let Some(version) = target {
        format!(
            "Ask the {} to move {} in {} to at least {}, then validate the upgrade path and rollout risk.",
            owner_hint,
            alert.dependency.package.name,
            alert.dependency.manifest_path,
            version
        )
    } else {
        format!(
            "Ask the {} to triage {} in {} and decide whether to patch, mitigate, or temporarily accept the risk.",
            owner_hint,
            alert.dependency.package.name,
            alert.dependency.manifest_path
        )
    }
}

fn code_scanning_severity(alert: &github::GitHubCodeScanningAlert) -> String {
    if !alert.rule.security_severity_level.trim().is_empty() {
        alert
            .rule
            .security_severity_level
            .trim()
            .to_ascii_lowercase()
    } else if !alert.rule.severity.trim().is_empty() {
        normalize_severity(alert.rule.severity.trim())
    } else {
        "medium".into()
    }
}

fn dependabot_severity(alert: &github::GitHubDependabotAlert) -> &str {
    let vuln = alert.security_vulnerability.severity.trim();
    if !vuln.is_empty() {
        vuln
    } else if !alert.security_advisory.severity.trim().is_empty() {
        alert.security_advisory.severity.trim()
    } else {
        "medium"
    }
}

fn normalize_severity(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "error" => "high".into(),
        "warning" => "medium".into(),
        "note" => "low".into(),
        other => other.into(),
    }
}

fn code_scanning_reachability(path: &str, classifications: &[String]) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if classifications
        .iter()
        .any(|value| value.to_ascii_lowercase().contains("test"))
        || lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains("/spec/")
    {
        return "test-only";
    }
    if lower.starts_with(".github/workflows/") || lower.contains("/ci/") {
        return "ci-only";
    }
    if lower.contains("/routes/")
        || lower.contains("/controller")
        || lower.contains("/api/")
        || lower.contains("/server/")
        || lower.contains("/handlers/")
    {
        return "public surface";
    }
    if lower.contains("/src/")
        || lower.contains("/app/")
        || lower.contains("/lib/")
        || lower.contains("/backend/")
    {
        return "runtime path";
    }
    "unknown"
}

fn dependency_reachability(manifest_path: &str, scope: &str) -> &'static str {
    let scope = scope.trim().to_ascii_lowercase();
    let lower = manifest_path.to_ascii_lowercase();
    if scope == "runtime" {
        return "runtime dependency";
    }
    if lower.starts_with(".github/") || lower.contains("actions") {
        return "ci-only";
    }
    if scope == "development" || lower.contains("requirements-dev") || lower.contains("test") {
        return "tooling";
    }
    "unknown"
}

fn score_code_scanning(severity: &str, reachability: &str, classifications: &[String]) -> u32 {
    let mut score = severity_score(severity);
    score += match reachability {
        "public surface" => 20,
        "runtime path" => 12,
        "ci-only" => 4,
        "test-only" => 1,
        _ => 6,
    };
    if classifications
        .iter()
        .any(|value| value.to_ascii_lowercase().contains("test"))
    {
        score = score.saturating_sub(18);
    }
    score.min(100)
}

fn score_dependency_alert(
    alert: &github::GitHubDependabotAlert,
    severity: &str,
    reachability: &str,
) -> u32 {
    let mut score = severity_score(severity);
    score += match reachability {
        "runtime dependency" => 18,
        "tooling" => 4,
        "ci-only" => 3,
        _ => 8,
    };
    if let Some(epss) = alert.security_advisory.epss.as_ref() {
        score += if epss.percentage >= 0.5 {
            18
        } else if epss.percentage >= 0.1 {
            10
        } else if epss.percentage >= 0.01 {
            5
        } else {
            0
        };
    }
    if !alert
        .security_vulnerability
        .first_patched_version
        .as_ref()
        .map(|item| item.identifier.trim().is_empty())
        .unwrap_or(true)
    {
        score += 4;
    }
    score.min(100)
}

fn severity_score(severity: &str) -> u32 {
    match severity.to_ascii_lowercase().as_str() {
        "critical" => 72,
        "high" => 56,
        "medium" | "moderate" => 38,
        "low" => 20,
        "warning" => 18,
        "note" => 8,
        _ => 28,
    }
}

fn recommend(score: u32, severity: &str) -> String {
    if matches!(severity, "critical" | "high") || score >= 70 {
        "fix_now".into()
    } else if score >= 42 {
        "plan_next".into()
    } else {
        "watch".into()
    }
}

fn recommendation_rank(value: &str) -> u8 {
    match value {
        "fix_now" => 3,
        "plan_next" => 2,
        _ => 1,
    }
}

fn severity_rank(value: &str) -> u8 {
    match value {
        "critical" => 5,
        "high" => 4,
        "medium" | "moderate" => 3,
        "low" => 2,
        "warning" => 2,
        "note" => 1,
        _ => 1,
    }
}

fn owner_hint_for_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.starts_with(".github/")
        || lower.starts_with("ci/")
        || lower.contains("/workflows/")
        || lower.contains("/ci/")
    {
        "platform / CI owners"
    } else if lower.starts_with("frontend/")
        || lower.starts_with("web/")
        || lower.starts_with("ui/")
        || lower.contains("/frontend/")
        || lower.contains("/web/")
        || lower.contains("/ui/")
    {
        "frontend owners"
    } else if lower.starts_with("mobile/")
        || lower.starts_with("android/")
        || lower.starts_with("ios/")
        || lower.contains("/mobile/")
        || lower.contains("/android/")
        || lower.contains("/ios/")
    {
        "mobile owners"
    } else if lower.starts_with("infra/")
        || lower.starts_with("terraform/")
        || lower.starts_with("helm/")
        || lower.starts_with("k8s/")
        || lower.contains("/infra/")
        || lower.contains("/terraform/")
        || lower.contains("/helm/")
        || lower.contains("/k8s/")
    {
        "infrastructure owners"
    } else if lower.starts_with("backend/")
        || lower.starts_with("server/")
        || lower.starts_with("api/")
        || lower.starts_with("routes/")
        || lower.contains("/backend/")
        || lower.contains("/server/")
        || lower.contains("/api/")
        || lower.contains("/routes/")
    {
        "backend owners"
    } else if lower.starts_with("auth/")
        || lower.starts_with("security/")
        || lower.contains("/auth/")
        || lower.contains("/security/")
    {
        "auth / security owners"
    } else if lower.starts_with("test/")
        || lower.starts_with("tests/")
        || lower.starts_with("spec/")
        || lower.contains("/test/")
        || lower.contains("/tests/")
        || lower.contains("/spec/")
    {
        "quality owners"
    } else {
        "repo maintainers"
    }
}

fn location_label(path: &str, start_line: u32) -> String {
    if path.trim().is_empty() {
        return "unknown location".into();
    }
    if start_line > 0 {
        format!("{path}:{start_line}")
    } else {
        path.into()
    }
}

fn push_evidence(items: &mut Vec<String>, value: String) {
    if value.trim().is_empty() || items.iter().any(|item| item == &value) {
        return;
    }
    if items.len() < 8 {
        items.push(value);
    }
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

#[cfg(test)]
mod tests {
    use super::{code_scanning_reachability, owner_hint_for_path, recommend, score_code_scanning};

    #[test]
    fn prefers_public_surface_code_paths() {
        assert_eq!(
            code_scanning_reachability("backend/src/api/auth/routes.rs", &[]),
            "public surface"
        );
        assert_eq!(
            code_scanning_reachability("backend/tests/auth_spec.rs", &["test".into()]),
            "test-only"
        );
    }

    #[test]
    fn owner_hints_follow_paths() {
        assert_eq!(
            owner_hint_for_path("frontend/src/app.jsx"),
            "frontend owners"
        );
        assert_eq!(
            owner_hint_for_path(".github/workflows/ci.yml"),
            "platform / CI owners"
        );
    }

    #[test]
    fn high_scoring_findings_get_fix_now() {
        let score = score_code_scanning("high", "public surface", &[]);
        assert_eq!(recommend(score, "high"), "fix_now");
        assert_eq!(recommend(30, "low"), "watch");
    }
}

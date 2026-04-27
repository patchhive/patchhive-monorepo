// scoring.rs - Scoring, severity, reachability, and ranking functions for VulnTriage

pub fn code_scanning_severity(alert: &crate::github::GitHubCodeScanningAlert) -> String {
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

pub fn dependabot_severity(alert: &crate::github::GitHubDependabotAlert) -> &str {
    let vuln = alert.security_vulnerability.severity.trim();
    if !vuln.is_empty() {
        vuln
    } else if !alert.security_advisory.severity.trim().is_empty() {
        alert.security_advisory.severity.trim()
    } else {
        "medium"
    }
}

pub fn normalize_severity(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "error" => "high".into(),
        "warning" => "medium".into(),
        "note" => "low".into(),
        other => other.into(),
    }
}

pub fn code_scanning_reachability(path: &str, classifications: &[String]) -> &'static str {
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

pub fn dependency_reachability(manifest_path: &str, scope: &str) -> &'static str {
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

pub fn score_code_scanning(severity: &str, reachability: &str, classifications: &[String]) -> u32 {
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

pub fn score_dependency_alert(
    alert: &crate::github::GitHubDependabotAlert,
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

pub fn severity_score(severity: &str) -> u32 {
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

pub fn recommend(score: u32, severity: &str) -> String {
    if matches!(severity, "critical" | "high") || score >= 70 {
        "fix_now".into()
    } else if score >= 42 {
        "plan_next".into()
    } else {
        "watch".into()
    }
}

pub fn recommendation_rank(value: &str) -> u8 {
    match value {
        "fix_now" => 3,
        "plan_next" => 2,
        _ => 1,
    }
}

pub fn severity_rank(value: &str) -> u8 {
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

pub fn owner_hint_for_path(path: &str) -> &'static str {
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

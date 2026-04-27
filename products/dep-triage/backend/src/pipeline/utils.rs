use axum::{
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde_json::json;

use crate::github;

pub type ApiError = (StatusCode, Json<serde_json::Value>);

pub fn api_error(status: StatusCode, error: impl Into<String>) -> ApiError {
    (status, Json(json!({ "error": error.into() })))
}

pub fn upstream_error(error: anyhow::Error) -> ApiError {
    api_error(StatusCode::BAD_GATEWAY, error.to_string())
}

pub fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}

pub fn triage_key(ecosystem: &str, package_name: &str, fallback: Option<u32>) -> String {
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

pub fn sanitize_key_part(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn stale_days(updated_at: &str) -> u32 {
    DateTime::parse_from_rfc3339(updated_at)
        .map(|value| {
            let now = Utc::now();
            let updated = value.with_timezone(&Utc);
            (now - updated).num_days().max(0) as u32
        })
        .unwrap_or(0)
}

pub fn push_evidence(evidence: &mut Vec<String>, line: String) {
    if line.trim().is_empty() || evidence.iter().any(|item| item == &line) {
        return;
    }
    if evidence.len() < 8 {
        evidence.push(line);
    }
}

pub fn strongest_update_kind<'a>(left: &'a str, right: &'a str) -> &'a str {
    if update_rank(right) > update_rank(left) {
        right
    } else {
        left
    }
}

pub fn update_rank(value: &str) -> u8 {
    match value {
        "major" => 4,
        "minor" => 3,
        "patch" => 2,
        "security" => 1,
        _ => 0,
    }
}

pub fn strongest_severity<'a>(left: &'a str, right: &'a str) -> &'a str {
    if severity_rank(right) > severity_rank(left) {
        right
    } else {
        left
    }
}

pub fn severity_rank(value: &str) -> u8 {
    match value {
        "critical" => 5,
        "high" => 4,
        "moderate" | "medium" => 3,
        "low" => 2,
        "" => 0,
        _ => 1,
    }
}

pub fn alert_severity(alert: &github::GitHubDependabotAlert) -> &str {
    let vuln = alert.security_vulnerability.severity.trim();
    if !vuln.is_empty() {
        vuln
    } else {
        alert.security_advisory.severity.trim()
    }
}

pub fn runtime_for_ecosystem(ecosystem: &str) -> &str {
    if ecosystem == "github-actions" {
        "ci"
    } else {
        "runtime"
    }
}

pub fn recommendation_priority(value: &str) -> u8 {
    match value {
        "update_now" => 3,
        "watch" => 2,
        _ => 1,
    }
}

pub fn finalize_runtime_impact(labels: &std::collections::BTreeSet<String>) -> String {
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

pub fn compare_versions(from: &str, to: &str) -> Option<String> {
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

pub fn version_parts(value: &str) -> Option<(u32, u32, u32)> {
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

pub fn parse_versions(title: &str, body: Option<&str>) -> (String, String) {
    parse_versions_from_text(title)
        .or_else(|| body.and_then(parse_versions_from_text))
        .unwrap_or_default()
}

pub fn parse_versions_from_text(text: &str) -> Option<(String, String)> {
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

pub fn clean_version_token(raw: &str) -> String {
    raw.trim_matches(|c: char| matches!(c, '`' | '"' | '\'' | ',' | '.' | ')' | '(' | ';'))
        .to_string()
}

pub fn dependency_manifest(path: &str) -> Option<(&'static str, &'static str)> {
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

pub fn is_ignorable_dependency_change(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    dependency_manifest(&normalized).is_some()
        || normalized.ends_with("README.md")
        || normalized.ends_with("CHANGELOG.md")
        || normalized.starts_with("docs/")
}

pub fn is_tooling_package(package_name: &str) -> bool {
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

pub fn infer_ecosystem(
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

pub fn infer_update_kind(title: &str, body: Option<&str>) -> String {
    let (from_version, to_version) = parse_versions(title, body);
    if from_version.is_empty() || to_version.is_empty() {
        return "unknown".into();
    }

    match compare_versions(&from_version, &to_version) {
        Some(kind) => kind,
        None => "unknown".into(),
    }
}

pub fn infer_source_tool(title: &str, body: Option<&str>) -> String {
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

pub fn infer_runtime_impact(
    manifests: &[String],
    title: &str,
    body: Option<&str>,
    package_name: &str,
) -> String {
    use std::collections::BTreeSet;

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

pub fn parse_package_name(text: &str) -> Option<String> {
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

pub fn infer_package_name(
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

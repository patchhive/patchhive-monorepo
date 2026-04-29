// analysis.rs - Core scan logic and finding mapping for VulnTriage

use chrono::Utc;
use uuid::Uuid;

use crate::{
    github,
    models::{VulnMetrics, VulnScanResult, VulnerabilityFinding},
    state::AppState,
};

use super::{
    scoring::{
        code_scanning_reachability, code_scanning_severity, dependabot_severity,
        dependency_reachability, owner_hint_for_path, recommend, recommendation_rank,
        score_code_scanning, score_dependency_alert, severity_rank,
    },
    utils::{location_label, push_evidence},
};

pub async fn build_scan_result(
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

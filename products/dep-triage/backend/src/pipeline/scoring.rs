use std::collections::HashMap;

use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    github,
    models::{
        DependencyAlertRef, DependencyPullRef, DependencyTriageItem, TriageMetrics,
        TriageScanResult,
    },
    state::AppState,
};

use super::analysis::{analyze_pull, looks_like_dependency_pr, Builder};
use super::utils::{
    alert_severity, finalize_runtime_impact, push_evidence, recommendation_priority,
    runtime_for_ecosystem, strongest_severity, strongest_update_kind, triage_key,
};

pub async fn build_scan_result(
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

pub fn build_metrics(scanned_pull_requests: u32, items: &[DependencyTriageItem]) -> TriageMetrics {
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

pub fn build_summary(
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

pub fn finalize_item(builder: Builder) -> DependencyTriageItem {
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

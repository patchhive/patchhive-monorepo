use std::collections::BTreeSet;

use crate::github;

use super::utils::{
    dependency_manifest, infer_ecosystem, infer_package_name, infer_runtime_impact,
    infer_source_tool, infer_update_kind, is_ignorable_dependency_change, parse_versions,
    stale_days,
};

pub struct PullAnalysis {
    pub package_name: String,
    pub ecosystem: String,
    pub update_kind: String,
    pub runtime_impact: String,
    pub stale_days: u32,
    pub source_tool: String,
    pub manifest_paths: Vec<String>,
    pub changed_paths: Vec<String>,
    pub from_version: String,
    pub to_version: String,
    pub summary_reason: String,
}

#[derive(Default)]
pub struct Builder {
    pub package_name: String,
    pub ecosystem: String,
    pub strongest_update_kind: String,
    pub runtime_labels: BTreeSet<String>,
    pub manifests: BTreeSet<String>,
    pub changed_paths: BTreeSet<String>,
    pub stale_days: u32,
    pub pull_requests: Vec<crate::models::DependencyPullRef>,
    pub alerts: Vec<crate::models::DependencyAlertRef>,
    pub reasons: BTreeSet<String>,
    pub evidence: Vec<String>,
    pub highest_severity: String,
}

pub fn analyze_pull(
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
    let stale = stale_days(&pr.updated_at);
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
        stale_days: stale,
        source_tool,
        manifest_paths,
        changed_paths,
        from_version,
        to_version,
        summary_reason,
    })
}

pub fn looks_like_dependency_pr(
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

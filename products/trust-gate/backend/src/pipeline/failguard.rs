// failguard.rs — FailGuard candidate building and submission for TrustGate reviews

use patchhive_product_core::repo_memory::{submit_failguard_candidate, FailGuardCandidateRequest};
use tracing::warn;

use crate::models::ReviewResult;

use super::types::short_text;

pub async fn publish_failguard_candidate(client: &reqwest::Client, review: &ReviewResult) {
    let Some(candidate) = build_failguard_candidate_from_review(review) else {
        return;
    };

    match submit_failguard_candidate(client, &candidate).await {
        Ok(Some(_)) => {
            tracing::info!(
                "Submitted FailGuard candidate for TrustGate review {} ({})",
                review.id,
                review.recommendation
            );
        }
        Ok(None) => {}
        Err(err) => {
            warn!(
                "FailGuard candidate submission failed for TrustGate review {}: {err}",
                review.id
            );
        }
    }
}

fn build_failguard_candidate_from_review(
    review: &ReviewResult,
) -> Option<FailGuardCandidateRequest> {
    if !matches!(review.recommendation.as_str(), "warn" | "block") {
        return None;
    }

    let top_findings = review
        .findings
        .iter()
        .filter(|f| f.severity == review.recommendation)
        .chain(
            review
                .findings
                .iter()
                .filter(|f| f.severity != review.recommendation),
        )
        .take(4)
        .collect::<Vec<_>>();
    let top_label = top_findings
        .first()
        .map(|f| f.label.as_str())
        .unwrap_or("review risk");
    let source_ref = review
        .github
        .as_ref()
        .and_then(|gh| {
            if gh.pr_url.trim().is_empty() {
                None
            } else {
                Some(gh.pr_url.clone())
            }
        })
        .unwrap_or_else(|| review.id.clone());

    let mut evidence = vec![
        format!("TrustGate review {}", review.id),
        format!(
            "Recommendation: {} - risk score {}",
            review.recommendation, review.risk_score
        ),
        format!(
            "Scope: {} files, +{}, -{}",
            review.metrics.files_changed, review.metrics.additions, review.metrics.deletions
        ),
    ];
    if let Some(gh) = review.github.as_ref() {
        if !gh.pr_url.trim().is_empty() {
            evidence.push(gh.pr_url.clone());
        }
        evidence.push(format!("GitHub PR #{}: {}", gh.pr_number, gh.pr_title));
    }
    if let Some(report) = review.github_report.as_ref() {
        for url in [&report.comment_url, &report.check_url, &report.status_url] {
            if !url.trim().is_empty() {
                evidence.push(url.clone());
            }
        }
    }
    for finding in &top_findings {
        evidence.push(format!("{}: {}", finding.label, finding.detail));
        evidence.extend(finding.evidence.iter().take(2).cloned());
    }
    evidence.sort();
    evidence.dedup();

    let affected_paths = review
        .files
        .iter()
        .filter(|f| f.status == "warn" || f.status == "block")
        .map(|f| f.path.clone())
        .take(12)
        .collect::<Vec<_>>();
    let finding_labels = if top_findings.is_empty() {
        "the recorded TrustGate findings".into()
    } else {
        top_findings
            .iter()
            .map(|f| f.label.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };

    Some(FailGuardCandidateRequest {
        repo: review.repo.clone(),
        source_type: format!("trustgate-{}", review.recommendation),
        source_ref,
        title: short_text(&format!("TrustGate {}: {}", review.recommendation.to_uppercase(), top_label), 140),
        outcome: short_text(
            &format!("{} TrustGate found {} blockers and {} warnings.", review.summary, review.metrics.blocked_findings, review.metrics.warning_findings),
            320,
        ),
        lesson: short_text(
            &format!("A previous TrustGate {} showed this repo should preserve guardrails for {} before similar AI-generated diffs proceed.", review.recommendation, finding_labels),
            260,
        ),
        prevention: short_text(
            &format!("Before accepting similar diffs, resolve or explicitly override {} and verify the affected paths have the expected tests or reviewer coverage.", finding_labels),
            260,
        ),
        affected_paths,
        evidence,
        confidence: Some(if review.recommendation == "block" { 90.0 } else { 78.0 }),
    })
}

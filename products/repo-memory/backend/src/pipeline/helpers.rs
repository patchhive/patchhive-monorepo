// helpers.rs - Shared helper functions for memory entry building

use uuid::Uuid;

use crate::models::{stable_memory_ref, IngestSummary, MemoryEntry, MemoryEvidence};

pub fn build_entry(
    run_id: &str,
    repo: &str,
    kind: &str,
    title: impl Into<String>,
    detail: impl Into<String>,
    prompt_line: impl Into<String>,
    frequency: u32,
    tags: Vec<&str>,
    evidence: Vec<MemoryEvidence>,
    created_at: &str,
) -> MemoryEntry {
    let title = title.into();
    let detail = detail.into();
    let prompt_line = prompt_line.into();
    MemoryEntry {
        id: Uuid::new_v4().to_string(),
        memory_ref: stable_memory_ref(repo, kind, &title),
        run_id: run_id.to_string(),
        repo: repo.to_string(),
        kind: kind.to_string(),
        title,
        detail,
        prompt_line,
        confidence: confidence_for(frequency, evidence.len()),
        frequency,
        disposition: "signal".into(),
        pinned: false,
        tags: tags.into_iter().map(str::to_string).collect(),
        evidence,
        created_at: created_at.to_string(),
    }
}

pub fn confidence_for(frequency: u32, evidence_count: usize) -> f64 {
    let base = 42.0 + (frequency as f64 * 9.5) + (evidence_count.min(4) as f64 * 4.0);
    base.min(96.0)
}

pub fn build_summary(
    entries: &[MemoryEntry],
    merged_prs_analyzed: u32,
    review_feedback_items: u32,
    closed_issues_analyzed: u32,
) -> IngestSummary {
    let conventions = entries
        .iter()
        .filter(|entry| entry.kind == "review_rule" || entry.kind == "testing_expectation")
        .count() as u32;
    let failures = entries
        .iter()
        .filter(|entry| entry.kind == "failure_pattern")
        .count() as u32;
    let hotspots = entries
        .iter()
        .filter(|entry| entry.kind == "hotspot")
        .count() as u32;

    IngestSummary {
        merged_prs_analyzed,
        review_feedback_items,
        closed_issues_analyzed,
        memories_created: entries.len() as u32,
        conventions,
        failures,
        hotspots,
        top_memory: entries
            .first()
            .map(|entry| entry.title.clone())
            .unwrap_or_else(|| "No strong memory signals yet.".into()),
    }
}

pub fn build_prompt_pack(repo: &str, summary: &IngestSummary, entries: &[MemoryEntry]) -> String {
    let mut sections = Vec::new();
    let convention_lines: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "review_rule" || entry.kind == "testing_expectation")
        .map(|entry| format!("- {}", entry.prompt_line))
        .collect();
    let failure_lines: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "failure_pattern")
        .map(|entry| format!("- {}", entry.prompt_line))
        .collect();
    let hotspot_lines: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "hotspot")
        .map(|entry| format!("- {}", entry.prompt_line))
        .collect();
    let reviewer_lines: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "reviewer_profile")
        .map(|entry| format!("- {}", entry.prompt_line))
        .collect();
    let maintainer_lines: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == "maintainer_profile")
        .map(|entry| format!("- {}", entry.prompt_line))
        .collect();

    if !convention_lines.is_empty() {
        sections.push(format!(
            "## Conventions and review habits\n{}",
            convention_lines.join("\n")
        ));
    }
    if !failure_lines.is_empty() {
        sections.push(format!(
            "## Failure patterns to watch\n{}",
            failure_lines.join("\n")
        ));
    }
    if !hotspot_lines.is_empty() {
        sections.push(format!("## Hotspots\n{}", hotspot_lines.join("\n")));
    }
    if !reviewer_lines.is_empty() {
        sections.push(format!(
            "## Reviewer feedback signatures\n{}",
            reviewer_lines.join("\n")
        ));
    }
    if !maintainer_lines.is_empty() {
        sections.push(format!(
            "## Maintainer patterns\n{}",
            maintainer_lines.join("\n")
        ));
    }

    if sections.is_empty() {
        sections.push("## Early signal\n- RepoMemory has not seen enough repeated patterns yet. Read recent merged PRs and reviewer comments before trusting automation.".into());
    }

    format!(
        "# RepoMemory Prompt Pack\n\nRepo: **{repo}**\nGenerated from **{}** merged PRs, **{}** review feedback items, and **{}** closed issues.\n\n{}\n",
        summary.merged_prs_analyzed,
        summary.review_feedback_items,
        summary.closed_issues_analyzed,
        sections.join("\n\n")
    )
}

// memory.rs — RepoMemory context building and FailGuard candidate submission

use anyhow::Result as AnyhowResult;
use patchhive_product_core::repo_memory::{
    submit_failguard_candidate, FailGuardCandidateRequest, RepoMemoryContextResponse,
};
use serde_json::Value;

use super::types::IssueScope;

pub fn build_repo_memory_block(context: Option<&RepoMemoryContextResponse>) -> String {
    let Some(context) = context else {
        return String::new();
    };
    if context.entries.is_empty() {
        return String::new();
    }

    let mut curated = Vec::new();
    let mut signal = Vec::new();
    for entry in context.entries.iter().take(6) {
        let prefix = match (entry.pinned, entry.disposition.as_str()) {
            (true, "policy") => "[pinned policy]",
            (true, _) => "[pinned]",
            (_, "policy") => "[policy]",
            _ => "",
        };
        let line = if prefix.is_empty() {
            format!("- [{}] {}", entry.kind, entry.prompt_line)
        } else {
            format!("- {} [{}] {}", prefix, entry.kind, entry.prompt_line)
        };
        if entry.pinned || entry.disposition == "policy" {
            curated.push(line);
        } else {
            signal.push(line);
        }
    }

    let mut sections = Vec::new();
    if !curated.is_empty() {
        sections.push(format!("Operator-curated memory:\n{}", curated.join("\n")));
    }
    if !signal.is_empty() {
        sections.push(format!("Retrieved signals:\n{}", signal.join("\n")));
    }

    format!(
        "RepoMemory says the latest durable context for this repo is:\n{}\n\nSummary: {}",
        sections.join("\n\n"),
        context.summary
    )
}

pub async fn submit_smith_rejection_candidate(
    http: &reqwest::Client,
    issue: &Value,
    scope: &IssueScope,
    selected_files: &[String],
    patch_diff: &str,
    feedback: &str,
    smith_confidence: i32,
    min_confidence: i32,
    run_id: &str,
) -> AnyhowResult<Option<patchhive_product_core::repo_memory::FailGuardCandidateResponse>> {
    let candidate = build_smith_rejection_candidate(
        issue,
        scope,
        selected_files,
        patch_diff,
        feedback,
        smith_confidence,
        min_confidence,
        run_id,
    );
    submit_failguard_candidate(http, &candidate).await
}

fn build_smith_rejection_candidate(
    issue: &Value,
    scope: &IssueScope,
    selected_files: &[String],
    patch_diff: &str,
    feedback: &str,
    smith_confidence: i32,
    min_confidence: i32,
    run_id: &str,
) -> FailGuardCandidateRequest {
    let issue_title = issue["title"].as_str().unwrap_or("Untitled issue");
    let issue_url = issue["url"].as_str().unwrap_or("");
    let issue_ref = if issue_url.trim().is_empty() {
        format!("{}#{}", scope.repo, scope.issue_num)
    } else {
        issue_url.to_string()
    };
    let mut affected_paths = selected_files
        .iter()
        .filter(|path| !path.trim().is_empty())
        .take(12)
        .cloned()
        .collect::<Vec<_>>();
    for path in diff_paths(patch_diff) {
        if affected_paths.len() >= 12 {
            break;
        }
        if !affected_paths.iter().any(|existing| existing == &path) {
            affected_paths.push(path);
        }
    }

    let feedback = feedback.trim();
    let feedback_text = if feedback.is_empty() {
        "Smith rejected the generated patch without detailed feedback."
    } else {
        feedback
    };
    let mut evidence = vec![
        format!("RepoReaper run {run_id}"),
        format!("Issue #{}: {issue_title}", scope.issue_num),
        format!("Smith confidence: {smith_confidence}% below required {min_confidence}%"),
        format!("Smith feedback: {feedback_text}"),
    ];
    if !issue_url.trim().is_empty() {
        evidence.push(issue_url.to_string());
    }
    if !affected_paths.is_empty() {
        evidence.push(format!("Affected paths: {}", affected_paths.join(", ")));
    }

    FailGuardCandidateRequest {
        repo: scope.repo.clone(),
        source_type: "repo-reaper-rejection".into(),
        source_ref: issue_ref,
        title: short_text(
            &format!("RepoReaper rejection: {issue_title}"),
            140,
        ),
        outcome: short_text(
            &format!(
                "Smith rejected RepoReaper's patch for issue #{} because confidence was {smith_confidence}% below the configured {min_confidence}% threshold. Feedback: {feedback_text}",
                scope.issue_num
            ),
            320,
        ),
        lesson: short_text(
            &format!(
                "A previous autonomous patch attempt failed Smith review. Future runs should account for this feedback before touching similar files: {feedback_text}"
            ),
            260,
        ),
        prevention: short_text(
            &format!(
                "Before retrying similar RepoReaper work, address Smith's feedback, narrow the patch scope, and add evidence that the change fixes issue #{} without creating the rejected risk.",
                scope.issue_num
            ),
            260,
        ),
        affected_paths,
        evidence,
        confidence: Some((100 - smith_confidence).clamp(55, 92) as f64),
    }
}

pub fn diff_paths(patch_diff: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in patch_diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git a/") {
            let path = rest.split(" b/").next().unwrap_or("").trim().to_string();
            if !path.is_empty() && !paths.iter().any(|existing| existing == &path) {
                paths.push(path);
            }
        }
    }
    paths
}

pub fn short_text(value: &str, limit: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= limit {
        return trimmed.to_string();
    }
    let mut out = trimmed
        .chars()
        .take(limit.saturating_sub(3))
        .collect::<String>();
    out.push_str("...");
    out
}

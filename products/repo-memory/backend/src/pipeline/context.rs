// context.rs - Context ranking for memory retrieval

use std::collections::HashSet;

use crate::models::{ContextEntry, MemoryEntry};

use super::{
    path_bucket, tokenize_context,
    normalize_disposition,
};

pub fn rank_context_entries(
    entries: &[MemoryEntry],
    consumer: &str,
    changed_paths: &[String],
    task_summary: &str,
    diff_summary: &str,
    limit: usize,
) -> Vec<ContextEntry> {
    let clean_paths: Vec<String> = changed_paths
        .iter()
        .map(|path| path.trim().trim_start_matches("./").to_string())
        .filter(|path| !path.is_empty())
        .collect();
    let context_tokens = tokenize_context(&format!("{task_summary} {diff_summary}"));

    let mut ranked = entries
        .iter()
        .map(|entry| {
            let disposition = normalize_disposition(&entry.disposition).to_string();
            if disposition == "suppressed" {
                return ContextEntry {
                    id: entry.id.clone(),
                    memory_ref: entry.memory_ref.clone(),
                    kind: entry.kind.clone(),
                    title: entry.title.clone(),
                    detail: entry.detail.clone(),
                    prompt_line: entry.prompt_line.clone(),
                    confidence: entry.confidence,
                    frequency: entry.frequency,
                    retrieval_score: -10_000.0,
                    disposition,
                    pinned: entry.pinned,
                    matched_paths: Vec::new(),
                    matched_terms: Vec::new(),
                    tags: entry.tags.clone(),
                    evidence: entry.evidence.clone(),
                };
            }
            let matched_paths = matching_entry_paths(entry, &clean_paths);
            let matched_terms = matching_entry_terms(entry, &context_tokens);
            let retrieval_score = entry.confidence * 0.48
                + (entry.frequency as f64 * 6.0)
                + (matched_paths.len() as f64 * 18.0)
                + (matched_terms.len() as f64 * 7.0)
                + context_kind_bonus(entry, &clean_paths, consumer)
                + profile_path_bonus(entry, &clean_paths, &matched_paths, consumer)
                + curation_bonus(entry);

            ContextEntry {
                id: entry.id.clone(),
                memory_ref: entry.memory_ref.clone(),
                kind: entry.kind.clone(),
                title: entry.title.clone(),
                detail: entry.detail.clone(),
                prompt_line: entry.prompt_line.clone(),
                confidence: entry.confidence,
                frequency: entry.frequency,
                retrieval_score,
                disposition,
                pinned: entry.pinned,
                matched_paths,
                matched_terms,
                tags: entry.tags.clone(),
                evidence: entry.evidence.clone(),
            }
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then_with(|| {
                disposition_rank(&right.disposition).cmp(&disposition_rank(&left.disposition))
            })
            .then_with(|| {
                right
                    .retrieval_score
                    .partial_cmp(&left.retrieval_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.frequency.cmp(&left.frequency))
    });

    let fallback_mode = clean_paths.is_empty() && context_tokens.is_empty();
    ranked
        .into_iter()
        .filter(|entry| entry.disposition != "suppressed")
        .filter(|entry| {
            fallback_mode
                || entry.pinned
                || entry.disposition == "policy"
                || !entry.matched_paths.is_empty()
                || !entry.matched_terms.is_empty()
                || entry.retrieval_score >= 68.0
        })
        .take(limit)
        .collect()
}

pub fn matching_entry_paths(entry: &MemoryEntry, changed_paths: &[String]) -> Vec<String> {
    let mut matched = Vec::new();
    for path in changed_paths {
        let path_lower = path.to_ascii_lowercase();
        let path_bucket = path_bucket(path).to_ascii_lowercase();
        let text = format!(
            "{} {} {} {}",
            entry.title,
            entry.detail,
            entry.prompt_line,
            entry.tags.join(" ")
        )
        .to_ascii_lowercase();

        let direct_match = entry
            .evidence
            .iter()
            .filter_map(|evidence| evidence.path.as_ref())
            .any(|candidate| path_matches_candidate(&path_lower, &path_bucket, candidate));

        if direct_match || text.contains(&path_bucket) || text.contains(&path_lower) {
            matched.push(path.clone());
        }
    }
    matched.sort();
    matched.dedup();
    matched
}

pub fn matching_entry_terms(entry: &MemoryEntry, context_tokens: &HashSet<String>) -> Vec<String> {
    if context_tokens.is_empty() {
        return Vec::new();
    }

    let entry_tokens = tokenize_context(&format!(
        "{} {} {} {} {}",
        entry.title,
        entry.detail,
        entry.prompt_line,
        entry.tags.join(" "),
        entry
            .evidence
            .iter()
            .map(|evidence| evidence.excerpt.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    ));

    let mut matched = context_tokens
        .intersection(&entry_tokens)
        .cloned()
        .collect::<Vec<_>>();
    matched.sort();
    matched.truncate(4);
    matched
}

pub fn context_kind_bonus(entry: &MemoryEntry, changed_paths: &[String], consumer: &str) -> f64 {
    match consumer {
        "trust-gate" => match entry.kind.as_str() {
            "testing_expectation" => 11.0,
            "failure_pattern" => 9.0,
            "hotspot" if !changed_paths.is_empty() => 8.0,
            "reviewer_profile" => 6.0,
            "maintainer_profile" => 3.0,
            "review_rule" => 5.0,
            _ => 0.0,
        },
        "repo-reaper" => match entry.kind.as_str() {
            "hotspot" if !changed_paths.is_empty() => 11.0,
            "failure_pattern" => 8.0,
            "maintainer_profile" => 6.0,
            "reviewer_profile" => 5.0,
            "review_rule" => 7.0,
            "testing_expectation" => 6.0,
            _ => 0.0,
        },
        _ => match entry.kind.as_str() {
            "hotspot" if !changed_paths.is_empty() => 9.0,
            "failure_pattern" => 7.0,
            "testing_expectation" => 6.0,
            "reviewer_profile" => 4.0,
            "maintainer_profile" => 4.0,
            "review_rule" => 4.0,
            _ => 0.0,
        },
    }
}

pub fn profile_path_bonus(
    entry: &MemoryEntry,
    changed_paths: &[String],
    matched_paths: &[String],
    consumer: &str,
) -> f64 {
    if changed_paths.is_empty() || matched_paths.is_empty() {
        return 0.0;
    }

    let matched_count = matched_paths.len().min(3) as f64;
    let coverage = matched_paths.len() as f64 / changed_paths.len().max(1) as f64;
    let evidence_hits = changed_paths
        .iter()
        .filter(|path| entry_path_focuses_on(entry, path))
        .count()
        .min(3) as f64;

    match (consumer, entry.kind.as_str()) {
        ("trust-gate", "reviewer_profile") => {
            16.0 + (matched_count * 5.0) + (coverage * 10.0) + (evidence_hits * 4.0)
        }
        ("trust-gate", "maintainer_profile") => {
            8.0 + (matched_count * 4.0) + (coverage * 8.0) + (evidence_hits * 3.0)
        }
        ("repo-reaper", "maintainer_profile") => {
            16.0 + (matched_count * 5.0) + (coverage * 10.0) + (evidence_hits * 4.0)
        }
        ("repo-reaper", "reviewer_profile") => {
            8.0 + (matched_count * 4.0) + (coverage * 8.0) + (evidence_hits * 3.0)
        }
        (_, "reviewer_profile" | "maintainer_profile") => {
            10.0 + (matched_count * 4.0) + (coverage * 8.0) + (evidence_hits * 3.0)
        }
        _ => 0.0,
    }
}

pub fn entry_path_focuses_on(entry: &MemoryEntry, path: &str) -> bool {
    let path_lower = path.to_ascii_lowercase();
    let path_bucket = path_bucket(path).to_ascii_lowercase();

    entry
        .evidence
        .iter()
        .filter_map(|evidence| evidence.path.as_ref())
        .any(|candidate| path_matches_candidate(&path_lower, &path_bucket, candidate))
}

pub fn path_matches_candidate(path_lower: &str, path_bucket_lower: &str, candidate: &str) -> bool {
    let candidate = candidate.to_ascii_lowercase();
    path_lower == candidate
        || path_lower.starts_with(&(candidate.clone() + "/"))
        || candidate.starts_with(&(path_lower.to_string() + "/"))
        || path_bucket_lower == candidate
}

pub fn curation_bonus(entry: &MemoryEntry) -> f64 {
    let mut score = 0.0;
    if entry.pinned {
        score += 24.0;
    }
    if normalize_disposition(&entry.disposition) == "policy" {
        score += 18.0;
    }
    score
}

pub fn disposition_rank(disposition: &str) -> i32 {
    match normalize_disposition(disposition) {
        "policy" => 2,
        "signal" => 1,
        _ => 0,
    }
}

// Note: build_entry and confidence_for stay in pipeline.rs
// They are used by both memory_run.rs and failguard.rs

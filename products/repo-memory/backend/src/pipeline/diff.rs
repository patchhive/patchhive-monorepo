// diff.rs - Run diff functions for comparing memory snapshots

use std::collections::HashMap;

use crate::models::{MemoryEntry, RunDiffItem, RunDiffResponse, RunDiffSummary};

pub fn build_run_diff(current: IngestRecord, previous: Option<IngestRecord>) -> RunDiffResponse {
    let Some(previous) = previous else {
        return RunDiffResponse {
            repo: current.repo.clone(),
            run_id: current.id.clone(),
            previous_run_id: None,
            created_at: current.created_at.clone(),
            previous_created_at: None,
            summary: "This is the first recorded RepoMemory run for this repo, so there is no earlier memory snapshot to compare yet.".into(),
            counts: RunDiffSummary {
                new_entries: current.entries.len() as u32,
                strengthened_entries: 0,
                faded_entries: 0,
                retired_entries: 0,
            },
            new_entries: current
                .entries
                .iter()
                .map(|entry| diff_item(entry, None))
                .take(8)
                .collect(),
            strengthened_entries: Vec::new(),
            faded_entries: Vec::new(),
            retired_entries: Vec::new(),
        };
    };

    let current_repo = current.repo.clone();
    let current_id = current.id.clone();
    let current_created_at = current.created_at.clone();
    let previous_id = previous.id.clone();
    let previous_created_at = previous.created_at.clone();
    let current_entries = current.entries;
    let previous_entries = previous.entries;

    let current_map = current_entries
        .iter()
        .map(|entry| (entry.memory_ref.clone(), entry))
        .collect::<HashMap<_, _>>();
    let previous_map = previous_entries
        .iter()
        .map(|entry| (entry.memory_ref.clone(), entry))
        .collect::<HashMap<_, _>>();

    let mut new_entries = Vec::new();
    let mut strengthened_entries = Vec::new();
    let mut faded_entries = Vec::new();
    let mut retired_entries = Vec::new();

    for entry in &current_entries {
        match previous_map.get(&entry.memory_ref) {
            None => new_entries.push(diff_item(entry, None)),
            Some(previous_entry) => {
                let delta_confidence = entry.confidence - previous_entry.confidence;
                let delta_frequency = entry.frequency as i32 - previous_entry.frequency as i32;
                if delta_confidence >= 4.0 || delta_frequency >= 1 {
                    strengthened_entries.push(diff_item(entry, Some(previous_entry)));
                } else if delta_confidence <= -4.0 || delta_frequency <= -1 {
                    faded_entries.push(diff_item(entry, Some(previous_entry)));
                }
            }
        }
    }

    for entry in &previous_entries {
        if !current_map.contains_key(&entry.memory_ref) {
            retired_entries.push(RunDiffItem {
                memory_ref: entry.memory_ref.clone(),
                kind: entry.kind.clone(),
                title: entry.title.clone(),
                prompt_line: entry.prompt_line.clone(),
                current_confidence: None,
                previous_confidence: Some(entry.confidence),
                current_frequency: None,
                previous_frequency: Some(entry.frequency),
                delta_confidence: -entry.confidence,
                delta_frequency: -(entry.frequency as i32),
            });
        }
    }

    sort_diff_items(&mut new_entries);
    sort_diff_items(&mut strengthened_entries);
    sort_diff_items(&mut faded_entries);
    sort_diff_items(&mut retired_entries);

    let counts = RunDiffSummary {
        new_entries: new_entries.len() as u32,
        strengthened_entries: strengthened_entries.len() as u32,
        faded_entries: faded_entries.len() as u32,
        retired_entries: retired_entries.len() as u32,
    };

    let summary = if counts.new_entries == 0
        && counts.strengthened_entries == 0
        && counts.faded_entries == 0
        && counts.retired_entries == 0
    {
        format!(
            "RepoMemory did not detect any major changes between this run and the previous snapshot for {}.",
            current_repo
        )
    } else {
        format!(
            "Compared with the previous RepoMemory run, {} has {} new, {} strengthened, {} faded, and {} retired memories.",
            current_repo,
            counts.new_entries,
            counts.strengthened_entries,
            counts.faded_entries,
            counts.retired_entries,
        )
    };

    RunDiffResponse {
        repo: current_repo,
        run_id: current_id,
        previous_run_id: Some(previous_id),
        created_at: current_created_at,
        previous_created_at: Some(previous_created_at),
        summary,
        counts,
        new_entries: new_entries.into_iter().take(8).collect(),
        strengthened_entries: strengthened_entries.into_iter().take(8).collect(),
        faded_entries: faded_entries.into_iter().take(8).collect(),
        retired_entries: retired_entries.into_iter().take(8).collect(),
    }
}

pub fn diff_item(entry: &MemoryEntry, previous: Option<&MemoryEntry>) -> RunDiffItem {
    let previous_confidence = previous.map(|e| e.confidence);
    let previous_frequency = previous.map(|e| e.frequency);
    RunDiffItem {
        memory_ref: entry.memory_ref.clone(),
        kind: entry.kind.clone(),
        title: entry.title.clone(),
        prompt_line: entry.prompt_line.clone(),
        current_confidence: Some(entry.confidence),
        previous_confidence,
        current_frequency: Some(entry.frequency),
        previous_frequency,
        delta_confidence: entry.confidence - previous_confidence.unwrap_or(0.0),
        delta_frequency: entry.frequency as i32 - previous_frequency.unwrap_or(0) as i32,
    }
}

pub fn sort_diff_items(items: &mut [RunDiffItem]) {
    items.sort_by(|left, right| {
        let right_magnitude =
            right.delta_confidence.abs() + (right.delta_frequency.abs() as f64 * 5.0);
        let left_magnitude =
            left.delta_confidence.abs() + (left.delta_frequency.abs() as f64 * 5.0);
        right_magnitude
            .partial_cmp(&left_magnitude)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.kind.cmp(&left.kind))
            .then_with(|| left.title.cmp(&right.title))
    });
}

// Need to import IngestRecord for build_run_diff
use crate::models::IngestRecord;

// failguard.rs - FailGuard related route handlers and helpers

use axum::{
 extract::{Json, Path, Query},
 Json as JsonResponse,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
 db,
 models::{
  FailGuardCandidate, FailGuardCandidateDismissRequest,
  FailGuardCandidateListResponse, FailGuardCandidatePromoteRequest,
  FailGuardCandidatePromoteResponse, FailGuardCandidateRequest,
  FailGuardCandidateResponse, FailGuardLessonRequest, FailGuardLessonResponse,
  IngestRecord, MemoryEntry, MemoryEvidence, stable_memory_ref,
 },
};

// Import shared helpers from parent module
use super::{
    build_prompt_pack, build_summary, disposition_rank, normalize_disposition, path_bucket,
    truncate, valid_repo, JsonError, JsonResult, internal_error, bad_request, not_found,
};
use crate::pipeline::utils::normalize_candidate_status;

pub async fn capture_failguard_lesson(
    Json(request): Json<FailGuardLessonRequest>,
) -> JsonResult<FailGuardLessonResponse> {
    Ok(JsonResponse(save_failguard_lesson(request)?))
}

pub async fn failguard_candidates(
    Query(query): Query<FailGuardCandidateQuery>,
) -> JsonResult<FailGuardCandidateListResponse> {
    let status = normalize_candidate_status(query.status.as_deref().unwrap_or("open")).to_string();
    let candidates = db::list_failguard_candidates(query.repo.as_deref(), Some(&status))
        .map_err(internal_error)?;
    Ok(JsonResponse(FailGuardCandidateListResponse { candidates }))
}

pub async fn create_failguard_candidate(
    Json(mut request): Json<FailGuardCandidateRequest>,
) -> JsonResult<FailGuardCandidateResponse> {
    request.repo = request.repo.trim().to_string();
    request.title = request.title.trim().to_string();
    request.outcome = request.outcome.trim().to_string();
    request.lesson = request.lesson.trim().to_string();
    request.prevention = request.prevention.trim().to_string();
    request.source_type = normalize_source_type(&request.source_type);
    request.source_ref = request.source_ref.trim().to_string();

    if !valid_repo(&request.repo) {
        return Err(bad_request(
            "RepoMemory expects repos in owner/repo format.",
        ));
    }
    if request.title.is_empty() {
        return Err(bad_request("FailGuard candidates need a short title."));
    }
    if request.outcome.is_empty() {
        return Err(bad_request(
            "FailGuard candidates need the bad outcome they came from.",
        ));
    }

    let candidate = build_failguard_candidate(request);
    db::save_failguard_candidate(&candidate).map_err(internal_error)?;

    Ok(JsonResponse(FailGuardCandidateResponse {
        ok: true,
        message: "FailGuard candidate queued for review.".into(),
        candidate,
    }))
}

pub async fn promote_failguard_candidate(
    Path(id): Path<String>,
    Json(request): Json<FailGuardCandidatePromoteRequest>,
) -> JsonResult<FailGuardCandidatePromoteResponse> {
    let candidate = db::get_failguard_candidate(id.trim())
        .map_err(internal_error)?
        .ok_or_else(|| not_found("FailGuard candidate not found."))?;

    if candidate.status != "open" {
        return Err(bad_request("FailGuard can only promote open candidates."));
    }

    let response = save_failguard_lesson(candidate_to_lesson_request(&candidate, request))?;
    let note = "Promoted to RepoMemory failure-pattern policy.";
    db::update_failguard_candidate_status(
        &candidate.id,
        "promoted",
        Some(&response.entry.memory_ref),
        note,
    )
    .map_err(internal_error)?;
    let updated = db::get_failguard_candidate(&candidate.id)
        .map_err(internal_error)?
        .unwrap_or(candidate);

    Ok(JsonResponse(FailGuardCandidatePromoteResponse {
        ok: true,
        message: "FailGuard candidate promoted into RepoMemory.".into(),
        candidate: updated,
        run: response.run,
        entry: response.entry,
    }))
}

pub async fn dismiss_failguard_candidate(
    Path(id): Path<String>,
    Json(request): Json<FailGuardCandidateDismissRequest>,
) -> JsonResult<FailGuardCandidateResponse> {
    let candidate = db::get_failguard_candidate(id.trim())
        .map_err(internal_error)?
        .ok_or_else(|| not_found("FailGuard candidate not found."))?;

    if candidate.status != "open" {
        return Err(bad_request("FailGuard can only dismiss open candidates."));
    }

    let reason = request.reason.trim();
    let note = if reason.is_empty() {
        "Dismissed by operator."
    } else {
        reason
    };
    db::update_failguard_candidate_status(&candidate.id, "dismissed", None, note)
        .map_err(internal_error)?;
    let updated = db::get_failguard_candidate(&candidate.id)
        .map_err(internal_error)?
        .unwrap_or(candidate);

    Ok(JsonResponse(FailGuardCandidateResponse {
        ok: true,
        message: "FailGuard candidate dismissed.".into(),
        candidate: updated,
    }))
}

pub fn save_failguard_lesson(
    mut request: FailGuardLessonRequest,
) -> std::result::Result<FailGuardLessonResponse, JsonError> {
    request.repo = request.repo.trim().to_string();
    request.title = request.title.trim().to_string();
    request.outcome = request.outcome.trim().to_string();
    request.lesson = request.lesson.trim().to_string();
    request.prevention = request.prevention.trim().to_string();
    request.disposition = normalize_disposition(&request.disposition).to_string();

    if !valid_repo(&request.repo) {
        return Err(bad_request(
            "RepoMemory expects repos in owner/repo format.",
        ));
    }
    if request.title.is_empty() {
        return Err(bad_request("FailGuard needs a short lesson title."));
    }
    if request.outcome.is_empty() {
        return Err(bad_request(
            "FailGuard needs the bad outcome this lesson came from.",
        ));
    }
    if request.lesson.is_empty() {
        return Err(bad_request(
            "FailGuard needs the durable lesson this repo should remember.",
        ));
    }
    if request.prevention.is_empty() {
        return Err(bad_request(
            "FailGuard needs the future prevention rule or guardrail.",
        ));
    }

    let carry_forward = latest_repo_entries(&request.repo)?;
    let captured_title = format!("FailGuard: {}", request.title);
    let run = build_failguard_lesson_run(request, carry_forward);
    let entry = run
        .entries
        .iter()
        .find(|entry| entry.title == captured_title)
        .cloned()
        .ok_or_else(|| internal_error(anyhow::anyhow!("FailGuard lesson did not create memory")))?;

    db::save_run(&run).map_err(internal_error)?;
    db::save_memory_curation(
        &entry.repo,
        &entry.memory_ref,
        &entry.disposition,
        entry.pinned,
    )
    .map_err(internal_error)?;

    Ok(FailGuardLessonResponse {
        ok: true,
        message: "FailGuard lesson captured as a RepoMemory failure-pattern policy.".into(),
        run,
        entry,
    })
}

fn latest_repo_entries(repo: &str) -> std::result::Result<Vec<MemoryEntry>, JsonError> {
    let latest = db::list_history(Some(repo))
        .map_err(internal_error)?
        .into_iter()
        .next();
    let Some(latest) = latest else {
        return Ok(Vec::new());
    };

    let run = db::get_history(&latest.id)
        .map_err(internal_error)?
        .ok_or_else(|| not_found("RepoMemory latest run not found."))?;
    Ok(run.entries)
}

pub fn build_failguard_candidate(request: FailGuardCandidateRequest) -> FailGuardCandidate {
    let now = Utc::now().to_rfc3339();
    let affected_paths = clean_failguard_items(request.affected_paths, 12);
    let mut evidence = clean_failguard_items(request.evidence, 10);
    if !request.source_ref.trim().is_empty()
        && !evidence
            .iter()
            .any(|item| item.trim() == request.source_ref.trim())
    {
        evidence.insert(0, request.source_ref.trim().to_string());
        evidence.truncate(10);
    }
    let lesson = if request.lesson.trim().is_empty() {
        draft_failguard_lesson(&request.title, &request.outcome)
    } else {
        truncate(request.lesson.trim(), 260)
    };
    let prevention = if request.prevention.trim().is_empty() {
        draft_failguard_prevention(&request.title, &request.outcome, &affected_paths)
    } else {
        truncate(request.prevention.trim(), 260)
    };

    FailGuardCandidate {
        id: Uuid::new_v4().to_string(),
        repo: request.repo.trim().to_string(),
        source_type: normalize_source_type(&request.source_type),
        source_ref: request.source_ref.trim().to_string(),
        title: truncate(request.title.trim(), 140),
        outcome: truncate(request.outcome.trim(), 320),
        lesson,
        prevention,
        affected_paths,
        evidence,
        confidence: normalized_candidate_confidence(request.confidence, &request.source_type),
        status: "open".into(),
        memory_ref: String::new(),
        resolution_note: String::new(),
        created_at: now.clone(),
        updated_at: now,
    }
}

pub fn candidate_to_lesson_request(
    candidate: &FailGuardCandidate,
    request: FailGuardCandidatePromoteRequest,
) -> FailGuardLessonRequest {
    let title = promote_text(request.title, &candidate.title);
    let outcome = promote_text(request.outcome, &candidate.outcome);
    let lesson = promote_text(request.lesson, &candidate.lesson);
    let prevention = promote_text(request.prevention, &candidate.prevention);
    let affected_paths = request
        .affected_paths
        .unwrap_or_else(|| candidate.affected_paths.clone());
    let evidence = request
        .evidence
        .unwrap_or_else(|| candidate.evidence.clone());

    FailGuardLessonRequest {
        repo: candidate.repo.clone(),
        title,
        outcome,
        lesson,
        prevention,
        affected_paths,
        evidence,
        disposition: normalize_disposition(&request.disposition).to_string(),
        pinned: request.pinned,
    }
}

fn promote_text(next: Option<String>, fallback: &str) -> String {
    next.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn draft_failguard_lesson(title: &str, outcome: &str) -> String {
    truncate(
        &format!(
            "A previous failure showed this repo must treat \"{}\" as a durable guardrail. Outcome to remember: {}",
            title.trim(),
            outcome.trim()
        ),
        260,
    )
}

fn draft_failguard_prevention(title: &str, outcome: &str, affected_paths: &[String]) -> String {
    let scope = affected_paths
        .first()
        .map(|path| format!(" around {path}"))
        .unwrap_or_default();
    truncate(
        &format!(
            "Before similar changes are accepted{scope}, verify the work prevents this failure mode: {}. Original signal: {}",
            title.trim(),
            outcome.trim()
        ),
        260,
    )
}

pub fn normalized_candidate_confidence(confidence: Option<f64>, source_type: &str) -> f64 {
    let default = match normalize_source_type(source_type).as_str() {
        "trustgate-block" | "trust-gate-block" => 86.0,
        "trustgate-warn" | "trust-gate-warn" => 78.0,
        "repo-reaper-rejection" | "repo_reaper_rejection" => 82.0,
        "reverted-pr" | "reverted_pr" => 88.0,
        "reviewbee-thread" | "reviewbee_thread" => 74.0,
        _ => 70.0,
    };
    let value = confidence.unwrap_or(default);
    if value.is_finite() {
        value.clamp(10.0, 96.0)
    } else {
        default
    }
}

pub fn normalize_source_type(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.trim().chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let normalized = out.trim_matches('-');
    if normalized.is_empty() {
        "operator".into()
    } else {
        normalized.to_string()
    }
}

pub fn build_failguard_lesson_run(
    request: FailGuardLessonRequest,
    carry_forward: Vec<MemoryEntry>,
) -> IngestRecord {
    let run_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let repo = request.repo.trim().to_string();
    let title = request.title.trim().to_string();
    let disposition = normalize_disposition(&request.disposition).to_string();
    let pinned = request.pinned || disposition == "policy";
    let affected_paths = clean_failguard_items(request.affected_paths, 12);
    let evidence_notes = clean_failguard_items(request.evidence, 8);
    let memory_title = format!("FailGuard: {title}");
    let outcome = truncate(request.outcome.trim(), 260);
    let lesson = truncate(request.lesson.trim(), 260);
    let prevention = truncate(request.prevention.trim(), 260);
    let detail = format!("Outcome: {outcome}\nLesson: {lesson}\nPrevention: {prevention}");
    let prompt_line = format!("FailGuard policy for {repo}: {prevention} Remember: {lesson}");

    let mut tags = vec![
        "failguard".to_string(),
        "manual".to_string(),
        "failure".to_string(),
        "policy".to_string(),
    ];
    for path in &affected_paths {
        let bucket = path_bucket(path);
        if !bucket.is_empty() {
            tags.push(bucket);
        }
    }
    tags.sort();
    tags.dedup();

    let mut evidence = Vec::new();
    for path in &affected_paths {
        evidence.push(MemoryEvidence {
            source_type: "failguard_path".into(),
            title: "Affected path".into(),
            url: String::new(),
            path: Some(path.clone()),
            excerpt: truncate(&format!("FailGuard lesson applies here: {prevention}"), 220),
        });
    }

    for note in &evidence_notes {
        let url = if note.starts_with("http://") || note.starts_with("https://") {
            note.clone()
        } else {
            String::new()
        };
        evidence.push(MemoryEvidence {
            source_type: "failguard_evidence".into(),
            title: if url.is_empty() {
                "Operator evidence".into()
            } else {
                "External evidence".into()
            },
            url,
            path: None,
            excerpt: truncate(note, 260),
        });
    }

    if evidence.is_empty() {
        evidence.push(MemoryEvidence {
            source_type: "failguard_lesson".into(),
            title: memory_title.clone(),
            url: String::new(),
            path: None,
            excerpt: truncate(&detail, 320),
        });
    }

    let new_memory_ref = stable_memory_ref(&repo, "failure_pattern", &memory_title);
    let entry = MemoryEntry {
        id: Uuid::new_v4().to_string(),
        memory_ref: new_memory_ref.clone(),
        run_id: run_id.clone(),
        repo: repo.clone(),
        kind: "failure_pattern".into(),
        title: memory_title.clone(),
        detail,
        prompt_line: prompt_line.clone(),
        confidence: if disposition == "policy" { 94.0 } else { 82.0 },
        frequency: 1,
        disposition,
        pinned,
        tags,
        evidence,
        created_at: created_at.clone(),
    };

    let mut entries = carry_forward
        .into_iter()
        .filter(|entry| entry.memory_ref != new_memory_ref)
        .map(|mut entry| {
            entry.id = Uuid::new_v4().to_string();
            entry.run_id = run_id.clone();
            entry.created_at = created_at.clone();
            entry
        })
        .collect::<Vec<_>>();
    entries.push(entry);
    entries.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then_with(|| {
                disposition_rank(&right.disposition).cmp(&disposition_rank(&left.disposition))
            })
            .then_with(|| {
                right
                    .confidence
                    .partial_cmp(&left.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.frequency.cmp(&left.frequency))
    });
    let summary = build_summary(&entries, 0, 0, 0);
    let prompt_pack = build_prompt_pack(&repo, &summary, &entries);

    IngestRecord {
        id: run_id,
        repo: repo.clone(),
        created_at,
        params: super::IngestParams {
            repo,
            merged_pr_limit: 0,
            issue_limit: 0,
            since_days: 0,
        },
        summary,
        prompt_pack,
        entries,
    }
}

pub fn clean_failguard_items(items: Vec<String>, limit: usize) -> Vec<String> {
    let mut clean = items
        .into_iter()
        .map(|item| item.trim().trim_start_matches("./").to_string())
        .filter(|item| !item.is_empty())
        .take(limit)
        .collect::<Vec<_>>();
    clean.sort();
    clean.dedup();
    clean
}

#[derive(serde::Deserialize)]
pub struct FailGuardCandidateQuery {
    pub repo: Option<String>,
    pub status: Option<String>,
}

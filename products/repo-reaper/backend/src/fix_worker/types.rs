// types.rs — Shared types, agent selection, scope builders

use anyhow::Result as AnyhowResult;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::state::AgentConfig;

use super::sse::sse_ev;

pub type Tx = tokio::sync::mpsc::Sender<Result<axum::response::sse::Event, std::convert::Infallible>>;

pub struct FixParams {
    pub retry_count: usize,
    pub min_conf: i32,
    pub run_id: String,
    pub cancel_requested: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct FixAgents {
    pub judge: Option<AgentConfig>,
    pub reaper: AgentConfig,
    pub smith: Option<AgentConfig>,
    pub gatekeeper: AgentConfig,
}

pub struct IssueScope {
    pub repo: String,
    pub issue_num: i64,
    pub branch: String,
    pub work_path: PathBuf,
}

pub struct CodeSelection {
    pub selected_files: Vec<String>,
    pub codebase: String,
}

pub struct SmithReviewOutcome {
    pub final_patch: String,
    pub smith_note: String,
}

pub fn work_dir() -> PathBuf {
    PathBuf::from(std::env::var("REAPER_WORK_DIR").unwrap_or_else(|_| "/tmp/repo-reaper".into()))
}

pub fn cancelled(params: &FixParams) -> bool {
    params.cancel_requested.load(Ordering::SeqCst)
}

pub fn pick_fix_agents(
    idx: usize,
    judges: &[AgentConfig],
    reapers: &[AgentConfig],
    smiths: &[AgentConfig],
    gatekeepers: &[AgentConfig],
) -> AnyhowResult<FixAgents> {
    if reapers.is_empty() {
        anyhow::bail!("no reaper agents configured — at least one reaper is required");
    }
    if judges.is_empty() {
        anyhow::bail!("no judge agents configured — at least one judge is required");
    }
    if smiths.is_empty() {
        anyhow::bail!("no smith agents configured — at least one smith is required");
    }

    let judge_idx = idx % judges.len().max(1);
    let reaper_idx = idx % reapers.len().max(1);
    let smith_idx = idx % smiths.len().max(1);
    let gatekeeper_idx = idx % gatekeepers.len().max(1);

    Ok(FixAgents {
        judge: judges.get(judge_idx).cloned(),
        reaper: reapers[reaper_idx].clone(),
        smith: smiths.get(smith_idx).cloned(),
        gatekeeper: gatekeepers
            .get(gatekeeper_idx)
            .cloned()
            .unwrap_or_else(|| reapers[reaper_idx.min(reapers.len().saturating_sub(1))].clone()),
    })
}

pub fn build_issue_scope(issue: &Value) -> IssueScope {
    let repo = issue["repo"].as_str().unwrap_or("").to_string();
    let repo_name = repo.split('/').nth(1).unwrap_or("repo").to_string();
    let issue_num = issue["number"].as_i64().unwrap_or(0);
    let branch = format!("reaper/issue-{issue_num}");
    let work_path = work_dir().join(format!("{repo_name}-{issue_num}"));

    IssueScope {
        repo,
        issue_num,
        branch,
        work_path,
    }
}

pub fn cfg(k: &str) -> String {
    std::env::var(k).unwrap_or_default()
}

pub async fn cleanup_work_path(work_path: &PathBuf) {
    if work_path.exists() {
        let _ = tokio::fs::remove_dir_all(work_path).await;
    }
}

pub async fn finish_skipped_attempt(
    tx: &Tx,
    issue: &Value,
    attempt_id: &str,
    reason: &str,
    cost: f64,
    patch_diff: Option<&str>,
    confidence: i32,
    started_at: &std::time::Instant,
    work_path: &PathBuf,
) {
    let _ = crate::db::finish_attempt(
        attempt_id,
        "skipped",
        None,
        None,
        cost,
        patch_diff,
        None,
        Some(reason),
        Some(started_at.elapsed().as_secs_f64()),
        confidence,
    );
    let _ = tx
        .send(sse_ev(
            "issue_result",
            serde_json::json!({"id":issue["id"],"status":"skipped","reason":reason}),
        ))
        .await;
    cleanup_work_path(work_path).await;
}

pub async fn finish_error_attempt(
    tx: &Tx,
    issue: &Value,
    attempt_id: &str,
    error: &str,
    cost: f64,
    confidence: i32,
    started_at: &std::time::Instant,
    work_path: &PathBuf,
) {
    let _ = crate::db::finish_attempt(
        attempt_id,
        "error",
        None,
        None,
        cost,
        None,
        Some(error),
        None,
        Some(started_at.elapsed().as_secs_f64()),
        confidence,
    );
    let _ = tx
        .send(sse_ev(
            "issue_result",
            serde_json::json!({"id":issue["id"],"status":"error"}),
        ))
        .await;
    cleanup_work_path(work_path).await;
}

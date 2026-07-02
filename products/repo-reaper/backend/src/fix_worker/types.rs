// types.rs — Shared types, agent selection, scope builders

use anyhow::Result as AnyhowResult;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicI64, Ordering},
    Arc,
};

use crate::state::AgentConfig;

use super::sse::sse_ev;

pub type Tx =
    tokio::sync::mpsc::Sender<Result<axum::response::sse::Event, std::convert::Infallible>>;

#[derive(Clone)]
pub struct FixParams {
    pub retry_count: usize,
    pub min_conf: i32,
    pub run_id: String,
    pub cancel_requested: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct FixAgentPools {
    pub judges: Vec<AgentConfig>,
    pub reapers: Vec<AgentConfig>,
    pub smiths: Vec<AgentConfig>,
    pub gatekeepers: Vec<AgentConfig>,
}

#[derive(Clone)]
pub struct FixRunContext {
    pub agents: Arc<FixAgentPools>,
    pub sem: Arc<tokio::sync::Semaphore>,
    pub params: FixParams,
    pub run_cost: Arc<AtomicI64>,
    pub tx: Tx,
    pub http: reqwest::Client,
}

pub struct FixIssueJob {
    pub issue: Value,
    pub idx: usize,
    pub context: FixRunContext,
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

pub fn build_attempt_target(issue: &Value) -> crate::db::IssueAttemptTarget {
    crate::db::IssueAttemptTarget {
        repo: issue["repo"].as_str().unwrap_or("").to_string(),
        issue_number: issue["number"].as_i64().unwrap_or(0),
        issue_title: issue["title"].as_str().unwrap_or("").to_string(),
        issue_url: issue["url"].as_str().unwrap_or("").to_string(),
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
    finish_skipped_attempt_with_error(
        tx, issue, attempt_id, reason, None, cost, patch_diff, confidence, started_at, work_path,
    )
    .await;
}

pub async fn finish_skipped_attempt_with_error(
    tx: &Tx,
    issue: &Value,
    attempt_id: &str,
    reason: &str,
    error_msg: Option<&str>,
    cost: f64,
    patch_diff: Option<&str>,
    confidence: i32,
    started_at: &std::time::Instant,
    work_path: &PathBuf,
) {
    let _ = crate::db::finish_attempt(crate::db::IssueAttemptFinish {
        attempt_id,
        status: crate::db::IssueAttemptStatus::Skipped,
        pr_url: None,
        pr_number: None,
        cost_usd: cost,
        patch_diff,
        error_msg,
        skip_reason: Some(reason),
        duration_seconds: Some(started_at.elapsed().as_secs_f64()),
        confidence,
    });
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
    let _ = crate::db::finish_attempt(crate::db::IssueAttemptFinish {
        attempt_id,
        status: crate::db::IssueAttemptStatus::Error,
        pr_url: None,
        pr_number: None,
        cost_usd: cost,
        patch_diff: None,
        error_msg: Some(error),
        skip_reason: None,
        duration_seconds: Some(started_at.elapsed().as_secs_f64()),
        confidence,
    });
    let _ = tx
        .send(sse_ev(
            "issue_result",
            serde_json::json!({"id":issue["id"],"status":"error"}),
        ))
        .await;
    cleanup_work_path(work_path).await;
}

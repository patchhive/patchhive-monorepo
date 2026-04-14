use anyhow::Result as AnyhowResult;
use axum::response::sse::Event;
use chrono::Utc;
use patchhive_github_pr::github_token_from_env;
use patchhive_product_core::repo_memory::{
    fetch_repo_memory_context, RepoMemoryContextRequest, RepoMemoryContextResponse,
};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicI64, Ordering},
    Arc,
};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::agents::*;
use crate::db::*;
use crate::git_ops::*;
use crate::github::*;
use crate::state::AgentConfig;

type Tx = Sender<Result<Event, Infallible>>;

pub fn work_dir() -> PathBuf {
    PathBuf::from(std::env::var("REAPER_WORK_DIR").unwrap_or_else(|_| "/tmp/repo-reaper".into()))
}

pub fn sse_ev(event: &str, data: Value) -> Result<Event, Infallible> {
    Ok(Event::default().event(event).data(data.to_string()))
}

pub fn sse(event: &str, data: Value) -> Result<Event, Infallible> {
    sse_ev(event, data)
}

fn ts() -> String {
    Utc::now().format("%H:%M:%S").to_string()
}

pub fn alog(agent: &AgentConfig, msg: &str, kind: &str) -> Result<Event, Infallible> {
    sse_ev(
        "agent_log",
        json!({
            "agent_id": agent.id, "agent": agent.name, "role": agent.role,
            "msg": msg, "type": kind, "ts": ts()
        }),
    )
}

pub fn alog_raw(
    agent_id: &str,
    agent_name: &str,
    role: &str,
    msg: &str,
    kind: &str,
) -> Result<Event, Infallible> {
    sse_ev(
        "agent_log",
        json!({
            "agent_id": agent_id, "agent": agent_name, "role": role,
            "msg": msg, "type": kind, "ts": ts()
        }),
    )
}

pub fn astatus(agent_id: &str, status: &str, task: &str) -> Result<Event, Infallible> {
    sse_ev(
        "agent_status",
        json!({"agent_id": agent_id, "status": status, "task": task}),
    )
}

fn cfg(k: &str) -> String {
    std::env::var(k).unwrap_or_default()
}

fn build_repo_memory_block(context: Option<&RepoMemoryContextResponse>) -> String {
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

pub struct FixParams {
    pub retry_count: usize,
    pub min_conf: i32,
    pub run_id: String,
    pub cancel_requested: Arc<AtomicBool>,
}

#[derive(Clone)]
struct FixAgents {
    judge: Option<AgentConfig>,
    reaper: AgentConfig,
    smith: Option<AgentConfig>,
    gatekeeper: AgentConfig,
}

struct IssueScope {
    repo: String,
    issue_num: i64,
    branch: String,
    work_path: PathBuf,
}

struct CodeSelection {
    selected_files: Vec<String>,
    codebase: String,
}

struct SmithReviewOutcome {
    final_patch: String,
    smith_note: String,
}

fn cancelled(params: &FixParams) -> bool {
    params.cancel_requested.load(Ordering::SeqCst)
}

fn pick_fix_agents(
    idx: usize,
    judges: &[AgentConfig],
    reapers: &[AgentConfig],
    smiths: &[AgentConfig],
    gatekeepers: &[AgentConfig],
) -> FixAgents {
    let judge_idx = idx % judges.len().max(1);
    let reaper_idx = idx % reapers.len().max(1);
    let smith_idx = idx % smiths.len().max(1);
    let gatekeeper_idx = idx % gatekeepers.len().max(1);

    FixAgents {
        judge: judges.get(judge_idx).cloned(),
        reaper: reapers
            .get(reaper_idx)
            .cloned()
            .unwrap_or_else(|| reapers[0].clone()),
        smith: smiths.get(smith_idx).cloned(),
        gatekeeper: gatekeepers
            .get(gatekeeper_idx)
            .cloned()
            .unwrap_or_else(|| reapers[reaper_idx.min(reapers.len().saturating_sub(1))].clone()),
    }
}

fn build_issue_scope(issue: &Value) -> IssueScope {
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

async fn cleanup_work_path(work_path: &PathBuf) {
    if work_path.exists() {
        let _ = tokio::fs::remove_dir_all(work_path).await;
    }
}

async fn finish_skipped_attempt(
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
    let _ = finish_attempt(
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
            json!({"id":issue["id"],"status":"skipped","reason":reason}),
        ))
        .await;
    cleanup_work_path(work_path).await;
}

async fn finish_error_attempt(
    tx: &Tx,
    issue: &Value,
    attempt_id: &str,
    error: &str,
    cost: f64,
    confidence: i32,
    started_at: &std::time::Instant,
    work_path: &PathBuf,
) {
    let _ = finish_attempt(
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
            json!({"id":issue["id"],"status":"error"}),
        ))
        .await;
    cleanup_work_path(work_path).await;
}

async fn clone_issue_repo(
    http: &reqwest::Client,
    tx: &Tx,
    issue: &Value,
    scope: &IssueScope,
    reaper: &AgentConfig,
    bot_token: &str,
    bot_user: &str,
) -> AnyhowResult<String> {
    let _ = tx
        .send(astatus(
            &reaper.id,
            "working",
            &format!("Forking {}", scope.repo),
        ))
        .await;
    let _ = tx
        .send(alog(
            reaper,
            &format!("[#{}] Forking {}…", scope.issue_num, scope.repo),
            "info",
        ))
        .await;

    let fork = gh_fork(http, &scope.repo, Some(bot_token), Some(bot_user)).await?;
    cleanup_work_path(&scope.work_path).await;

    git_clone(
        fork["clone_url"].as_str().unwrap_or(""),
        &scope.work_path,
        Some(bot_user),
        Some(bot_token),
    )
    .await?;
    git_branch(&scope.work_path, &scope.branch).await?;

    let issue_ctx = gh_get_issue_context(http, &scope.repo, scope.issue_num, Some(bot_token)).await;
    gh_comment_issue(
        http,
        &scope.repo,
        scope.issue_num,
        &format!(
            "🔱 **RepoReaper** is hunting this bug.\n\n> Fixability score: **{}/100**{}\n\nA pull request will be opened shortly.\n\n*by PatchHive*",
            issue["fixability_score"].as_i64().unwrap_or(50),
            issue["fixability_reason"]
                .as_str()
                .filter(|value| !value.is_empty())
                .map(|reason| format!(" — {reason}"))
                .unwrap_or_default()
        ),
        Some(bot_token),
    )
    .await;

    Ok(issue_ctx)
}

async fn select_code_context(
    http: &reqwest::Client,
    tx: &Tx,
    issue: &Value,
    scope: &IssueScope,
    judge: Option<&AgentConfig>,
) -> (CodeSelection, f64) {
    let Some(judge) = judge else {
        return (
            CodeSelection {
                selected_files: Vec::new(),
                codebase: collect_files_all(&scope.work_path, 60_000).await,
            },
            0.0,
        );
    };

    let _ = tx
        .send(astatus(
            &judge.id,
            "working",
            &format!("#{}", scope.issue_num),
        ))
        .await;
    let structure = collect_repo_structure(&scope.work_path).await;
    let result = agent_select_files(
        http,
        &structure,
        issue["title"].as_str().unwrap_or(""),
        issue["body"].as_str().unwrap_or(""),
        judge,
    )
    .await;
    let _ = tx.send(astatus(&judge.id, "idle", "")).await;

    match result {
        Ok((files, cost)) if !files.is_empty() => {
            let _ = tx
                .send(alog(
                    judge,
                    &format!(
                        "Targeted {} files: {}",
                        files.len(),
                        files.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                    ),
                    "success",
                ))
                .await;
            let codebase = collect_files_selective(&scope.work_path, &files, 80_000).await;
            (
                CodeSelection {
                    selected_files: files,
                    codebase,
                },
                cost,
            )
        }
        Ok(_) => (
            CodeSelection {
                selected_files: Vec::new(),
                codebase: collect_files_all(&scope.work_path, 60_000).await,
            },
            0.0,
        ),
        Err(e) => {
            let _ = tx
                .send(alog(
                    judge,
                    &format!("Targeting failed (fallback): {e}"),
                    "warn",
                ))
                .await;
            (
                CodeSelection {
                    selected_files: Vec::new(),
                    codebase: collect_files_all(&scope.work_path, 60_000).await,
                },
                0.0,
            )
        }
    }
}

async fn load_enriched_issue_context(
    http: &reqwest::Client,
    tx: &Tx,
    issue: &Value,
    reaper: &AgentConfig,
    selected_files: &[String],
    issue_ctx: &str,
) -> String {
    let repo_memory_context = match fetch_repo_memory_context(
        http,
        &RepoMemoryContextRequest {
            repo: issue["repo"].as_str().unwrap_or("").to_string(),
            consumer: "repo-reaper".into(),
            changed_paths: selected_files.to_vec(),
            task_summary: format!(
                "Fix GitHub issue #{} in {}: {}",
                issue["number"].as_i64().unwrap_or(0),
                issue["repo"].as_str().unwrap_or(""),
                issue["title"].as_str().unwrap_or("")
            ),
            diff_summary: issue_ctx.chars().take(1200).collect::<String>(),
            limit: 5,
        },
    )
    .await
    {
        Ok(context) => {
            if let Some(ref context) = context {
                if !context.entries.is_empty() {
                    let _ = tx
                        .send(alog(
                            reaper,
                            &format!("Loaded {} RepoMemory hints", context.entries.len()),
                            "info",
                        ))
                        .await;
                }
            }
            context
        }
        Err(e) => {
            let _ = tx
                .send(alog(
                    reaper,
                    &format!("RepoMemory unavailable (continuing): {e}"),
                    "warn",
                ))
                .await;
            None
        }
    };

    let block = build_repo_memory_block(repo_memory_context.as_ref());
    if block.is_empty() {
        issue_ctx.to_string()
    } else {
        format!("{issue_ctx}\n\n{block}")
    }
}

async fn apply_patch_with_self_heal(
    http: &reqwest::Client,
    tx: &Tx,
    issue: &Value,
    scope: &IssueScope,
    reaper: &AgentConfig,
    codebase: &str,
    enriched_issue_ctx: &str,
    mut result: Value,
    cost: &mut f64,
) -> std::result::Result<Value, String> {
    let patch_str = result["patch"].as_str().unwrap_or("").to_string();
    let (mut applied, apply_err) = apply_patch(&scope.work_path, &patch_str).await;

    if !applied {
        let _ = tx
            .send(alog(reaper, "Apply failed — self-healing…", "warn"))
            .await;
        if let Ok((retry_result, retry_cost)) = agent_patch_retry(
            http,
            issue["title"].as_str().unwrap_or(""),
            issue["body"].as_str().unwrap_or(""),
            codebase,
            &patch_str,
            &format!("git apply error:\n{apply_err}\n\n{enriched_issue_ctx}"),
            reaper,
        )
        .await
        {
            *cost += retry_cost;
            if !retry_result["patch"].is_null() {
                let (ok, err) = apply_patch(
                    &scope.work_path,
                    retry_result["patch"].as_str().unwrap_or(""),
                )
                .await;
                if ok {
                    result = retry_result;
                    applied = true;
                    let _ = tx.send(alog(reaper, "Self-healed ✓", "success")).await;
                } else {
                    let _ = tx
                        .send(alog(
                            reaper,
                            &format!("Self-heal apply failed: {err}"),
                            "warn",
                        ))
                        .await;
                }
            }
        }
    }

    if !applied {
        let _ = tx
            .send(alog(reaper, "Cannot apply patch — skipping", "error"))
            .await;
        return Err("apply_failed".into());
    }

    Ok(result)
}

async fn publish_pull_request(
    http: &reqwest::Client,
    issue: &Value,
    scope: &IssueScope,
    agents: &FixAgents,
    bot_token: &str,
    bot_user: &str,
    result: &Value,
    smith_note: &str,
    confidence: i32,
    test: &TestResult,
) -> AnyhowResult<(Value, i64)> {
    let commit_msg = format!(
        "fix: {} (closes #{})",
        issue["title"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(72)
            .collect::<String>(),
        scope.issue_num,
    );
    git_commit_push(&scope.work_path, &scope.branch, &commit_msg).await?;

    let files_changed: Vec<String> = result["files_changed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(|path| path.to_string()))
        .collect();
    let files_md = files_changed
        .iter()
        .map(|path| format!("- `{path}`"))
        .collect::<Vec<_>>()
        .join("\n");

    let pr_body = format!(
        "## 🔱 Reaping #{}: {}\n\n\
        ### What changed\n{}\n\n\
        **Reaper confidence:** {confidence}/100\n\n\
        ### Files targeted\n{files_md}\n\n\
        ### Fixability Score\n**{}/100** — {}\n\n\
        {smith_note}\n\n\
        ### Tests\n{}\n\n\
        ---\n\
        ⚖ Judge: {} · ⚔ Reaper: {} · ⬢ Smith: {} · 🔒 Gatekeeper: {}\n\n\
        *RepoReaper by PatchHive · Closes #{}*",
        scope.issue_num,
        issue["title"].as_str().unwrap_or(""),
        result["explanation"].as_str().unwrap_or(""),
        issue["fixability_score"].as_i64().unwrap_or(50),
        issue["fixability_reason"].as_str().unwrap_or(""),
        if test.passed {
            "✅ Passed"
        } else {
            "⚠️ Failed (draft PR)"
        },
        agents
            .judge
            .as_ref()
            .map(|judge| judge.name.as_str())
            .unwrap_or("none"),
        agents.reaper.name,
        agents
            .smith
            .as_ref()
            .map(|smith| smith.name.as_str())
            .unwrap_or("none"),
        agents.gatekeeper.name,
        scope.issue_num,
    );

    let base_branch = gh_default_branch(http, &scope.repo, Some(bot_token))
        .await
        .unwrap_or_else(|| "main".to_string());
    let pr = gh_post(
        http,
        &format!("/repos/{}/pulls", scope.repo),
        &json!({
            "title": commit_msg,
            "body": pr_body,
            "head": format!("{bot_user}:{}", scope.branch),
            "base": base_branch,
            "draft": !test.passed,
        }),
        Some(bot_token),
    )
    .await?;

    Ok((pr.clone(), pr["number"].as_i64().unwrap_or(0)))
}

pub async fn fix_one(
    issue: Value,
    idx: usize,
    judges: Vec<AgentConfig>,
    reapers: Vec<AgentConfig>,
    smiths: Vec<AgentConfig>,
    gatekeepers: Vec<AgentConfig>,
    sem: Arc<tokio::sync::Semaphore>,
    params: FixParams,
    run_cost: Arc<AtomicI64>,
    tx: Tx,
    http: reqwest::Client,
) {
    let _permit = sem.acquire().await.unwrap();
    if cancelled(&params) {
        return;
    }

    let agents = pick_fix_agents(idx, &judges, &reapers, &smiths, &gatekeepers);
    let scope = build_issue_scope(&issue);
    let attempt_id = Uuid::new_v4().to_string()[..12].to_string();
    let t_start = std::time::Instant::now();
    let mut cost = 0.0f64;

    let bot_token = agents
        .reaper
        .bot_token
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(github_token_from_env)
        .unwrap_or_default();
    let bot_user = agents
        .reaper
        .bot_user
        .clone()
        .unwrap_or_else(|| cfg("BOT_GITHUB_USER"));

    if gh_check_duplicate(
        &http,
        &scope.repo,
        &scope.branch,
        Some(&bot_user),
        Some(&bot_token),
    )
    .await
    {
        let _ = tx
            .send(alog(
                &agents.reaper,
                &format!("[#{}] Branch/PR exists — skipping", scope.issue_num),
                "warn",
            ))
            .await;
        let _ = tx
            .send(sse_ev(
                "issue_result",
                json!({"id":issue["id"],"status":"skipped","reason":"duplicate"}),
            ))
            .await;
        return;
    }

    let _ = tx
        .send(sse_ev(
            "issue_assign",
            json!({
                "id": issue["id"],
                "score": issue["fixability_score"],
                "reaper": agents.reaper.id,
                "judge": agents.judge.as_ref().map(|judge| judge.id.as_str()),
                "smith": agents.smith.as_ref().map(|smith| smith.id.as_str()),
                "gatekeeper": agents.gatekeeper.id,
            }),
        ))
        .await;

    let _ = start_attempt(
        &attempt_id,
        &params.run_id,
        &issue,
        &agents.reaper.name,
        agents.smith.as_ref().map(|smith| smith.name.as_str()),
        &agents.gatekeeper.name,
    );

    let issue_ctx = match clone_issue_repo(
        &http,
        &tx,
        &issue,
        &scope,
        &agents.reaper,
        &bot_token,
        &bot_user,
    )
    .await
    {
        Ok(context) => context,
        Err(e) => {
            finish_error_attempt(
                &tx,
                &issue,
                &attempt_id,
                &e.to_string(),
                cost,
                0,
                &t_start,
                &scope.work_path,
            )
            .await;
            return;
        }
    };

    if cancelled(&params) {
        finish_skipped_attempt(
            &tx,
            &issue,
            &attempt_id,
            "cancelled",
            cost,
            None,
            0,
            &t_start,
            &scope.work_path,
        )
        .await;
        return;
    }

    let (code_selection, selection_cost) =
        select_code_context(&http, &tx, &issue, &scope, agents.judge.as_ref()).await;
    cost += selection_cost;
    let enriched_issue_ctx = load_enriched_issue_context(
        &http,
        &tx,
        &issue,
        &agents.reaper,
        &code_selection.selected_files,
        &issue_ctx,
    )
    .await;

    if cancelled(&params) {
        finish_skipped_attempt(
            &tx,
            &issue,
            &attempt_id,
            "cancelled",
            cost,
            None,
            0,
            &t_start,
            &scope.work_path,
        )
        .await;
        return;
    }

    let _ = tx
        .send(astatus(
            &agents.reaper.id,
            "working",
            &format!("Reaping #{}", scope.issue_num),
        ))
        .await;
    let patch_result = agent_generate_patch(
        &http,
        issue["title"].as_str().unwrap_or(""),
        issue["body"].as_str().unwrap_or(""),
        &code_selection.codebase,
        &enriched_issue_ctx,
        &agents.reaper,
    )
    .await;

    let (result, patch_cost) = match patch_result {
        Ok(result) => result,
        Err(e) => {
            let _ = tx
                .send(alog(
                    &agents.reaper,
                    &format!("Patch generation error: {e}"),
                    "error",
                ))
                .await;
            finish_skipped_attempt(
                &tx,
                &issue,
                &attempt_id,
                "patch_error",
                cost,
                None,
                0,
                &t_start,
                &scope.work_path,
            )
            .await;
            return;
        }
    };
    cost += patch_cost;

    if result["patch"]
        .as_str()
        .map(|patch| patch.trim().is_empty())
        .unwrap_or(true)
    {
        let _ = tx
            .send(alog(
                &agents.reaper,
                &format!("No patch: {}", result["explanation"].as_str().unwrap_or("")),
                "warn",
            ))
            .await;
        finish_skipped_attempt(
            &tx,
            &issue,
            &attempt_id,
            "no_patch",
            cost,
            None,
            0,
            &t_start,
            &scope.work_path,
        )
        .await;
        return;
    }

    let confidence = result["confidence"].as_i64().unwrap_or(50) as i32;
    let _ = tx
        .send(alog(
            &agents.reaper,
            &format!(
                "Patch forged: {} (confidence: {}/100)",
                result["explanation"].as_str().unwrap_or(""),
                confidence
            ),
            "success",
        ))
        .await;
    let _ = tx
        .send(sse_ev(
            "issue_confidence",
            json!({"id":issue["id"],"confidence":confidence}),
        ))
        .await;

    let mut result = match apply_patch_with_self_heal(
        &http,
        &tx,
        &issue,
        &scope,
        &agents.reaper,
        &code_selection.codebase,
        &enriched_issue_ctx,
        result,
        &mut cost,
    )
    .await
    {
        Ok(result) => result,
        Err(reason) => {
            finish_skipped_attempt(
                &tx,
                &issue,
                &attempt_id,
                &reason,
                cost,
                None,
                0,
                &t_start,
                &scope.work_path,
            )
            .await;
            return;
        }
    };
    let _ = tx.send(astatus(&agents.reaper.id, "idle", "")).await;

    let mut smith_review = SmithReviewOutcome {
        final_patch: result["patch"].as_str().unwrap_or("").to_string(),
        smith_note: String::new(),
    };

    if let Some(ref smith) = agents.smith {
        if cancelled(&params) {
            finish_skipped_attempt(
                &tx,
                &issue,
                &attempt_id,
                "cancelled",
                cost,
                Some(&smith_review.final_patch),
                confidence,
                &t_start,
                &scope.work_path,
            )
            .await;
            return;
        }
        let _ = tx
            .send(astatus(
                &smith.id,
                "working",
                &format!("Smithing #{}", scope.issue_num),
            ))
            .await;
        match agent_smith_patch(
            &http,
            issue["title"].as_str().unwrap_or(""),
            &smith_review.final_patch,
            result["explanation"].as_str().unwrap_or(""),
            smith,
        )
        .await
        {
            Ok((rev, rc)) => {
                cost += rc;
                let sconf = rev["confidence"].as_i64().unwrap_or(50) as i32;
                let approved = rev["approved"].as_bool().unwrap_or(true);
                let feedback = rev["feedback"].as_str().unwrap_or("").to_string();

                let _ = tx
                    .send(alog(
                        smith,
                        &format!("{sconf}% — {feedback}"),
                        if approved { "success" } else { "warn" },
                    ))
                    .await;

                if let Some(improved_patch) = rev["improved_patch"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                {
                    smith_review.final_patch = improved_patch.to_string();
                }
                smith_review.smith_note =
                    format!("\n\n### Smith Review\n{feedback} (confidence: {sconf}%)");

                if !approved && sconf < params.min_conf {
                    let _ = tx
                        .send(alog(
                            smith,
                            &format!("Confidence {sconf}% < {}% — rejected", params.min_conf),
                            "warn",
                        ))
                        .await;
                    let _ = save_rejected_patch(
                        &Uuid::new_v4().to_string()[..12],
                        &params.run_id,
                        &scope.repo,
                        scope.issue_num,
                        issue["title"].as_str().unwrap_or(""),
                        &format!("confidence_{sconf}"),
                        &feedback,
                        sconf,
                        &smith_review.final_patch,
                    );
                    let _ = tx
                        .send(sse_ev(
                            "issue_result",
                            json!({
                                "id": issue["id"],
                                "status": "rejected",
                                "reason": format!("confidence_{sconf}"),
                                "feedback": feedback,
                                "confidence": sconf,
                            }),
                        ))
                        .await;
                    let _ = finish_attempt(
                        &attempt_id,
                        "skipped",
                        None,
                        None,
                        cost,
                        Some(&smith_review.final_patch),
                        None,
                        Some(&format!("confidence_{sconf}")),
                        Some(t_start.elapsed().as_secs_f64()),
                        sconf,
                    );
                    let _ = tx.send(astatus(&smith.id, "idle", "")).await;
                    cleanup_work_path(&scope.work_path).await;
                    return;
                }
            }
            Err(e) => {
                let _ = tx
                    .send(alog(
                        smith,
                        &format!("Smith error (continuing): {e}"),
                        "warn",
                    ))
                    .await;
            }
        }
        let _ = tx.send(astatus(&smith.id, "idle", "")).await;
    }

    if cancelled(&params) {
        finish_skipped_attempt(
            &tx,
            &issue,
            &attempt_id,
            "cancelled",
            cost,
            Some(&smith_review.final_patch),
            confidence,
            &t_start,
            &scope.work_path,
        )
        .await;
        return;
    }

    let _ = tx
        .send(astatus(
            &agents.gatekeeper.id,
            "working",
            &format!("Testing #{}", scope.issue_num),
        ))
        .await;
    let mut test = run_tests(&scope.work_path).await;
    let _ = tx
        .send(alog(
            &agents.gatekeeper,
            &format!("Tests {}", if test.passed { "passed ✓" } else { "failed" }),
            if test.passed { "success" } else { "warn" },
        ))
        .await;

    for retry in 0..params.retry_count {
        if test.passed {
            break;
        }
        let _ = tx
            .send(alog(
                &agents.reaper,
                &format!(
                    "Test failure → retry {} of {}",
                    retry + 1,
                    params.retry_count
                ),
                "warn",
            ))
            .await;
        let _ = tx
            .send(astatus(
                &agents.reaper.id,
                "working",
                &format!("Retry #{}", scope.issue_num),
            ))
            .await;
        let _ = git_reset(&scope.work_path).await;
        match agent_patch_retry(
            &http,
            issue["title"].as_str().unwrap_or(""),
            issue["body"].as_str().unwrap_or(""),
            &code_selection.codebase,
            &smith_review.final_patch,
            &format!("Test failure:\n{}\n\n{}", test.output, enriched_issue_ctx),
            &agents.reaper,
        )
        .await
        {
            Ok((retry_result, retry_cost)) => {
                cost += retry_cost;
                if !retry_result["patch"].is_null() {
                    let retry_patch = retry_result["patch"].as_str().unwrap_or("").to_string();
                    let (applied, _) = apply_patch(&scope.work_path, &retry_patch).await;
                    if applied {
                        smith_review.final_patch = retry_patch;
                        result = retry_result;
                        test = run_tests(&scope.work_path).await;
                        let _ = tx
                            .send(alog(
                                &agents.reaper,
                                &format!(
                                    "Retry {}: {}",
                                    retry + 1,
                                    if test.passed {
                                        "passed ✓"
                                    } else {
                                        "still failing"
                                    }
                                ),
                                if test.passed { "success" } else { "warn" },
                            ))
                            .await;
                    }
                }
            }
            Err(e) => {
                let _ = tx
                    .send(alog(&agents.reaper, &format!("Retry error: {e}"), "warn"))
                    .await;
            }
        }
        let _ = tx.send(astatus(&agents.reaper.id, "idle", "")).await;
    }

    if cancelled(&params) {
        finish_skipped_attempt(
            &tx,
            &issue,
            &attempt_id,
            "cancelled",
            cost,
            Some(&smith_review.final_patch),
            confidence,
            &t_start,
            &scope.work_path,
        )
        .await;
        return;
    }

    let _ = tx
        .send(astatus(
            &agents.gatekeeper.id,
            "working",
            &format!("PR #{}", scope.issue_num),
        ))
        .await;
    let (pr, pr_number) = match publish_pull_request(
        &http,
        &issue,
        &scope,
        &agents,
        &bot_token,
        &bot_user,
        &result,
        &smith_review.smith_note,
        confidence,
        &test,
    )
    .await
    {
        Ok(outcome) => outcome,
        Err(e) => {
            finish_error_attempt(
                &tx,
                &issue,
                &attempt_id,
                &e.to_string(),
                cost,
                confidence,
                &t_start,
                &scope.work_path,
            )
            .await;
            return;
        }
    };

    let _ = track_pr(pr_number, &scope.repo, &params.run_id);
    let duration = t_start.elapsed().as_secs_f64();
    let _ = finish_attempt(
        &attempt_id,
        "fixed",
        pr["html_url"].as_str(),
        Some(pr_number),
        cost,
        Some(&smith_review.final_patch),
        None,
        None,
        Some(duration),
        confidence,
    );
    let _ = update_perf(
        &agents.reaper.name,
        &agents.reaper.provider,
        &agents.reaper.model,
        "reaper",
        "fixed",
        cost,
    );

    run_cost.fetch_add((cost * 1_000_000.0) as i64, Ordering::Relaxed);

    let _ = tx
        .send(alog(
            &agents.gatekeeper,
            &format!(
                "Kill confirmed — PR #{pr_number} → {}",
                pr["html_url"].as_str().unwrap_or("")
            ),
            "success",
        ))
        .await;
    let _ = tx
        .send(sse_ev(
            "issue_result",
            json!({
                "id": issue["id"],
                "status": "fixed",
                "pr": {
                    "number": pr_number,
                    "url": pr["html_url"],
                    "draft": !test.passed,
                    "repo": scope.repo,
                    "title": issue["title"],
                    "fix": result["explanation"],
                    "diff": smith_review.final_patch,
                    "confidence": confidence,
                    "team": {
                        "judge": agents.judge.as_ref().map(|judge| judge.name.as_str()),
                        "reaper": agents.reaper.name.as_str(),
                        "smith": agents.smith.as_ref().map(|smith| smith.name.as_str()),
                        "gatekeeper": agents.gatekeeper.name.as_str(),
                    }
                }
            }),
        ))
        .await;
    let _ = tx.send(astatus(&agents.gatekeeper.id, "idle", "")).await;

    cleanup_work_path(&scope.work_path).await;
}

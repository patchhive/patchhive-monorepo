use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicI64, Ordering}};
use tokio::sync::mpsc::Sender;
use std::convert::Infallible;
use axum::response::sse::Event;
use chrono::Utc;
use uuid::Uuid;

use crate::agents::*;
use crate::db::*;
use crate::github::*;
use crate::git_ops::*;
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

fn ts() -> String { Utc::now().format("%H:%M:%S").to_string() }

pub fn alog(agent: &AgentConfig, msg: &str, kind: &str) -> Result<Event, Infallible> {
    sse_ev("agent_log", json!({
        "agent_id": agent.id, "agent": agent.name, "role": agent.role,
        "msg": msg, "type": kind, "ts": ts()
    }))
}

pub fn alog_raw(agent_id: &str, agent_name: &str, role: &str, msg: &str, kind: &str) -> Result<Event, Infallible> {
    sse_ev("agent_log", json!({
        "agent_id": agent_id, "agent": agent_name, "role": role,
        "msg": msg, "type": kind, "ts": ts()
    }))
}

pub fn astatus(agent_id: &str, status: &str, task: &str) -> Result<Event, Infallible> {
    sse_ev("agent_status", json!({"agent_id": agent_id, "status": status, "task": task}))
}

fn cfg(k: &str) -> String { std::env::var(k).unwrap_or_default() }

pub struct FixParams {
    pub retry_count: usize,
    pub min_conf: i32,
    pub budget: f64,
    pub run_id: String,
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

    // Pick agents by index
    let judge_idx      = idx % judges.len().max(1);
    let reaper_idx     = idx % reapers.len().max(1);
    let smith_idx      = idx % smiths.len().max(1);
    let gatekeeper_idx = idx % gatekeepers.len().max(1);

    let judge      = judges.get(judge_idx).cloned();
    let reaper     = reapers.get(reaper_idx).cloned().unwrap_or_else(|| reapers[0].clone());
    let smith      = smiths.get(smith_idx).cloned();
    let gatekeeper = gatekeepers.get(gatekeeper_idx).cloned().unwrap_or_else(|| reaper.clone());

    let repo_name  = issue["repo"].as_str().unwrap_or("").split('/').nth(1).unwrap_or("repo").to_string();
    let issue_num  = issue["number"].as_i64().unwrap_or(0);
    let branch     = format!("reaper/issue-{issue_num}");
    let work_path  = work_dir().join(format!("{repo_name}-{issue_num}"));
    let attempt_id = Uuid::new_v4().to_string()[..12].to_string();
    let t_start    = std::time::Instant::now();
    let mut cost   = 0.0f64;

    let bot_token = reaper.bot_token.clone().unwrap_or_else(|| cfg("BOT_GITHUB_TOKEN"));
    let bot_user  = reaper.bot_user.clone().unwrap_or_else(|| cfg("BOT_GITHUB_USER"));

    let send = |ev: Result<Event, Infallible>| {
        let tx2 = tx.clone();
        async move { let _ = tx2.send(ev).await; }
    };

    // Duplicate check
    if gh_check_duplicate(&http, issue["repo"].as_str().unwrap_or(""), &branch,
        Some(&bot_user), Some(&bot_token)).await {
        let _ = tx.send(alog(&reaper, &format!("[#{issue_num}] Branch/PR exists — skipping"), "warn")).await;
        let _ = tx.send(sse_ev("issue_result", json!({"id":issue["id"],"status":"skipped","reason":"duplicate"}))).await;
        return;
    }

    let _ = tx.send(sse_ev("issue_assign", json!({
        "id": issue["id"], "score": issue["fixability_score"],
        "reaper": reaper.id,
        "judge": judge.as_ref().map(|j| j.id.as_str()),
        "smith": smith.as_ref().map(|s| s.id.as_str()),
        "gatekeeper": gatekeeper.id,
    }))).await;

    let _ = start_attempt(
        &attempt_id, &params.run_id, &issue,
        &reaper.name,
        smith.as_ref().map(|s| s.name.as_str()),
        &gatekeeper.name,
    );

    // Fork + clone
    let _ = tx.send(astatus(&reaper.id, "working", &format!("Forking {}", issue["repo"].as_str().unwrap_or("")))).await;
    let _ = tx.send(alog(&reaper, &format!("[#{issue_num}] Forking {}…", issue["repo"].as_str().unwrap_or("")), "info")).await;

    let fork = match gh_fork(&http, issue["repo"].as_str().unwrap_or(""), Some(&bot_token), Some(&bot_user)).await {
        Ok(f) => f,
        Err(e) => {
            let _ = finish_attempt(&attempt_id, "error", None, None, cost, None, Some(&e.to_string()), None, Some(t_start.elapsed().as_secs_f64()), 0);
            let _ = tx.send(sse_ev("issue_result", json!({"id":issue["id"],"status":"error"}))).await;
            if work_path.exists() { let _ = tokio::fs::remove_dir_all(&work_path).await; }
            return;
        }
    };

    if work_path.exists() { let _ = tokio::fs::remove_dir_all(&work_path).await; }

    if let Err(e) = git_clone(fork["clone_url"].as_str().unwrap_or(""), &work_path, Some(&bot_user), Some(&bot_token)).await {
        let _ = finish_attempt(&attempt_id, "error", None, None, cost, None, Some(&e.to_string()), None, Some(t_start.elapsed().as_secs_f64()), 0);
        let _ = tx.send(sse_ev("issue_result", json!({"id":issue["id"],"status":"error"}))).await;
        return;
    }
    if let Err(e) = git_branch(&work_path, &branch).await {
        let _ = finish_attempt(&attempt_id, "error", None, None, cost, None, Some(&e.to_string()), None, Some(t_start.elapsed().as_secs_f64()), 0);
        let _ = tx.send(sse_ev("issue_result", json!({"id":issue["id"],"status":"error"}))).await;
        return;
    }

    let issue_ctx = gh_get_issue_context(&http, issue["repo"].as_str().unwrap_or(""), issue_num, Some(&bot_token)).await;

    // Working-on-it comment
    gh_comment_issue(&http, issue["repo"].as_str().unwrap_or(""), issue_num,
        &format!("🔱 **RepoReaper** is hunting this bug.\n\n> Fixability score: **{}/100**{}\n\nA pull request will be opened shortly.\n\n*by PatchHive*",
            issue["fixability_score"].as_i64().unwrap_or(50),
            issue["fixability_reason"].as_str().filter(|s| !s.is_empty()).map(|r| format!(" — {r}")).unwrap_or_default()
        ), Some(&bot_token)).await;

    // Judge: select relevant files
    let codebase = if let Some(ref judge) = judge {
        let _ = tx.send(astatus(&judge.id, "working", &format!("#{issue_num}"))).await;
        let structure = collect_repo_structure(&work_path);
        let result = agent_select_files(
            &http, &structure,
            issue["title"].as_str().unwrap_or(""),
            issue["body"].as_str().unwrap_or(""),
            judge,
        ).await;
        let _ = tx.send(astatus(&judge.id, "idle", "")).await;
        match result {
            Ok((files, c)) if !files.is_empty() => {
                cost += c;
                let _ = tx.send(alog(judge, &format!("Targeted {} files: {}", files.len(), files.iter().take(3).cloned().collect::<Vec<_>>().join(", ")), "success")).await;
                collect_files_selective(&work_path, &files, 80_000)
            }
            Ok(_) => collect_files_all(&work_path, 60_000),
            Err(e) => {
                let _ = tx.send(alog(judge, &format!("Targeting failed (fallback): {e}"), "warn")).await;
                collect_files_all(&work_path, 60_000)
            }
        }
    } else {
        collect_files_all(&work_path, 60_000)
    };

    // Reaper: generate patch
    let _ = tx.send(astatus(&reaper.id, "working", &format!("Reaping #{issue_num}"))).await;
    let patch_result = agent_generate_patch(
        &http,
        issue["title"].as_str().unwrap_or(""),
        issue["body"].as_str().unwrap_or(""),
        &codebase, &issue_ctx, &reaper,
    ).await;

    let (mut result, pc) = match patch_result {
        Ok(v) => v,
        Err(e) => {
            let _ = tx.send(alog(&reaper, &format!("Patch generation error: {e}"), "error")).await;
            let _ = finish_attempt(&attempt_id, "skipped", None, None, cost, None, None, Some("patch_error"), Some(t_start.elapsed().as_secs_f64()), 0);
            let _ = tx.send(sse_ev("issue_result", json!({"id":issue["id"],"status":"skipped","reason":"patch_error"}))).await;
            if work_path.exists() { let _ = tokio::fs::remove_dir_all(&work_path).await; }
            return;
        }
    };
    cost += pc;

    if result["patch"].is_null() || result["patch"].as_str().map(|s| s.trim().is_empty()).unwrap_or(true) {
        let _ = tx.send(alog(&reaper, &format!("No patch: {}", result["explanation"].as_str().unwrap_or("")), "warn")).await;
        let _ = finish_attempt(&attempt_id, "skipped", None, None, cost, None, None, Some("no_patch"), Some(t_start.elapsed().as_secs_f64()), 0);
        let _ = tx.send(sse_ev("issue_result", json!({"id":issue["id"],"status":"skipped","reason":"no_patch"}))).await;
        if work_path.exists() { let _ = tokio::fs::remove_dir_all(&work_path).await; }
        return;
    }

    let confidence = result["confidence"].as_i64().unwrap_or(50) as i32;
    let _ = tx.send(alog(&reaper, &format!("Patch forged: {} (confidence: {}/100)", result["explanation"].as_str().unwrap_or(""), confidence), "success")).await;
    let _ = tx.send(sse_ev("issue_confidence", json!({"id":issue["id"],"confidence":confidence}))).await;

    // Apply patch with self-healing
    let patch_str = result["patch"].as_str().unwrap_or("").to_string();
    let (mut applied, mut apply_err) = apply_patch(&work_path, &patch_str).await;

    if !applied {
        let _ = tx.send(alog(&reaper, "Apply failed — self-healing…", "warn")).await;
        if let Ok((r2, c2)) = agent_patch_retry(
            &http, issue["title"].as_str().unwrap_or(""), issue["body"].as_str().unwrap_or(""),
            &codebase, &patch_str, &format!("git apply error:\n{apply_err}"), &reaper,
        ).await {
            cost += c2;
            if !r2["patch"].is_null() {
                let (ok, err) = apply_patch(&work_path, r2["patch"].as_str().unwrap_or("")).await;
                if ok {
                    result = r2;
                    applied = true;
                    let _ = tx.send(alog(&reaper, "Self-healed ✓", "success")).await;
                } else {
                    apply_err = err;
                }
            }
        }
        if !applied {
            let _ = tx.send(alog(&reaper, "Cannot apply patch — skipping", "error")).await;
            let _ = finish_attempt(&attempt_id, "skipped", None, None, cost, None, None, Some("apply_failed"), Some(t_start.elapsed().as_secs_f64()), 0);
            let _ = tx.send(sse_ev("issue_result", json!({"id":issue["id"],"status":"skipped","reason":"apply_failed"}))).await;
            if work_path.exists() { let _ = tokio::fs::remove_dir_all(&work_path).await; }
            return;
        }
    }
    let _ = tx.send(astatus(&reaper.id, "idle", "")).await;

    // Smith: review patch
    let mut final_patch = result["patch"].as_str().unwrap_or("").to_string();
    let mut smith_note = String::new();

    if let Some(ref smith) = smith {
        let _ = tx.send(astatus(&smith.id, "working", &format!("Smithing #{issue_num}"))).await;
        match agent_smith_patch(&http, issue["title"].as_str().unwrap_or(""), &final_patch, result["explanation"].as_str().unwrap_or(""), smith).await {
            Ok((rev, rc)) => {
                cost += rc;
                let sconf = rev["confidence"].as_i64().unwrap_or(50) as i32;
                let approved = rev["approved"].as_bool().unwrap_or(true);
                let feedback = rev["feedback"].as_str().unwrap_or("").to_string();

                let _ = tx.send(alog(smith, &format!("{sconf}% — {feedback}"), if approved { "success" } else { "warn" })).await;

                if let Some(imp) = rev["improved_patch"].as_str().filter(|s| !s.is_empty()) {
                    final_patch = imp.to_string();
                }
                smith_note = format!("\n\n### Smith Review\n{feedback} (confidence: {sconf}%)");

                if !approved && sconf < params.min_conf {
                    let _ = tx.send(alog(smith, &format!("Confidence {sconf}% < {}% — rejected", params.min_conf), "warn")).await;
                    let _ = save_rejected_patch(
                        &Uuid::new_v4().to_string()[..12], &params.run_id,
                        issue["repo"].as_str().unwrap_or(""), issue_num,
                        issue["title"].as_str().unwrap_or(""),
                        &format!("confidence_{sconf}"), &feedback, sconf, &final_patch,
                    );
                    let _ = tx.send(sse_ev("issue_result", json!({
                        "id": issue["id"], "status": "rejected",
                        "reason": format!("confidence_{sconf}"),
                        "feedback": feedback, "confidence": sconf,
                    }))).await;
                    let _ = finish_attempt(&attempt_id, "skipped", None, None, cost, Some(&final_patch), None, Some(&format!("confidence_{sconf}")), Some(t_start.elapsed().as_secs_f64()), sconf);
                    let _ = tx.send(astatus(&smith.id, "idle", "")).await;
                    if work_path.exists() { let _ = tokio::fs::remove_dir_all(&work_path).await; }
                    return;
                }
            }
            Err(e) => { let _ = tx.send(alog(smith, &format!("Smith error (continuing): {e}"), "warn")).await; }
        }
        let _ = tx.send(astatus(&smith.id, "idle", "")).await;
    }

    // Gatekeeper: run tests with configurable retries
    let _ = tx.send(astatus(&gatekeeper.id, "working", &format!("Testing #{issue_num}"))).await;
    let mut test = run_tests(&work_path).await;
    let _ = tx.send(alog(&gatekeeper, &format!("Tests {}", if test.passed { "passed ✓" } else { "failed" }), if test.passed { "success" } else { "warn" })).await;

    for retry in 0..params.retry_count {
        if test.passed { break; }
        let _ = tx.send(alog(&reaper, &format!("Test failure → retry {} of {}", retry + 1, params.retry_count), "warn")).await;
        let _ = tx.send(astatus(&reaper.id, "working", &format!("Retry #{issue_num}"))).await;
        let _ = git_reset(&work_path).await;
        match agent_patch_retry(
            &http, issue["title"].as_str().unwrap_or(""), issue["body"].as_str().unwrap_or(""),
            &codebase, &final_patch, &format!("Test failure:\n{}", test.output), &reaper,
        ).await {
            Ok((r3, c3)) => {
                cost += c3;
                if !r3["patch"].is_null() {
                    let r3_patch = r3["patch"].as_str().unwrap_or("").to_string();
                    let (ok3, _) = apply_patch(&work_path, &r3_patch).await;
                    if ok3 {
                        final_patch = r3_patch;
                        result = r3;
                        test = run_tests(&work_path).await;
                        let _ = tx.send(alog(&reaper, &format!("Retry {}: {}", retry + 1, if test.passed { "passed ✓" } else { "still failing" }), if test.passed { "success" } else { "warn" })).await;
                    }
                }
            }
            Err(e) => { let _ = tx.send(alog(&reaper, &format!("Retry error: {e}"), "warn")).await; }
        }
        let _ = tx.send(astatus(&reaper.id, "idle", "")).await;
    }

    // Commit + push + open PR
    let _ = tx.send(astatus(&gatekeeper.id, "working", &format!("PR #{issue_num}"))).await;
    let commit_msg = format!("fix: {} (closes #{issue_num})", issue["title"].as_str().unwrap_or("").chars().take(72).collect::<String>());

    if let Err(e) = git_commit_push(&work_path, &branch, &commit_msg).await {
        let _ = finish_attempt(&attempt_id, "error", None, None, cost, None, Some(&e.to_string()), None, Some(t_start.elapsed().as_secs_f64()), confidence);
        let _ = tx.send(sse_ev("issue_result", json!({"id":issue["id"],"status":"error"}))).await;
        if work_path.exists() { let _ = tokio::fs::remove_dir_all(&work_path).await; }
        return;
    }

    let files_changed: Vec<String> = result["files_changed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    let files_md = files_changed.iter().map(|f| format!("- `{f}`")).collect::<Vec<_>>().join("\n");

    let pr_body = format!(
        "## 🔱 Reaping #{issue_num}: {}\n\n\
        ### What changed\n{}\n\n\
        **Reaper confidence:** {confidence}/100\n\n\
        ### Files targeted\n{files_md}\n\n\
        ### Fixability Score\n**{}/100** — {}\n\n\
        {smith_note}\n\n\
        ### Tests\n{}\n\n\
        ---\n\
        ⚖ Judge: {} · ⚔ Reaper: {} · ⬢ Smith: {} · 🔒 Gatekeeper: {}\n\n\
        *RepoReaper by PatchHive · Closes #{issue_num}*",
        issue["title"].as_str().unwrap_or(""),
        result["explanation"].as_str().unwrap_or(""),
        issue["fixability_score"].as_i64().unwrap_or(50),
        issue["fixability_reason"].as_str().unwrap_or(""),
        if test.passed { "✅ Passed" } else { "⚠️ Failed (draft PR)" },
        judge.as_ref().map(|j| j.name.as_str()).unwrap_or("none"),
        reaper.name,
        smith.as_ref().map(|s| s.name.as_str()).unwrap_or("none"),
        gatekeeper.name,
    );

    let base_branch = gh_default_branch(&http, issue["repo"].as_str().unwrap_or(""), Some(&bot_token))
        .await
        .unwrap_or_else(|| "main".to_string());

    let pr = match gh_post(&http, &format!("/repos/{}/pulls", issue["repo"].as_str().unwrap_or("")), &json!({
        "title": commit_msg, "body": pr_body,
        "head": format!("{bot_user}:{branch}"),
        "base": base_branch, "draft": !test.passed,
    }), Some(&bot_token)).await {
        Ok(p) => p,
        Err(e) => {
            let _ = finish_attempt(&attempt_id, "error", None, None, cost, None, Some(&e.to_string()), None, Some(t_start.elapsed().as_secs_f64()), confidence);
            let _ = tx.send(sse_ev("issue_result", json!({"id":issue["id"],"status":"error"}))).await;
            if work_path.exists() { let _ = tokio::fs::remove_dir_all(&work_path).await; }
            return;
        }
    };

    let pr_number = pr["number"].as_i64().unwrap_or(0);
    let _ = track_pr(pr_number, issue["repo"].as_str().unwrap_or(""), &params.run_id);
    let duration = t_start.elapsed().as_secs_f64();
    let _ = finish_attempt(&attempt_id, "fixed", pr["html_url"].as_str(), Some(pr_number), cost, Some(&final_patch), None, None, Some(duration), confidence);
    let _ = update_perf(&reaper.name, &reaper.provider, &reaper.model, "reaper", "fixed", cost);

    run_cost.fetch_add((cost * 1_000_000.0) as i64, Ordering::Relaxed);

    let _ = tx.send(alog(&gatekeeper, &format!("Kill confirmed — PR #{pr_number} → {}", pr["html_url"].as_str().unwrap_or("")), "success")).await;
    let _ = tx.send(sse_ev("issue_result", json!({
        "id": issue["id"], "status": "fixed",
        "pr": {
            "number": pr_number, "url": pr["html_url"],
            "draft": !test.passed, "repo": issue["repo"],
            "title": issue["title"],
            "fix": result["explanation"],
            "diff": final_patch,
            "confidence": confidence,
            "team": {
                "judge": judge.as_ref().map(|j| j.name.as_str()),
                "reaper": reaper.name.as_str(),
                "smith": smith.as_ref().map(|s| s.name.as_str()),
                "gatekeeper": gatekeeper.name.as_str(),
            }
        }
    }))).await;
    let _ = tx.send(astatus(&gatekeeper.id, "idle", "")).await;

    if work_path.exists() { let _ = tokio::fs::remove_dir_all(&work_path).await; }
}

// orchestrate.rs — Main fix_one orchestrator

use patchhive_github_pr::github_token_from_env;
use serde_json::json;
use uuid::Uuid;

use crate::agents::*;
use crate::db::*;
use crate::github::*;
use crate::state::AgentConfig;

use super::context::{
    clone_issue_repo, load_enriched_issue_context, select_code_context,
};
use super::memory::submit_smith_rejection_candidate;
use super::patch::{apply_patch_with_self_heal, publish_pull_request};
use super::sse::{alog, astatus, sse_ev};
use super::types::{
    build_issue_scope, cancelled, cleanup_work_path, cfg, finish_error_attempt,
    finish_skipped_attempt, pick_fix_agents, FixParams, SmithReviewOutcome,
    Tx,
};

pub async fn fix_one(
    issue: serde_json::Value,
    idx: usize,
    judges: Vec<AgentConfig>,
    reapers: Vec<AgentConfig>,
    smiths: Vec<AgentConfig>,
    gatekeepers: Vec<AgentConfig>,
    sem: std::sync::Arc<tokio::sync::Semaphore>,
    params: FixParams,
    run_cost: std::sync::Arc<std::sync::atomic::AtomicI64>,
    tx: Tx,
    http: reqwest::Client,
) {
    let Ok(_permit) = sem.acquire().await else {
        tracing::warn!("RepoReaper fix worker semaphore closed before issue execution");
        return;
    };
    if cancelled(&params) {
        return;
    }

    let agents = match pick_fix_agents(idx, &judges, &reapers, &smiths, &gatekeepers) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Cannot pick fix agents: {e:#}");
            return;
        }
    };
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
            &tx, &issue, &attempt_id, "cancelled", cost, None, 0, &t_start, &scope.work_path,
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
            &tx, &issue, &attempt_id, "cancelled", cost, None, 0, &t_start, &scope.work_path,
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
                &tx, &issue, &attempt_id, "patch_error", cost, None, 0, &t_start, &scope.work_path,
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
            &tx, &issue, &attempt_id, "no_patch", cost, None, 0, &t_start, &scope.work_path,
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
                &tx, &issue, &attempt_id, &reason, cost, None, 0, &t_start, &scope.work_path,
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
                &tx, &issue, &attempt_id, "cancelled", cost, Some(&smith_review.final_patch), confidence, &t_start, &scope.work_path,
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
                    match submit_smith_rejection_candidate(
                        &http,
                        &issue,
                        &scope,
                        &code_selection.selected_files,
                        &smith_review.final_patch,
                        &feedback,
                        sconf,
                        params.min_conf,
                        &params.run_id,
                    )
                    .await
                    {
                        Ok(Some(_)) => {
                            let _ = tx
                                .send(alog(
                                    smith,
                                    "Queued FailGuard lesson candidate from Smith rejection",
                                    "info",
                                ))
                                .await;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            let _ = tx
                                .send(alog(
                                    smith,
                                    &format!("FailGuard candidate submission skipped: {e}"),
                                    "warn",
                                ))
                                .await;
                        }
                    }
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
            &tx, &issue, &attempt_id, "cancelled", cost, Some(&smith_review.final_patch), confidence, &t_start, &scope.work_path,
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
    let mut test = crate::git_ops::run_tests(&scope.work_path).await;
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
        let _ = crate::git_ops::git_reset(&scope.work_path).await;
        match crate::agents::agent_patch_retry(
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
                    let (applied, _) = crate::git_ops::apply_patch(&scope.work_path, &retry_patch).await;
                    if applied {
                        smith_review.final_patch = retry_patch;
                        result = retry_result;
                        test = crate::git_ops::run_tests(&scope.work_path).await;
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
            &tx, &issue, &attempt_id, "cancelled", cost, Some(&smith_review.final_patch), confidence, &t_start, &scope.work_path,
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
                &tx, &issue, &attempt_id, &e.to_string(), cost, confidence, &t_start, &scope.work_path,
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

    run_cost.fetch_add((cost * 1_000_000.0) as i64, std::sync::atomic::Ordering::Relaxed);

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

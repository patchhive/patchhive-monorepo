// orchestrate.rs — Main fix_one orchestrator

use patchhive_github_pr::github_token_from_env;
use patchhive_product_core::hivecore_policy::{
    check_repository_policy, commit_pr_slot, release_pr_slot, reserve_pr_slot,
    PrReservationRequest, RepositoryPolicyDecisionRequest,
};
use serde_json::json;
use uuid::Uuid;

use crate::agents::{agent_generate_patch, agent_patch_retry, agent_smith_patch};
use crate::db::{
    finish_attempt, record_run_artifact, save_rejected_patch, start_attempt, track_pr, update_perf,
    IssueAttemptFinish, IssueAttemptStart, IssueAttemptStatus, RejectedPatchRecord,
    RunArtifactInput,
};
use crate::github::{
    gh_check_duplicate, gh_find_open_linked_pr, gh_upsert_repo_reaper_issue_comment, OpenLinkedPr,
};

use super::context::{clone_issue_repo, load_enriched_issue_context, select_code_context};
use super::memory::{submit_smith_rejection_candidate, SmithRejectionCandidateInput};
use super::patch::{
    apply_patch_with_self_heal, publish_pull_request, PatchSelfHealInput, PullRequestPublishInput,
};
use super::sse::{alog, astatus, sse_ev};
use super::types::{
    build_attempt_target, build_issue_scope, cancelled, cfg, cleanup_work_path, pick_fix_agents,
    AttemptFinisher, FixIssueJob, IssueScope, SmithReviewOutcome,
};

fn artifact(
    run_id: &str,
    attempt_id: &str,
    phase: &str,
    kind: &str,
    status: &str,
    message: &str,
    metadata: serde_json::Value,
) {
    if let Err(error) = record_run_artifact(RunArtifactInput {
        run_id,
        attempt_id: Some(attempt_id),
        phase,
        kind,
        status,
        message,
        metadata: Some(&metadata),
    }) {
        tracing::warn!(
            run_id,
            attempt_id,
            kind,
            "could not persist run artifact: {error:#}"
        );
    }
}

fn compact_no_change_detail(test: &crate::git_ops::TestResult) -> String {
    if test.passed() {
        return "No commit-ready diff remained after patch review and validation.".to_string();
    }

    let output = test.output.trim();
    if output.is_empty() {
        return format!(
            "No commit-ready diff remained after validation/retry. Last test runner `{}` failed without output.",
            test.runner
        );
    }

    let excerpt = output.chars().take(500).collect::<String>();
    let suffix = if output.chars().count() > 500 {
        "..."
    } else {
        ""
    };
    format!(
        "No commit-ready diff remained after validation/retry. Last test runner `{}` reported: {excerpt}{suffix}",
        test.runner
    )
}

fn should_retry_test_failure(test: &crate::git_ops::TestResult) -> bool {
    test.status.should_retry()
}

fn test_log_label(test: &crate::git_ops::TestResult) -> &'static str {
    match test.status {
        patchhive_product_core::validation::TestExecutionStatus::Passed => "passed",
        patchhive_product_core::validation::TestExecutionStatus::Failed => "failed",
        patchhive_product_core::validation::TestExecutionStatus::Disabled => "disabled",
        patchhive_product_core::validation::TestExecutionStatus::Skipped => "skipped",
    }
}

fn truncate_public(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    let mut out = trimmed.chars().take(max_chars).collect::<String>();
    if trimmed.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn issue_fixability_line(issue: &serde_json::Value) -> String {
    let score = issue["fixability_score"].as_i64().unwrap_or(50);
    let reason = issue["fixability_reason"].as_str().unwrap_or("").trim();
    if reason.is_empty() {
        format!("**Fixability:** {score}/100")
    } else {
        format!(
            "**Fixability:** {score}/100 - {}",
            truncate_public(reason, 220)
        )
    }
}

fn public_hold_reason(reason: &str) -> String {
    if let Some(confidence) = reason.strip_prefix("confidence_") {
        return format!(
            "Smith review held the patch at {confidence}% confidence, below the configured release threshold."
        );
    }
    match reason {
        "cancelled" => "The run was cancelled before RepoReaper could finish this attempt.".to_string(),
        "duplicate" => "An existing PatchHive branch or pull request already appears to cover this issue.".to_string(),
        "existing_pr" => "An existing open pull request is already linked to this issue, so RepoReaper will not compete with it.".to_string(),
        "linked_pr_check_failed" => "RepoReaper could not verify whether this issue already has an open linked pull request, so it held the attempt instead of risking a duplicate PR.".to_string(),
        "no_changes" => "RepoReaper did not end with a commit-ready diff, so it did not open an empty pull request.".to_string(),
        "no_patch" => "RepoReaper could not produce a concrete patch for this issue from the available context.".to_string(),
        "patch_error" => "RepoReaper generated a candidate patch, but it could not apply cleanly enough to open a pull request.".to_string(),
        "suite_policy_blocked" => "HiveCore repository policy blocked automation for this repository.".to_string(),
        "suite_policy_unavailable" => "HiveCore policy was configured but unavailable, so RepoReaper failed closed before repository automation.".to_string(),
        "pr_budget_denied" => "HiveCore denied PR capacity because a product or suite budget is full.".to_string(),
        "pr_budget_unavailable" => "HiveCore PR budgeting was configured but unavailable, so RepoReaper did not open an unbudgeted pull request.".to_string(),
        other => format!("RepoReaper held this issue before PR delivery: {other}."),
    }
}

fn existing_pr_detail(pr: &OpenLinkedPr) -> String {
    format!(
        "{}. RepoReaper will not open another pull request for an issue that already has active PR coverage.",
        pr.detail()
    )
}

fn issue_comment_attempting(issue: &serde_json::Value, run_id: &str, attempt_id: &str) -> String {
    format!(
        "🔱 **RepoReaper by [PatchHive](https://github.com/patchhive)** is working on this issue.\n\n\
        **Status:** attempting fix\n\
        **Run:** `{run_id}`\n\
        **Attempt:** `{attempt_id}`\n\
        {}\n\n\
        RepoReaper will open a pull request for review if it produces a commit-ready diff. If it cannot, a separate outcome comment will explain why.",
        issue_fixability_line(issue),
    )
}

fn issue_comment_held(
    issue: &serde_json::Value,
    run_id: &str,
    attempt_id: &str,
    reason: &str,
) -> String {
    format!(
        "🔱 **RepoReaper by [PatchHive](https://github.com/patchhive)** checked this issue.\n\n\
        **Status:** held - no pull request opened\n\
        **Run:** `{run_id}`\n\
        **Attempt:** `{attempt_id}`\n\
        {}\n\
        **Reason:** {}",
        issue_fixability_line(issue),
        public_hold_reason(reason),
    )
}

fn issue_comment_error(issue: &serde_json::Value, run_id: &str, attempt_id: &str) -> String {
    format!(
        "🔱 **RepoReaper by [PatchHive](https://github.com/patchhive)** checked this issue.\n\n\
        **Status:** error - no pull request opened\n\
        **Run:** `{run_id}`\n\
        **Attempt:** `{attempt_id}`\n\
        {}\n\
        **Reason:** RepoReaper hit an execution error before PR delivery. The run was stopped instead of opening an unsafe or empty pull request.",
        issue_fixability_line(issue),
    )
}

fn issue_comment_fixed(
    issue: &serde_json::Value,
    run_id: &str,
    attempt_id: &str,
    pr_number: i64,
    pr_url: &str,
    test: &crate::git_ops::TestResult,
) -> String {
    let test_line = if test.passed() {
        "Validation tests passed."
    } else {
        "Validation tests were not proven safe/passing here, so the pull request requires review."
    };
    format!(
        "🔱 **RepoReaper by [PatchHive](https://github.com/patchhive)** produced a candidate fix for this issue.\n\n\
        **Status:** pull request opened\n\
        **Pull request:** [#{pr_number}]({pr_url})\n\
        **Run:** `{run_id}`\n\
        **Attempt:** `{attempt_id}`\n\
        {}\n\
        **Validation:** {test_line}",
        issue_fixability_line(issue),
    )
}

async fn update_issue_status_comment(
    http: &reqwest::Client,
    _issue: &serde_json::Value,
    scope: &IssueScope,
    bot_token: &str,
    body: String,
) {
    if bot_token.trim().is_empty() {
        tracing::warn!(
            "RepoReaper skipped managed issue comment for {}/{} because bot token is empty",
            scope.repo,
            scope.issue_num
        );
        return;
    }
    if let Err(error) = gh_upsert_repo_reaper_issue_comment(
        http,
        &scope.repo,
        scope.issue_num,
        &body,
        Some(bot_token),
    )
    .await
    {
        tracing::warn!(
            "RepoReaper managed issue comment update failed for {}/{}: {error:#}",
            scope.repo,
            scope.issue_num
        );
    }
}

pub async fn fix_one(job: FixIssueJob) {
    let FixIssueJob {
        issue,
        idx,
        context,
    } = job;
    let agents_pool = context.agents;
    let sem = context.sem;
    let process_sem = context.process_sem;
    let params = context.params;
    let run_cost = context.run_cost;
    let tx = context.tx;
    let http = context.http;

    let Ok(_permit) = sem.acquire().await else {
        tracing::warn!("RepoReaper fix worker semaphore closed before issue execution");
        return;
    };
    let _process_permit = match process_sem.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(tokio::sync::TryAcquireError::NoPermits) => {
            let _ = tx
                .send(sse_ev(
                    "capacity",
                    json!({"status":"waiting","message":"Process worker capacity is full; this issue is queued"}),
                ))
                .await;
            match process_sem.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    tracing::warn!(
                        "RepoReaper process worker semaphore closed before issue execution"
                    );
                    return;
                }
            }
        }
        Err(tokio::sync::TryAcquireError::Closed) => {
            tracing::warn!("RepoReaper process worker semaphore closed before issue execution");
            return;
        }
    };
    if cancelled(&params) {
        return;
    }

    let agents = match pick_fix_agents(
        idx,
        &agents_pool.judges,
        &agents_pool.reapers,
        &agents_pool.smiths,
        &agents_pool.gatekeepers,
    ) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Cannot pick fix agents: {e:#}");
            return;
        }
    };
    let scope = build_issue_scope(&issue);
    let attempt_target = build_attempt_target(&issue);
    let attempt_id = Uuid::new_v4().to_string()[..12].to_string();
    let t_start = std::time::Instant::now();
    let attempt_finisher = AttemptFinisher {
        tx: &tx,
        issue: &issue,
        attempt_id: &attempt_id,
        started_at: &t_start,
        work_path: &scope.work_path,
    };
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

    let repository_policy = match check_repository_policy(
        &http,
        &RepositoryPolicyDecisionRequest {
            repository: scope.repo.clone(),
            product: "repo-reaper".into(),
            operation: "repo_automation".into(),
        },
    )
    .await
    {
        Ok(Some(decision)) if !decision.allowed() => {
            let _ = start_attempt(IssueAttemptStart {
                attempt_id: &attempt_id,
                run_id: &params.run_id,
                target: &attempt_target,
                reaper_agent: &agents.reaper.name,
                smith_agent: agents.smith.as_ref().map(|smith| smith.name.as_str()),
                gatekeeper_agent: &agents.gatekeeper.name,
            });
            artifact(
                &params.run_id,
                &attempt_id,
                "policy",
                "hivecore.repository.blocked",
                "blocked",
                &decision.reason,
                json!({"repo": scope.repo, "policy": decision}),
            );
            attempt_finisher
                .skipped_with_error(
                    "suite_policy_blocked",
                    Some(&decision.reason),
                    cost,
                    None,
                    0,
                )
                .await;
            return;
        }
        Ok(Some(decision)) => Some(decision),
        Ok(None) => None,
        Err(error) => {
            let detail = format!("HiveCore repository policy check failed: {error}");
            let _ = start_attempt(IssueAttemptStart {
                attempt_id: &attempt_id,
                run_id: &params.run_id,
                target: &attempt_target,
                reaper_agent: &agents.reaper.name,
                smith_agent: agents.smith.as_ref().map(|smith| smith.name.as_str()),
                gatekeeper_agent: &agents.gatekeeper.name,
            });
            artifact(
                &params.run_id,
                &attempt_id,
                "policy",
                "hivecore.repository.unavailable",
                "blocked",
                &detail,
                json!({"repo": scope.repo}),
            );
            attempt_finisher
                .skipped_with_error("suite_policy_unavailable", Some(&detail), cost, None, 0)
                .await;
            return;
        }
    };
    let repository_trusted = repository_policy
        .as_ref()
        .map(|decision| decision.trusted)
        .unwrap_or(false);

    match gh_find_open_linked_pr(&http, &scope.repo, scope.issue_num, Some(&bot_token)).await {
        Ok(Some(pr)) => {
            let detail = existing_pr_detail(&pr);
            let _ = start_attempt(IssueAttemptStart {
                attempt_id: &attempt_id,
                run_id: &params.run_id,
                target: &attempt_target,
                reaper_agent: &agents.reaper.name,
                smith_agent: agents.smith.as_ref().map(|smith| smith.name.as_str()),
                gatekeeper_agent: &agents.gatekeeper.name,
            });
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_held(&issue, &params.run_id, &attempt_id, "existing_pr"),
            )
            .await;
            let _ = tx
                .send(alog(
                    &agents.reaper,
                    &format!("[#{}] Existing open linked PR — skipping", scope.issue_num),
                    "warn",
                ))
                .await;
            attempt_finisher
                .skipped_with_error("existing_pr", Some(&detail), cost, None, 0)
                .await;
            return;
        }
        Ok(None) => {}
        Err(error) => {
            let detail =
                format!("Could not verify linked PR state before attempting a fix: {error}");
            let _ = start_attempt(IssueAttemptStart {
                attempt_id: &attempt_id,
                run_id: &params.run_id,
                target: &attempt_target,
                reaper_agent: &agents.reaper.name,
                smith_agent: agents.smith.as_ref().map(|smith| smith.name.as_str()),
                gatekeeper_agent: &agents.gatekeeper.name,
            });
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_held(
                    &issue,
                    &params.run_id,
                    &attempt_id,
                    "linked_pr_check_failed",
                ),
            )
            .await;
            let _ = tx
                .send(alog(
                    &agents.reaper,
                    &format!("[#{}] Linked PR check failed — skipping", scope.issue_num),
                    "warn",
                ))
                .await;
            attempt_finisher
                .skipped_with_error("linked_pr_check_failed", Some(&detail), cost, None, 0)
                .await;
            return;
        }
    }

    if gh_check_duplicate(
        &http,
        &scope.repo,
        &scope.branch,
        Some(&bot_user),
        Some(&bot_token),
    )
    .await
    {
        update_issue_status_comment(
            &http,
            &issue,
            &scope,
            &bot_token,
            issue_comment_held(&issue, &params.run_id, &attempt_id, "duplicate"),
        )
        .await;
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

    let _ = start_attempt(IssueAttemptStart {
        attempt_id: &attempt_id,
        run_id: &params.run_id,
        target: &attempt_target,
        reaper_agent: &agents.reaper.name,
        smith_agent: agents.smith.as_ref().map(|smith| smith.name.as_str()),
        gatekeeper_agent: &agents.gatekeeper.name,
    });
    if let Some(decision) = &repository_policy {
        artifact(
            &params.run_id,
            &attempt_id,
            "policy",
            "hivecore.repository.allowed",
            "succeeded",
            &decision.reason,
            json!({"repo": scope.repo, "policy": decision}),
        );
    }
    update_issue_status_comment(
        &http,
        &issue,
        &scope,
        &bot_token,
        issue_comment_attempting(&issue, &params.run_id, &attempt_id),
    )
    .await;

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
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_error(&issue, &params.run_id, &attempt_id),
            )
            .await;
            attempt_finisher.error(&e.to_string(), cost, 0).await;
            return;
        }
    };

    if cancelled(&params) {
        update_issue_status_comment(
            &http,
            &issue,
            &scope,
            &bot_token,
            issue_comment_held(&issue, &params.run_id, &attempt_id, "cancelled"),
        )
        .await;
        attempt_finisher.skipped("cancelled", cost, None, 0).await;
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
        update_issue_status_comment(
            &http,
            &issue,
            &scope,
            &bot_token,
            issue_comment_held(&issue, &params.run_id, &attempt_id, "cancelled"),
        )
        .await;
        attempt_finisher.skipped("cancelled", cost, None, 0).await;
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
            let error = e.to_string();
            artifact(
                &params.run_id,
                &attempt_id,
                "patch",
                "agent.patch.failed",
                "failed",
                &error,
                json!({"agent": agents.reaper.name}),
            );
            let _ = tx
                .send(alog(
                    &agents.reaper,
                    &format!("Patch generation error: {error}"),
                    "error",
                ))
                .await;
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_held(&issue, &params.run_id, &attempt_id, "patch_error"),
            )
            .await;
            attempt_finisher
                .skipped_with_error("patch_error", Some(&error), cost, None, 0)
                .await;
            return;
        }
    };
    cost += patch_cost;
    artifact(
        &params.run_id,
        &attempt_id,
        "patch",
        "agent.patch.generated",
        if result.patch.is_some() {
            "succeeded"
        } else {
            "skipped"
        },
        &result.explanation,
        json!({
            "agent": agents.reaper.name,
            "confidence": result.confidence,
            "files_changed": result.files_changed,
            "patch_present": result.patch.is_some(),
        }),
    );

    if result
        .patch
        .as_deref()
        .map(|patch| patch.trim().is_empty())
        .unwrap_or(true)
    {
        let explanation = result.explanation.trim().to_string();
        let _ = tx
            .send(alog(
                &agents.reaper,
                &format!("No patch: {explanation}"),
                "warn",
            ))
            .await;
        update_issue_status_comment(
            &http,
            &issue,
            &scope,
            &bot_token,
            issue_comment_held(&issue, &params.run_id, &attempt_id, "no_patch"),
        )
        .await;
        attempt_finisher
            .skipped_with_error(
                "no_patch",
                if explanation.is_empty() {
                    None
                } else {
                    Some(&explanation)
                },
                cost,
                None,
                0,
            )
            .await;
        return;
    }

    let confidence = result.confidence;
    let _ = tx
        .send(alog(
            &agents.reaper,
            &format!(
                "Patch forged: {} (confidence: {}/100)",
                result.explanation, confidence
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
        PatchSelfHealInput {
            issue: &issue,
            scope: &scope,
            reaper: &agents.reaper,
            codebase: &code_selection.codebase,
            enriched_issue_ctx: &enriched_issue_ctx,
            result,
        },
        &mut cost,
    )
    .await
    {
        Ok(result) => {
            artifact(
                &params.run_id,
                &attempt_id,
                "apply",
                "patch.applied",
                "succeeded",
                "Generated patch applied to the disposable worktree",
                json!({"self_heal_available": true}),
            );
            result
        }
        Err(reason) => {
            artifact(
                &params.run_id,
                &attempt_id,
                "apply",
                "patch.apply_failed",
                "failed",
                &reason,
                json!({}),
            );
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_held(&issue, &params.run_id, &attempt_id, "patch_error"),
            )
            .await;
            attempt_finisher
                .skipped_with_error("patch_error", Some(&reason), cost, None, 0)
                .await;
            return;
        }
    };
    let _ = tx.send(astatus(&agents.reaper.id, "idle", "")).await;

    match crate::git_ops::has_changes(&scope.work_path).await {
        Ok(true) => {}
        Ok(false) => {
            let _ = tx
                .send(alog(
                    &agents.reaper,
                    "Patch applied but produced no file changes — skipping PR",
                    "warn",
                ))
                .await;
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_held(&issue, &params.run_id, &attempt_id, "no_changes"),
            )
            .await;
            attempt_finisher
                .skipped("no_changes", cost, result.patch.as_deref(), confidence)
                .await;
            return;
        }
        Err(e) => {
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_error(&issue, &params.run_id, &attempt_id),
            )
            .await;
            attempt_finisher
                .error(
                    &format!("Could not inspect patch changes: {e}"),
                    cost,
                    confidence,
                )
                .await;
            return;
        }
    }

    let mut smith_review = SmithReviewOutcome {
        final_patch: result.patch.as_deref().unwrap_or("").to_string(),
        smith_note: String::new(),
    };

    if let Some(ref smith) = agents.smith {
        if cancelled(&params) {
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_held(&issue, &params.run_id, &attempt_id, "cancelled"),
            )
            .await;
            attempt_finisher
                .skipped(
                    "cancelled",
                    cost,
                    Some(&smith_review.final_patch),
                    confidence,
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
            &result.explanation,
            smith,
        )
        .await
        {
            Ok((rev, rc)) => {
                cost += rc;
                let sconf = rev.confidence;
                let approved = rev.approved;
                let feedback = rev.feedback;
                artifact(
                    &params.run_id,
                    &attempt_id,
                    "smith",
                    "smith.reviewed",
                    if approved { "succeeded" } else { "rejected" },
                    &feedback,
                    json!({"agent": smith.name, "confidence": sconf, "approved": approved}),
                );

                let _ = tx
                    .send(alog(
                        smith,
                        &format!("{sconf}% — {feedback}"),
                        if approved { "success" } else { "warn" },
                    ))
                    .await;

                if let Some(improved_patch) = rev
                    .improved_patch
                    .as_deref()
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
                    let rejection_id = Uuid::new_v4().to_string()[..12].to_string();
                    let rejection_reason = format!("confidence_{sconf}");
                    let _ = save_rejected_patch(RejectedPatchRecord {
                        id: &rejection_id,
                        run_id: &params.run_id,
                        repo: &scope.repo,
                        issue_number: scope.issue_num,
                        issue_title: issue["title"].as_str().unwrap_or(""),
                        reason: &rejection_reason,
                        feedback: &feedback,
                        confidence: sconf,
                        diff: &smith_review.final_patch,
                    });
                    match submit_smith_rejection_candidate(
                        &http,
                        SmithRejectionCandidateInput {
                            issue: &issue,
                            scope: &scope,
                            selected_files: &code_selection.selected_files,
                            patch_diff: &smith_review.final_patch,
                            feedback: &feedback,
                            smith_confidence: sconf,
                            min_confidence: params.min_conf,
                            run_id: &params.run_id,
                        },
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
                    let skip_reason = format!("confidence_{sconf}");
                    update_issue_status_comment(
                        &http,
                        &issue,
                        &scope,
                        &bot_token,
                        issue_comment_held(&issue, &params.run_id, &attempt_id, &skip_reason),
                    )
                    .await;
                    let _ = finish_attempt(IssueAttemptFinish {
                        attempt_id: &attempt_id,
                        status: IssueAttemptStatus::Skipped,
                        pr_url: None,
                        pr_number: None,
                        cost_usd: cost,
                        patch_diff: Some(&smith_review.final_patch),
                        error_msg: None,
                        skip_reason: Some(&skip_reason),
                        duration_seconds: Some(t_start.elapsed().as_secs_f64()),
                        confidence: sconf,
                    });
                    let _ = tx.send(astatus(&smith.id, "idle", "")).await;
                    cleanup_work_path(&scope.work_path).await;
                    return;
                }
            }
            Err(e) => {
                artifact(
                    &params.run_id,
                    &attempt_id,
                    "smith",
                    "smith.review_failed",
                    "failed",
                    &e.to_string(),
                    json!({"agent": smith.name}),
                );
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
        update_issue_status_comment(
            &http,
            &issue,
            &scope,
            &bot_token,
            issue_comment_held(&issue, &params.run_id, &attempt_id, "cancelled"),
        )
        .await;
        attempt_finisher
            .skipped(
                "cancelled",
                cost,
                Some(&smith_review.final_patch),
                confidence,
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
    let mut test = crate::git_ops::run_tests(&scope.work_path, repository_trusted).await;
    artifact(
        &params.run_id,
        &attempt_id,
        "validate",
        "test.initial",
        test.status.as_str(),
        &test.output,
        json!({"runner": test.runner, "status": test.status.as_str()}),
    );
    let _ = tx
        .send(alog(
            &agents.gatekeeper,
            &format!("Tests via {} {}", test.runner, test_log_label(&test)),
            if test.passed() {
                "success"
            } else if should_retry_test_failure(&test) {
                "warn"
            } else {
                "info"
            },
        ))
        .await;

    for retry in 0..params.retry_count {
        if !should_retry_test_failure(&test) {
            if !test.passed() && retry == 0 {
                let _ = tx
                    .send(alog(
                        &agents.gatekeeper,
                        "Validation did not run tests in a trusted sandbox - preserving patch for PR review",
                        "info",
                    ))
                    .await;
            }
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
                if let Some(retry_patch) = retry_result.patch {
                    let (applied, _) =
                        crate::git_ops::apply_patch(&scope.work_path, &retry_patch).await;
                    if applied {
                        smith_review.final_patch = retry_patch.clone();
                        result.explanation = retry_result.explanation;
                        result.files_changed = retry_result.files_changed;
                        result.patch = Some(retry_patch);
                        test =
                            crate::git_ops::run_tests(&scope.work_path, repository_trusted).await;
                        let _ = tx
                            .send(alog(
                                &agents.reaper,
                                &format!("Retry {}: {}", retry + 1, test_log_label(&test)),
                                if test.passed() { "success" } else { "warn" },
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

    artifact(
        &params.run_id,
        &attempt_id,
        "validate",
        "test.completed",
        test.status.as_str(),
        &test.output,
        json!({"runner": test.runner, "status": test.status.as_str()}),
    );

    if cancelled(&params) {
        update_issue_status_comment(
            &http,
            &issue,
            &scope,
            &bot_token,
            issue_comment_held(&issue, &params.run_id, &attempt_id, "cancelled"),
        )
        .await;
        attempt_finisher
            .skipped(
                "cancelled",
                cost,
                Some(&smith_review.final_patch),
                confidence,
            )
            .await;
        return;
    }

    match crate::git_ops::has_changes(&scope.work_path).await {
        Ok(true) => {}
        Ok(false) => {
            let detail = compact_no_change_detail(&test);
            let _ = tx
                .send(alog(
                    &agents.gatekeeper,
                    "No commit-ready diff remained after validation - skipping PR",
                    "warn",
                ))
                .await;
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_held(&issue, &params.run_id, &attempt_id, "no_changes"),
            )
            .await;
            attempt_finisher
                .skipped_with_error(
                    "no_changes",
                    Some(&detail),
                    cost,
                    Some(&smith_review.final_patch),
                    confidence,
                )
                .await;
            return;
        }
        Err(e) => {
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_error(&issue, &params.run_id, &attempt_id),
            )
            .await;
            attempt_finisher
                .error(
                    &format!("Could not inspect final patch changes: {e}"),
                    cost,
                    confidence,
                )
                .await;
            return;
        }
    }

    let _ = tx
        .send(astatus(
            &agents.gatekeeper.id,
            "working",
            &format!("PR #{}", scope.issue_num),
        ))
        .await;

    let reservation_id = match reserve_pr_slot(
        &http,
        &PrReservationRequest {
            product: "repo-reaper".into(),
            repository: scope.repo.clone(),
            run_id: params.run_id.clone(),
            action: "open_pull_request".into(),
        },
    )
    .await
    {
        Ok(Some(response)) if response.granted => {
            let Some(reservation) = response.reservation else {
                let detail = "HiveCore granted PR capacity without returning a reservation ID.";
                artifact(
                    &params.run_id,
                    &attempt_id,
                    "policy",
                    "hivecore.pr_budget.invalid_response",
                    "blocked",
                    detail,
                    json!({"repo": scope.repo}),
                );
                attempt_finisher
                    .skipped_with_error(
                        "pr_budget_unavailable",
                        Some(detail),
                        cost,
                        Some(&smith_review.final_patch),
                        confidence,
                    )
                    .await;
                return;
            };
            reservation.id
        }
        Ok(Some(response)) => {
            artifact(
                &params.run_id,
                &attempt_id,
                "policy",
                "hivecore.pr_budget.denied",
                "blocked",
                &response.reason,
                json!({
                    "repo": scope.repo,
                    "limiting_layer": response.limiting_layer,
                    "product_used": response.product_used,
                    "product_limit": response.product_limit,
                    "suite_used": response.suite_used,
                    "suite_limit": response.suite_limit,
                }),
            );
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_held(&issue, &params.run_id, &attempt_id, "pr_budget_denied"),
            )
            .await;
            attempt_finisher
                .skipped_with_error(
                    "pr_budget_denied",
                    Some(&response.reason),
                    cost,
                    Some(&smith_review.final_patch),
                    confidence,
                )
                .await;
            return;
        }
        Ok(None) => String::new(),
        Err(error) => {
            let detail = format!("HiveCore PR reservation failed: {error}");
            artifact(
                &params.run_id,
                &attempt_id,
                "policy",
                "hivecore.pr_budget.unavailable",
                "blocked",
                &detail,
                json!({"repo": scope.repo}),
            );
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_held(&issue, &params.run_id, &attempt_id, "pr_budget_unavailable"),
            )
            .await;
            attempt_finisher
                .skipped_with_error(
                    "pr_budget_unavailable",
                    Some(&detail),
                    cost,
                    Some(&smith_review.final_patch),
                    confidence,
                )
                .await;
            return;
        }
    };
    artifact(
        &params.run_id,
        &attempt_id,
        "submit",
        "github.pr.requested",
        "started",
        "Preparing commit, push, and pull request",
        json!({"repo": scope.repo, "issue_number": scope.issue_num}),
    );
    let (pr, pr_number) = match publish_pull_request(
        &http,
        PullRequestPublishInput {
            issue: &issue,
            scope: &scope,
            agents: &agents,
            bot_token: &bot_token,
            bot_user: &bot_user,
            result: &result,
            smith_note: &smith_review.smith_note,
            confidence,
            test: &test,
        },
    )
    .await
    {
        Ok(outcome) => {
            artifact(
                &params.run_id,
                &attempt_id,
                "submit",
                "github.pr.opened",
                "succeeded",
                "Pull request opened",
                json!({"pr_number": outcome.1, "url": outcome.0["html_url"]}),
            );
            outcome
        }
        Err(e) => {
            if !reservation_id.is_empty() {
                if let Err(release_error) =
                    release_pr_slot(&http, &reservation_id, "GitHub PR creation failed.").await
                {
                    tracing::warn!(
                        reservation_id,
                        "could not release HiveCore PR reservation: {release_error:#}"
                    );
                }
            }
            artifact(
                &params.run_id,
                &attempt_id,
                "submit",
                "github.pr.failed",
                "failed",
                &e.to_string(),
                json!({"repo": scope.repo}),
            );
            update_issue_status_comment(
                &http,
                &issue,
                &scope,
                &bot_token,
                issue_comment_error(&issue, &params.run_id, &attempt_id),
            )
            .await;
            attempt_finisher
                .error(&e.to_string(), cost, confidence)
                .await;
            return;
        }
    };

    if !reservation_id.is_empty() {
        let pr_url = pr["html_url"].as_str().unwrap_or("");
        match commit_pr_slot(&http, &reservation_id, pr_url).await {
            Ok(Some(_)) => artifact(
                &params.run_id,
                &attempt_id,
                "policy",
                "hivecore.pr_budget.committed",
                "succeeded",
                "HiveCore committed the PR budget reservation.",
                json!({"reservation_id": reservation_id, "pr_url": pr_url}),
            ),
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(
                    reservation_id,
                    "could not commit HiveCore PR reservation: {error:#}"
                );
                artifact(
                    &params.run_id,
                    &attempt_id,
                    "policy",
                    "hivecore.pr_budget.commit_failed",
                    "failed",
                    &error.to_string(),
                    json!({"reservation_id": reservation_id, "pr_url": pr_url}),
                );
            }
        }
    }

    let _ = track_pr(pr_number, &scope.repo, &params.run_id);
    let duration = t_start.elapsed().as_secs_f64();
    let _ = finish_attempt(IssueAttemptFinish {
        attempt_id: &attempt_id,
        status: IssueAttemptStatus::Fixed,
        pr_url: pr["html_url"].as_str(),
        pr_number: Some(pr_number),
        cost_usd: cost,
        patch_diff: Some(&smith_review.final_patch),
        error_msg: None,
        skip_reason: None,
        duration_seconds: Some(duration),
        confidence,
    });
    let _ = update_perf(
        &agents.reaper.name,
        &agents.reaper.provider,
        &agents.reaper.model,
        "reaper",
        "fixed",
        cost,
    );

    run_cost.fetch_add(
        (cost * 1_000_000.0) as i64,
        std::sync::atomic::Ordering::Relaxed,
    );
    update_issue_status_comment(
        &http,
        &issue,
        &scope,
        &bot_token,
        issue_comment_fixed(
            &issue,
            &params.run_id,
            &attempt_id,
            pr_number,
            pr["html_url"].as_str().unwrap_or(""),
            &test,
        ),
    )
    .await;

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
                    "draft": test.requires_draft(),
                    "repo": scope.repo,
                    "title": issue["title"],
                    "fix": result.explanation,
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

// patch.rs — Patch application, self-heal, and PR publishing

use anyhow::Result as AnyhowResult;
use serde_json::{json, Value};

use crate::agents::agent_patch_retry;
use crate::git_ops::{apply_patch, git_commit_push};
use crate::github::{gh_default_branch, gh_post};
use crate::state::AgentConfig;

use super::sse::alog;
use super::types::{FixAgents, IssueScope, Tx};

fn pr_test_status(test: &crate::git_ops::TestResult) -> &'static str {
    if test.passed {
        return "✅ Passed";
    }

    match test.runner.as_str() {
        "disabled" => "⚠️ Not run — untrusted test execution is disabled (draft PR)",
        "host-disabled" => "⚠️ Not run — host test execution is disabled (draft PR)",
        "invalid" => "⚠️ Not run — test runner configuration is invalid (draft PR)",
        "none" => "⚠️ Not run — no supported test runner was found (draft PR)",
        _ => "⚠️ Failed (draft PR)",
    }
}

pub async fn apply_patch_with_self_heal(
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
    let mut final_apply_err = apply_err.trim().to_string();

    if !applied {
        let _ = tx
            .send(alog(reaper, "Apply failed — self-healing…", "warn"))
            .await;
        match agent_patch_retry(
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
            Ok((retry_result, retry_cost)) => {
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
                        final_apply_err.clear();
                        let _ = tx.send(alog(reaper, "Self-healed ✓", "success")).await;
                    } else {
                        final_apply_err = err.trim().to_string();
                        let _ = tx
                            .send(alog(
                                reaper,
                                &format!("Self-heal apply failed: {err}"),
                                "warn",
                            ))
                            .await;
                    }
                } else {
                    final_apply_err = "self-heal returned no patch".to_string();
                }
            }
            Err(e) => {
                final_apply_err = if final_apply_err.is_empty() {
                    format!("self-heal error: {e}")
                } else {
                    format!("initial git apply error: {final_apply_err}; self-heal error: {e}")
                };
            }
        }
    }

    if !applied {
        let _ = tx
            .send(alog(reaper, "Cannot apply patch — skipping", "error"))
            .await;
        let reason = if final_apply_err.is_empty() {
            "git apply failed without stderr".to_string()
        } else {
            format!("git apply failed: {final_apply_err}")
        };
        return Err(reason);
    }

    Ok(result)
}

pub async fn publish_pull_request(
    http: &reqwest::Client,
    issue: &Value,
    scope: &IssueScope,
    agents: &FixAgents,
    bot_token: &str,
    bot_user: &str,
    result: &Value,
    smith_note: &str,
    confidence: i32,
    test: &crate::git_ops::TestResult,
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
    git_commit_push(
        &scope.work_path,
        &scope.branch,
        &commit_msg,
        Some(bot_user),
        Some(bot_token),
    )
    .await?;

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
        Generated autonomously by **RepoReaper by PatchHive**.\n\
        Closes #{}.\n\n\
        ⚖ Judge: {} · ⚔ Reaper: {} · ⬢ Smith: {} · 🔒 Gatekeeper: {}\n\n\
        *RepoReaper by PatchHive*",
        scope.issue_num,
        issue["title"].as_str().unwrap_or(""),
        result["explanation"].as_str().unwrap_or(""),
        issue["fixability_score"].as_i64().unwrap_or(50),
        issue["fixability_reason"].as_str().unwrap_or(""),
        pr_test_status(test),
        scope.issue_num,
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

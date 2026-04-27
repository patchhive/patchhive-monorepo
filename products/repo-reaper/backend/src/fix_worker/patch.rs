// patch.rs — Patch application, self-heal, and PR publishing

use anyhow::Result as AnyhowResult;
use serde_json::{json, Value};

use crate::agents::*;
use crate::git_ops::*;
use crate::github::*;
use crate::state::AgentConfig;

use super::sse::alog;
use super::types::{FixAgents, IssueScope, Tx};

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

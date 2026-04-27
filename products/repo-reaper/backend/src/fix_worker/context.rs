// context.rs — Repo cloning, code selection, enriched issue context loading

use anyhow::Result as AnyhowResult;
use patchhive_product_core::repo_memory::RepoMemoryContextRequest;
use serde_json::Value;

use crate::agents::*;
use crate::git_ops::*;
use crate::github::*;

use super::memory::build_repo_memory_block;
use super::sse::{alog, astatus};
use super::types::{cleanup_work_path, CodeSelection, IssueScope, Tx};
use crate::state::AgentConfig;

pub async fn clone_issue_repo(
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

pub async fn select_code_context(
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

pub async fn load_enriched_issue_context(
    http: &reqwest::Client,
    tx: &Tx,
    issue: &Value,
    reaper: &AgentConfig,
    selected_files: &[String],
    issue_ctx: &str,
) -> String {
    let repo_memory_context = match patchhive_product_core::repo_memory::fetch_repo_memory_context(
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

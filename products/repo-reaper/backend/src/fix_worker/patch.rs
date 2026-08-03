// patch.rs — Patch application, self-heal, and PR publishing

use anyhow::Result as AnyhowResult;
use patchhive_product_core::write_authorization::{PrBudget, ValidatedChange};
use serde_json::{json, Value};

use crate::agents::{agent_patch_retry, GeneratedPatchResponse};
use crate::git_ops::{apply_patch, git_commit_push};
use crate::github::{gh_default_branch, gh_post};
use crate::state::AgentConfig;

use super::sse::alog;
use super::types::{FixAgents, IssueScope, Tx};

fn pr_test_status(test: &crate::git_ops::TestResult) -> &'static str {
    match test.status {
        patchhive_product_core::validation::TestExecutionStatus::Passed => "✅ Passed",
        patchhive_product_core::validation::TestExecutionStatus::Failed => {
            "⚠️ Failed; review required"
        }
        patchhive_product_core::validation::TestExecutionStatus::Disabled => {
            "⚠️ Not verified — test execution is disabled; review required"
        }
        patchhive_product_core::validation::TestExecutionStatus::Skipped => {
            "⚠️ Not verified — tests were skipped or unavailable; review required"
        }
    }
}

pub struct PatchSelfHealInput<'a> {
    pub issue: &'a Value,
    pub scope: &'a IssueScope,
    pub reaper: &'a AgentConfig,
    pub codebase: &'a str,
    pub enriched_issue_ctx: &'a str,
    pub result: GeneratedPatchResponse,
}

pub async fn apply_patch_with_self_heal(
    http: &reqwest::Client,
    tx: &Tx,
    input: PatchSelfHealInput<'_>,
    cost: &mut f64,
) -> std::result::Result<GeneratedPatchResponse, String> {
    let PatchSelfHealInput {
        issue,
        scope,
        reaper,
        codebase,
        enriched_issue_ctx,
        mut result,
    } = input;
    let patch_str = result.patch.as_deref().unwrap_or("").to_string();
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
                if let Some(retry_patch) = retry_result.patch.as_deref() {
                    let (ok, err) = apply_patch(&scope.work_path, retry_patch).await;
                    if ok {
                        result.explanation = retry_result.explanation;
                        result.files_changed = retry_result.files_changed;
                        result.patch = retry_result.patch;
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

pub struct PullRequestPublishInput<'a> {
    pub issue: &'a Value,
    pub scope: &'a IssueScope,
    pub agents: &'a FixAgents,
    pub bot_token: &'a str,
    pub bot_user: &'a str,
    pub result: &'a GeneratedPatchResponse,
    pub smith_note: &'a str,
    /// The validation evidence for this change. The draft decision is derived
    /// from it here; callers cannot pass a draft flag of their own.
    pub change: &'a ValidatedChange<&'a crate::git_ops::TestResult>,
    /// Exact staged diff authorized after validation. Publication refuses any
    /// staged change that is not byte-for-byte identical.
    pub reviewed_diff: &'a str,
}

/// The one place RepoReaper creates a pull request.
///
/// Publication requires validation evidence and a budget outcome from
/// [`patchhive_product_core::write_authorization::request_pr_budget`], and the
/// reservation is settled here where the pull request URL is known. Do not add
/// a second `POST /repos/{repo}/pulls` call site; route new delivery shapes
/// through this function instead.
pub async fn publish_pull_request(
    http: &reqwest::Client,
    input: PullRequestPublishInput<'_>,
    budget: PrBudget,
) -> AnyhowResult<(Value, i64)> {
    let PullRequestPublishInput {
        issue,
        scope,
        agents,
        bot_token,
        bot_user,
        result,
        smith_note,
        change,
        reviewed_diff,
    } = input;
    let confidence = change.confidence();
    let test = *change.payload();
    let budget_guard = match budget {
        PrBudget::Granted(guard) => Some(guard),
        PrBudget::Unconfigured => None,
        PrBudget::Denied(response) => {
            return Err(anyhow::anyhow!(
                "HiveCore refused pull request capacity: {}",
                response.reason
            ))
        }
    };
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
        reviewed_diff,
    )
    .await?;

    let files_md = result
        .files_changed
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
        Generated autonomously by **RepoReaper by [PatchHive](https://github.com/patchhive)**.\n\
        Closes #{}.\n\n\
        ⚖ Judge: {} · ⚔ Reaper: {} · ⬢ Smith: {} · 🔒 Gatekeeper: {}\n\n\
        *RepoReaper by [PatchHive](https://github.com/patchhive)*",
        scope.issue_num,
        issue["title"].as_str().unwrap_or(""),
        result.explanation,
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
    let pr = match gh_post(
        http,
        &format!("/repos/{}/pulls", scope.repo),
        &json!({
            "title": commit_msg,
            "body": pr_body,
            "head": format!("{bot_user}:{}", scope.branch),
            "base": base_branch,
            // Derived from recorded validation, never supplied by a caller.
            "draft": change.requires_draft(),
        }),
        Some(bot_token),
    )
    .await
    {
        Ok(pr) => pr,
        Err(error) => {
            if let Some(guard) = budget_guard {
                if let Err(release_error) = guard.release("GitHub PR creation failed.").await {
                    tracing::warn!("could not release PR reservation: {release_error:#}");
                }
            }
            return Err(error);
        }
    };

    if let Some(guard) = budget_guard {
        let pr_url = pr["html_url"].as_str().unwrap_or("");
        if let Err(error) = guard.commit(pr_url).await {
            tracing::warn!("could not commit PR reservation: {error:#}");
        }
    }

    Ok((pr.clone(), pr["number"].as_i64().unwrap_or(0)))
}

#[cfg(test)]
mod tests {
    /// Blank out `#[cfg(test)]` modules, preserving line numbering and any
    /// production code that follows them.
    ///
    /// Truncating at the attribute instead would hide a call site written
    /// below a test module, which is exactly the kind of blind spot this
    /// guard exists to prevent.
    fn strip_test_modules(source: &str) -> String {
        let mut output = source.to_string();
        while let Some(start) = output.find("#[cfg(test)]") {
            let Some(open) = output[start..].find('{').map(|offset| start + offset) else {
                output.replace_range(start.., "");
                break;
            };
            let mut depth = 0usize;
            let mut end = None;
            for (offset, character) in output[open..].char_indices() {
                match character {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(open + offset + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let end = end.unwrap_or(output.len());
            let blanked = output[start..end]
                .chars()
                .map(|character| if character == '\n' { '\n' } else { ' ' })
                .collect::<String>();
            output.replace_range(start..end, &blanked);
        }
        output
    }

    /// RepoReaper's write gates are only as good as the number of places that
    /// can open a pull request. A previous webhook path reimplemented the flow
    /// and shipped unvalidated, ready-for-review pull requests because it never
    /// touched `publish_pull_request`. Adding a second creation site must fail
    /// here rather than in production.
    #[test]
    fn pull_request_creation_has_exactly_one_call_site() {
        let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut creation_sites = Vec::new();
        let mut draft_flags = Vec::new();

        let mut pending = vec![source_root];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("readable source directory") {
                let path = entry.expect("readable entry").path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                    continue;
                }
                let file = std::fs::read_to_string(&path).expect("readable source file");
                let source = strip_test_modules(&file);
                for (index, line) in source.lines().enumerate() {
                    let location = format!("{}:{}", path.display(), index + 1);
                    if line.contains("gh_post(") && source.contains("/pulls") {
                        // Narrow to the actual creation call rather than any
                        // POST in a file that also mentions the pulls API.
                        if source
                            .lines()
                            .skip(index)
                            .take(6)
                            .any(|following| following.contains("/pulls"))
                        {
                            creation_sites.push(location.clone());
                        }
                    }
                    if line.contains("\"draft\":") && !line.contains("requires_draft()") {
                        draft_flags.push(location);
                    }
                }
            }
        }

        assert_eq!(
            creation_sites.len(),
            1,
            "pull request creation must stay behind publish_pull_request; found: {creation_sites:?}"
        );
        assert!(
            draft_flags.iter().all(|site| site.contains("github.rs")),
            "a draft flag must be derived from validation evidence, never written literally: {draft_flags:?}"
        );
    }
}

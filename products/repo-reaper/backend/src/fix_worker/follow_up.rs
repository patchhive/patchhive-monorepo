// follow_up.rs — Maintainer-feedback follow-ups on RepoReaper's own pull requests
//
// A follow-up adds a commit to a pull request RepoReaper already opened. It
// never opens a competing pull request: the main pipeline refuses to compete
// with an open linked PR, and a second PR for the same issue would burn suite
// budget, split the review thread, and read as spam.
//
// Because no pull request is created, no PR budget reservation applies. Every
// other gate does: watch mode, ownership of the pull request, a per-PR cap,
// repository policy, review confidence, and recorded run evidence.

use patchhive_github_pr::GitHubPrClient;
use patchhive_product_core::hivecore_policy::{
    check_repository_policy, RepositoryPolicyDecisionRequest,
};
use patchhive_product_core::write_authorization::ValidatedChange;
use serde_json::json;
use uuid::Uuid;

use crate::agents::agent_pr_comment_fix;
use crate::db::{
    finish_run, record_pr_follow_up, record_run_artifact, start_run, tracked_pull_request,
    RunArtifactInput, RunStart, RunStatus, MAX_PR_FOLLOW_UPS,
};
use crate::git_ops::{
    apply_patch, collect_files_all, git_checkout_remote_branch, git_clone, git_commit_push,
    run_tests,
};
use crate::github::{gh_comment_issue, gh_fork, repo_reaper_github_token};
use crate::state::AppState;

pub struct FollowUpRequest {
    pub repo: String,
    pub pr_number: i64,
    pub issue_title: String,
    pub comment_body: String,
    pub comment_author: String,
}

/// Why a follow-up did not happen. Every variant is recorded as run evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowUpRefusal {
    NotOurPullRequest,
    PullRequestClosed,
    FollowUpCapReached,
    RepositoryBlocked,
    NoReaperAgent,
    ConfidenceBelowFloor,
}

impl FollowUpRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NotOurPullRequest => "not_our_pull_request",
            Self::PullRequestClosed => "pull_request_closed",
            Self::FollowUpCapReached => "follow_up_cap_reached",
            Self::RepositoryBlocked => "repository_blocked",
            Self::NoReaperAgent => "no_reaper_agent",
            Self::ConfidenceBelowFloor => "confidence_below_floor",
        }
    }

    pub fn detail(self) -> String {
        match self {
            Self::NotOurPullRequest => {
                "RepoReaper did not open this pull request, so it will not push to it.".into()
            }
            Self::PullRequestClosed => {
                "The pull request is closed or merged, so a follow-up commit would be pointless."
                    .into()
            }
            Self::FollowUpCapReached => format!(
                "RepoReaper has already attempted {MAX_PR_FOLLOW_UPS} follow-ups on this pull request. Further feedback needs a human."
            ),
            Self::RepositoryBlocked => {
                "Repository policy blocks automated writes to this repository.".into()
            }
            Self::NoReaperAgent => "No Reaper agent is configured to generate a patch.".into(),
            Self::ConfidenceBelowFloor => {
                "The corrected patch did not reach the configured review confidence floor.".into()
            }
        }
    }
}

fn artifact(run_id: &str, phase: &str, kind: &str, status: &str, message: &str) {
    if let Err(error) = record_run_artifact(RunArtifactInput {
        run_id,
        attempt_id: None,
        phase,
        kind,
        status,
        message,
        metadata: None,
    }) {
        tracing::warn!(
            run_id,
            kind,
            "could not persist follow-up artifact: {error:#}"
        );
    }
}

/// Run one maintainer-feedback follow-up.
///
/// Returns the refusal when a gate stops the work, so the caller can report it
/// without inspecting run rows.
pub async fn run_follow_up(
    state: AppState,
    request: FollowUpRequest,
) -> Result<(), FollowUpRefusal> {
    let run_id = Uuid::new_v4().to_string()[..12].to_string();
    let config = json!({
        "kind": "pr_follow_up",
        "repo": request.repo,
        "pr_number": request.pr_number,
        "comment_author": request.comment_author,
    });
    let _ = start_run(RunStart {
        run_id: &run_id,
        config_json: &config.to_string(),
        dry_run: false,
    });

    let outcome = execute(&state, &request, &run_id).await;
    let status = match &outcome {
        Ok(()) => RunStatus::Done,
        Err(refusal) => {
            artifact(
                &run_id,
                "policy",
                "follow_up.refused",
                "blocked",
                &refusal.detail(),
            );
            RunStatus::Skipped
        }
    };
    let _ = finish_run(&run_id, 0, 1, 0.0, status);
    outcome
}

async fn execute(
    state: &AppState,
    request: &FollowUpRequest,
    run_id: &str,
) -> Result<(), FollowUpRefusal> {
    // RepoReaper answers its own review threads only.
    let tracked = tracked_pull_request(&request.repo, request.pr_number)
        .ok()
        .flatten()
        .ok_or(FollowUpRefusal::NotOurPullRequest)?;
    if tracked.state != "open" || tracked.merged {
        return Err(FollowUpRefusal::PullRequestClosed);
    }
    if !tracked.accepts_follow_up() {
        return Err(FollowUpRefusal::FollowUpCapReached);
    }

    let policy = check_repository_policy(
        &state.http,
        &RepositoryPolicyDecisionRequest {
            repository: request.repo.clone(),
            product: "repo-reaper".into(),
            operation: "repo_automation".into(),
        },
    )
    .await
    .map_err(|_| FollowUpRefusal::RepositoryBlocked)?;
    if policy.as_ref().is_some_and(|decision| !decision.allowed()) {
        return Err(FollowUpRefusal::RepositoryBlocked);
    }
    let repository_trusted = policy
        .as_ref()
        .map(|decision| decision.trusted)
        .unwrap_or(false);

    let agents = state.agents.read().await.clone();
    let reaper = agents
        .values()
        .find(|agent| agent.role == "reaper")
        .or_else(|| agents.values().next())
        .cloned()
        .ok_or(FollowUpRefusal::NoReaperAgent)?;

    // The cap is consumed as soon as the attempt is real, so a crash or a
    // failure cannot be retried indefinitely by re-commenting.
    let attempt = record_pr_follow_up(&request.repo, request.pr_number).unwrap_or_default();
    artifact(
        run_id,
        "discover",
        "follow_up.started",
        "started",
        &format!(
            "Follow-up {attempt}/{MAX_PR_FOLLOW_UPS} on {}#{}",
            request.repo, request.pr_number
        ),
    );

    let bot_token = reaper
        .bot_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(repo_reaper_github_token)
        .unwrap_or_default();
    let bot_user = reaper
        .bot_user
        .clone()
        .unwrap_or_else(|| std::env::var("BOT_GITHUB_USER").unwrap_or_default());

    let pr_client = GitHubPrClient::new(
        state.http.clone(),
        Some(bot_token.clone()),
        "RepoReaper by PatchHive",
    );
    let Ok(detail) = pr_client
        .fetch_pull_request(&request.repo, request.pr_number)
        .await
    else {
        artifact(
            run_id,
            "discover",
            "follow_up.pr_unreadable",
            "failed",
            "Could not read the pull request from GitHub.",
        );
        return Ok(());
    };
    if detail.state != "open" || detail.merged {
        return Err(FollowUpRefusal::PullRequestClosed);
    }

    let work_dir = crate::fix_worker::types::work_dir()
        .join(format!("followup-{}-{}", request.pr_number, run_id));
    if work_dir.exists() {
        let _ = tokio::fs::remove_dir_all(&work_dir).await;
    }

    let outcome = deliver(
        state,
        request,
        run_id,
        DeliveryInput {
            reaper: &reaper,
            bot_token: &bot_token,
            bot_user: &bot_user,
            head_ref: &detail.head_ref,
            work_dir: &work_dir,
            repository_trusted,
            pr_was_draft: detail.draft,
            pr_client: &pr_client,
        },
    )
    .await;

    if work_dir.exists() {
        let _ = tokio::fs::remove_dir_all(&work_dir).await;
    }
    outcome
}

struct DeliveryInput<'a> {
    reaper: &'a crate::state::AgentConfig,
    bot_token: &'a str,
    bot_user: &'a str,
    head_ref: &'a str,
    work_dir: &'a std::path::Path,
    repository_trusted: bool,
    pr_was_draft: bool,
    pr_client: &'a GitHubPrClient,
}

async fn deliver(
    state: &AppState,
    request: &FollowUpRequest,
    run_id: &str,
    input: DeliveryInput<'_>,
) -> Result<(), FollowUpRefusal> {
    let Ok(fork) = gh_fork(
        &state.http,
        &request.repo,
        Some(input.bot_token),
        Some(input.bot_user),
    )
    .await
    else {
        artifact(
            run_id,
            "patch",
            "follow_up.fork_failed",
            "failed",
            "Could not resolve the RepoReaper fork.",
        );
        return Ok(());
    };

    if git_clone(
        fork["clone_url"].as_str().unwrap_or(""),
        input.work_dir,
        Some(input.bot_user),
        Some(input.bot_token),
    )
    .await
    .is_err()
        || git_checkout_remote_branch(input.work_dir, input.head_ref)
            .await
            .is_err()
    {
        artifact(
            run_id,
            "patch",
            "follow_up.checkout_failed",
            "failed",
            &format!(
                "Could not check out the pull request branch {}.",
                input.head_ref
            ),
        );
        return Ok(());
    }

    let codebase = collect_files_all(input.work_dir, 60_000).await;
    let Ok((patch_response, _cost)) = agent_pr_comment_fix(
        &state.http,
        &request.issue_title,
        &request.comment_body,
        &codebase,
        input.reaper,
    )
    .await
    else {
        artifact(
            run_id,
            "patch",
            "follow_up.generation_failed",
            "failed",
            "The Reaper could not produce a corrected patch.",
        );
        return Ok(());
    };

    let minimum_confidence = std::env::var("MIN_REVIEW_CONFIDENCE")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(40);
    if patch_response.confidence < minimum_confidence {
        return Err(FollowUpRefusal::ConfidenceBelowFloor);
    }

    let Some(patch) = patch_response
        .patch
        .as_deref()
        .filter(|p| !p.trim().is_empty())
    else {
        artifact(
            run_id,
            "patch",
            "follow_up.no_patch",
            "skipped",
            "The Reaper returned no patch for this feedback.",
        );
        return Ok(());
    };

    let (applied, apply_error) = apply_patch(input.work_dir, patch).await;
    if !applied {
        artifact(
            run_id,
            "patch",
            "follow_up.apply_failed",
            "failed",
            &apply_error,
        );
        return Ok(());
    }

    let test = run_tests(input.work_dir, input.repository_trusted).await;
    let validated = ValidatedChange::new(&test, test.status, patch_response.confidence);
    artifact(
        run_id,
        "validate",
        "follow_up.tested",
        test.status.as_str(),
        &test.output,
    );

    let commit_message = format!(
        "fix: follow-up based on maintainer feedback (re #{})",
        request.pr_number
    );
    if git_commit_push(
        input.work_dir,
        input.head_ref,
        &commit_message,
        Some(input.bot_user),
        Some(input.bot_token),
    )
    .await
    .is_err()
    {
        artifact(
            run_id,
            "submit",
            "follow_up.push_failed",
            "failed",
            "Could not push the follow-up commit.",
        );
        return Ok(());
    }
    artifact(
        run_id,
        "submit",
        "follow_up.pushed",
        "succeeded",
        &format!("Pushed a follow-up commit to {}.", input.head_ref),
    );

    // A commit that did not pass validation must not sit in a pull request
    // presented as ready to merge. Demoting is more honest than staying quiet:
    // the maintainer sees the attempt and its result.
    let demoted = if validated.requires_draft() && !input.pr_was_draft {
        match input
            .pr_client
            .convert_pull_request_to_draft(&request.repo, request.pr_number)
            .await
        {
            Ok(is_draft) => {
                artifact(
                    run_id,
                    "submit",
                    "follow_up.converted_to_draft",
                    if is_draft { "succeeded" } else { "failed" },
                    "Validation did not pass, so the pull request was returned to draft.",
                );
                is_draft
            }
            Err(error) => {
                artifact(
                    run_id,
                    "submit",
                    "follow_up.draft_conversion_failed",
                    "failed",
                    &error.to_string(),
                );
                false
            }
        }
    } else {
        false
    };

    gh_comment_issue(
        &state.http,
        &request.repo,
        request.pr_number,
        &follow_up_comment(&patch_response.explanation, &test, demoted),
        Some(input.bot_token),
    )
    .await;
    Ok(())
}

fn follow_up_comment(
    explanation: &str,
    test: &crate::git_ops::TestResult,
    demoted: bool,
) -> String {
    let validation = match test.status {
        patchhive_product_core::validation::TestExecutionStatus::Passed => {
            format!("✅ Validation passed via `{}`.", test.runner)
        }
        patchhive_product_core::validation::TestExecutionStatus::Failed => {
            format!("⚠️ Validation failed via `{}`.", test.runner)
        }
        patchhive_product_core::validation::TestExecutionStatus::Disabled => {
            "⚠️ Validation is disabled for this repository, so this commit is unverified.".into()
        }
        patchhive_product_core::validation::TestExecutionStatus::Skipped => {
            "⚠️ No usable test runner was found, so this commit is unverified.".into()
        }
    };
    let draft_note = if demoted {
        "\n\nThis pull request has been returned to draft until the change is verified."
    } else {
        ""
    };
    patchhive_product_core::branding::append_product_signature(
        &format!(
            "🔱 RepoReaper pushed a follow-up commit based on your feedback.\n\n\
            **What changed:** {explanation}\n\n\
            {validation}{draft_note}\n\n\
            Generated autonomously by RepoReaper."
        ),
        "RepoReaper",
    )
}

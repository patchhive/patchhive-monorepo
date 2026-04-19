use anyhow::Result;
use patchhive_github_pr::{
    env_value, github_token_from_env, GitHubCheckRunRequest, GitHubCommitHealth,
    GitHubCommitStatusRequest, GitHubManagedCommentResult, GitHubPrClient, GitHubPullRequestDetail,
    GitHubPullReview, GitHubPullReviewThread,
};
use reqwest::Client;

use crate::models::{GitHubReportOutcome, MergeAssessment};

const STATUS_CONTEXT: &str = "mergekeeper/readiness";
const CHECK_RUN_NAME: &str = "MergeKeeper";
const COMMENT_MARKER: &str = "<!-- patchhive-mergekeeper-report -->";

pub struct GitHubMergeContext {
    pub pr: GitHubPullRequestDetail,
    pub reviews: Vec<GitHubPullReview>,
    pub threads: Vec<GitHubPullReviewThread>,
    pub commit_health: GitHubCommitHealth,
    pub diff: String,
}

pub fn github_token_configured() -> bool {
    github_token_from_env().is_some()
}

pub fn webhook_secret() -> Option<String> {
    env_value(&["MERGE_KEEPER_GITHUB_WEBHOOK_SECRET"])
}

pub fn webhook_secret_configured() -> bool {
    webhook_secret().is_some()
}

pub fn public_url_configured() -> bool {
    env_value(&["MERGE_KEEPER_PUBLIC_URL"]).is_some()
}

fn pr_client(client: &Client) -> GitHubPrClient {
    GitHubPrClient::with_env_token(client.clone(), "merge-keeper/0.1")
}

pub async fn fetch_merge_context(
    client: &Client,
    repo: &str,
    pr_number: i64,
) -> Result<GitHubMergeContext> {
    let client = pr_client(client);
    let pr = client.fetch_pull_request(repo, pr_number).await?;
    let reviews = client.fetch_pull_request_reviews(repo, pr_number).await?;
    let threads = client
        .fetch_pull_request_review_threads(repo, pr_number)
        .await?;
    let commit_health = client.fetch_commit_health(repo, &pr.head_sha).await?;
    let diff = client.fetch_pull_request_diff(repo, pr_number).await?;

    Ok(GitHubMergeContext {
        pr,
        reviews,
        threads,
        commit_health,
        diff,
    })
}

fn details_url(assessment: &MergeAssessment) -> Option<String> {
    let base = std::env::var("MERGE_KEEPER_PUBLIC_URL").ok()?;
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    Some(format!("{trimmed}?run={}", assessment.id))
}

fn readiness_emoji(readiness: &str) -> &'static str {
    match readiness {
        "ready" => "🟢",
        "blocked" => "🔴",
        _ => "🟡",
    }
}

fn check_conclusion(assessment: &MergeAssessment) -> &'static str {
    match assessment.readiness.as_str() {
        "ready" => "success",
        "hold" => "action_required",
        _ => "failure",
    }
}

fn commit_state(assessment: &MergeAssessment) -> &'static str {
    match assessment.readiness.as_str() {
        "ready" => "success",
        "hold" => "pending",
        _ => "failure",
    }
}

fn next_move(assessment: &MergeAssessment) -> &'static str {
    match assessment.readiness.as_str() {
        "ready" => "This PR looks merge-ready from MergeKeeper's perspective. A final human glance is still healthy, but the major merge pressure looks cleared.",
        "hold" => "This PR is not blocked outright, but MergeKeeper still sees enough open pressure that a human should pause before merging.",
        _ => "Do not merge this PR yet. MergeKeeper still sees blockers that need to clear before merge is healthy.",
    }
}

fn markdown_signals(items: &[crate::models::MergeSignal], empty: &str) -> String {
    if items.is_empty() {
        return format!("- {empty}");
    }

    items
        .iter()
        .take(6)
        .map(|signal| {
            let evidence = if signal.evidence.is_empty() {
                String::new()
            } else {
                format!(" Evidence: {}.", signal.evidence.join("; "))
            };
            format!(
                "- **{}** (`{}`): {}.{}",
                signal.label, signal.severity, signal.detail, evidence
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn cross_product_markdown(assessment: &MergeAssessment) -> String {
    let mut lines = Vec::new();

    if let Some(review_bee) = assessment.review_bee.as_ref() {
        lines.push(format!(
            "- **ReviewBee** `{}`: {} ({} open item{}, {} actionable thread{})",
            if review_bee.status.trim().is_empty() {
                "linked"
            } else {
                review_bee.status.as_str()
            },
            review_bee.summary,
            review_bee.open_items,
            if review_bee.open_items == 1 { "" } else { "s" },
            review_bee.actionable_threads,
            if review_bee.actionable_threads == 1 {
                ""
            } else {
                "s"
            },
        ));
    }

    if let Some(trust_gate) = assessment.trust_gate.as_ref() {
        lines.push(format!(
            "- **TrustGate** `{}`: {} (risk {}, {} blocked finding{}, {} warning finding{})",
            if trust_gate.recommendation.trim().is_empty() {
                "linked"
            } else {
                trust_gate.recommendation.as_str()
            },
            trust_gate.summary,
            trust_gate.risk_score,
            trust_gate.blocked_findings,
            if trust_gate.blocked_findings == 1 {
                ""
            } else {
                "s"
            },
            trust_gate.warning_findings,
            if trust_gate.warning_findings == 1 {
                ""
            } else {
                "s"
            },
        ));
    }

    if let Some(repo_memory) = assessment.repo_memory.as_ref() {
        lines.push(format!(
            "- **RepoMemory**: {} ({} policy entr{}, {} pinned entr{})",
            repo_memory.summary,
            repo_memory.policy_entries,
            if repo_memory.policy_entries == 1 {
                "y"
            } else {
                "ies"
            },
            repo_memory.pinned_entries,
            if repo_memory.pinned_entries == 1 {
                "y"
            } else {
                "ies"
            },
        ));
    }

    if lines.is_empty() {
        "- No additional PatchHive context was linked for this run.".into()
    } else {
        lines.join("\n")
    }
}

fn render_comment_markdown(assessment: &MergeAssessment) -> String {
    let details_line = details_url(assessment)
        .map(|url| format!("[Open MergeKeeper details]({url})"))
        .unwrap_or_else(|| "MergeKeeper details are local to the current PatchHive host.".into());
    let github = assessment.github.as_ref();

    format!(
        "{COMMENT_MARKER}
## {emoji} MergeKeeper: {readiness}

**Summary:** {summary}

### Snapshot
- Repo: `{repo}`
- PR: #{pr_number}
- Approvals: **{approvals}**
- Requested changes: **{changes_requested}**
- Failing checks: **{failing_checks}**
- Pending checks: **{pending_checks}**
- Actionable open threads: **{open_threads}**
- Changed files: **{changed_files}**
- Additions / deletions: **+{additions} / -{deletions}**
- Trigger: `{trigger}`

### Blockers
{blockers}

### Holds
{warnings}

### Cross-product context
{cross_product}

### Recommendation
{next_move}

{details_line}",
        emoji = readiness_emoji(&assessment.readiness),
        readiness = assessment.readiness.to_uppercase(),
        summary = assessment.summary,
        repo = assessment.repo,
        pr_number = assessment.pr_number,
        approvals = assessment.metrics.approvals,
        changes_requested = assessment.metrics.changes_requested,
        failing_checks = assessment.metrics.failing_checks,
        pending_checks = assessment.metrics.pending_checks,
        open_threads = assessment.metrics.actionable_open_threads,
        changed_files = assessment.metrics.changed_files,
        additions = assessment.metrics.additions,
        deletions = assessment.metrics.deletions,
        trigger = github
            .map(|value| value.trigger.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("manual"),
        blockers = markdown_signals(&assessment.blockers, "No hard blockers."),
        warnings = markdown_signals(&assessment.warnings, "No hold-level warnings."),
        cross_product = cross_product_markdown(assessment),
        next_move = next_move(assessment),
        details_line = details_line,
    )
}

pub fn preview_assessment_outcome(
    assessment: &MergeAssessment,
    message: &str,
) -> GitHubReportOutcome {
    GitHubReportOutcome {
        attempted: false,
        delivered: false,
        method: "none".into(),
        state: "skipped".into(),
        message: message.into(),
        details: Vec::new(),
        check_url: String::new(),
        status_url: String::new(),
        comment_url: String::new(),
        comment_mode: String::new(),
        report_markdown: render_comment_markdown(assessment),
    }
}

pub async fn publish_assessment_outcome(
    client: &Client,
    assessment: &MergeAssessment,
) -> GitHubReportOutcome {
    let Some(github) = assessment.github.as_ref() else {
        return preview_assessment_outcome(
            assessment,
            "This MergeKeeper run was not tied to a GitHub pull request.",
        );
    };

    let markdown = render_comment_markdown(assessment);
    if !github_token_configured() {
        return GitHubReportOutcome {
            attempted: true,
            delivered: false,
            method: "none".into(),
            state: "missing_token".into(),
            message: "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required before MergeKeeper can publish back to GitHub.".into(),
            details: vec![
                "GitHub PR assessment still works for public repos without a token.".into(),
                "Maintained PR comments and check-style signals stay disabled until a token is configured.".into(),
            ],
            check_url: String::new(),
            status_url: String::new(),
            comment_url: String::new(),
            comment_mode: String::new(),
            report_markdown: markdown,
        };
    }

    let gh = pr_client(client);
    let target_repo = if github.head_repo.trim().is_empty() {
        assessment.repo.as_str()
    } else {
        github.head_repo.as_str()
    };
    let mut details = Vec::new();
    let mut method = "none".to_string();
    let mut delivered = false;
    let mut check_url = String::new();
    let mut status_url = String::new();
    let mut comment_url = String::new();
    let mut comment_mode = String::new();

    match gh
        .create_check_run(
            target_repo,
            GitHubCheckRunRequest {
                name: CHECK_RUN_NAME.into(),
                head_sha: github.head_sha.clone(),
                conclusion: check_conclusion(assessment).into(),
                external_id: assessment.id.clone(),
                details_url: details_url(assessment),
                title: format!("MergeKeeper: {}", assessment.readiness.to_uppercase()),
                summary: assessment.summary.clone(),
                text: format!(
                    "Blockers:\n{}\n\nHolds:\n{}",
                    markdown_signals(&assessment.blockers, "No hard blockers."),
                    markdown_signals(&assessment.warnings, "No hold-level warnings."),
                ),
            },
        )
        .await
    {
        Ok(result) => {
            check_url = result.html_url;
            details.push(if check_url.is_empty() {
                "Created a GitHub check run.".into()
            } else {
                format!("Created GitHub check run: {check_url}")
            });
            method = "check_run".into();
            delivered = true;
        }
        Err(err) => details.push(format!(
            "Check run failed, falling back to commit status: {err}"
        )),
    }

    if !delivered {
        match gh
            .create_commit_status(
                target_repo,
                GitHubCommitStatusRequest {
                    sha: github.head_sha.clone(),
                    state: commit_state(assessment).into(),
                    context: STATUS_CONTEXT.into(),
                    description: match assessment.readiness.as_str() {
                        "ready" => "MergeKeeper says this PR looks ready.".into(),
                        "hold" => "MergeKeeper sees hold-level merge pressure.".into(),
                        _ => "MergeKeeper sees blockers on this PR.".into(),
                    },
                    target_url: details_url(assessment),
                },
            )
            .await
        {
            Ok(result) => {
                status_url = result.url;
                details.push(if status_url.is_empty() {
                    "Created a commit status fallback.".into()
                } else {
                    format!("Created commit status fallback: {status_url}")
                });
                method = "commit_status".into();
                delivered = true;
            }
            Err(err) => details.push(format!("Commit status failed: {err}")),
        }
    }

    match gh
        .upsert_issue_comment(&github.repo, github.pr_number, COMMENT_MARKER, &markdown)
        .await
    {
        Ok(GitHubManagedCommentResult { mode, html_url }) => {
            comment_mode = mode;
            comment_url = html_url.clone();
            if method == "none" {
                method = "pr_comment".into();
            }
            details.push(if html_url.is_empty() {
                format!("{} MergeKeeper PR comment.", comment_mode)
            } else {
                format!("{} MergeKeeper PR comment: {html_url}", comment_mode)
            });
            delivered = true;
        }
        Err(err) => details.push(format!("PR comment upsert failed: {err}")),
    }

    GitHubReportOutcome {
        attempted: true,
        delivered,
        method,
        state: if delivered {
            assessment.readiness.clone()
        } else {
            "report_failed".into()
        },
        message: if delivered {
            "MergeKeeper published its merge-readiness call back to GitHub with a maintained PR comment and status signal.".into()
        } else {
            "MergeKeeper assessed the PR but could not publish the result back to GitHub.".into()
        },
        details,
        check_url,
        status_url,
        comment_url,
        comment_mode,
        report_markdown: markdown,
    }
}

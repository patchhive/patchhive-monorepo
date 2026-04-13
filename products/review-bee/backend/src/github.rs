use anyhow::Result;
use patchhive_github_pr::{
    env_value, github_token_from_env, GitHubManagedCommentResult, GitHubPrClient,
    GitHubPullRequestDetail, GitHubPullReview, GitHubPullReviewThread,
};
use reqwest::Client;

use crate::models::{GitHubReportOutcome, ReviewResult};

const COMMENT_MARKER: &str = "<!-- patchhive-reviewbee-report -->";

pub struct GitHubReviewContext {
    pub pr: GitHubPullRequestDetail,
    pub reviews: Vec<GitHubPullReview>,
    pub threads: Vec<GitHubPullReviewThread>,
}

pub fn github_token_configured() -> bool {
    github_token_from_env().is_some()
}

pub fn webhook_secret() -> Option<String> {
    env_value(&["REVIEW_BEE_GITHUB_WEBHOOK_SECRET"])
}

pub fn webhook_secret_configured() -> bool {
    webhook_secret().is_some()
}

pub fn public_url_configured() -> bool {
    env_value(&["REVIEW_BEE_PUBLIC_URL"]).is_some()
}

fn pr_client(client: &Client) -> GitHubPrClient {
    GitHubPrClient::with_env_token(client.clone(), "review-bee/0.1")
}

pub async fn fetch_review_context(
    client: &Client,
    repo: &str,
    pr_number: i64,
) -> Result<GitHubReviewContext> {
    let client = pr_client(client);
    let pr = client.fetch_pull_request(repo, pr_number).await?;
    let reviews = client.fetch_pull_request_reviews(repo, pr_number).await?;
    let threads = client
        .fetch_pull_request_review_threads(repo, pr_number)
        .await?;

    Ok(GitHubReviewContext {
        pr,
        reviews,
        threads,
    })
}

fn details_url(review: &ReviewResult) -> Option<String> {
    let base = std::env::var("REVIEW_BEE_PUBLIC_URL").ok()?;
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    Some(format!("{trimmed}/history/{}", review.id))
}

fn recommendation_emoji(status: &str) -> &'static str {
    match status {
        "clear" | "resolved" => "🟢",
        "attention" => "🔴",
        _ => "🟡",
    }
}

fn next_move(review: &ReviewResult) -> &'static str {
    match review.status.as_str() {
        "clear" => "This PR looks close to merge from a reviewer-feedback standpoint. A quick human scan is still healthy, but ReviewBee did not find active follow-up.",
        "resolved" => "Most prior requests look cleared. Double-check the remaining thread state and merge when the repo is comfortable.",
        "attention" => "This PR still has concentrated review pressure. The open checklist below is the work to clear before merge.",
        _ => "There is a manageable but real follow-up list here. Closing these items should reduce back-and-forth before merge.",
    }
}

fn checklist_markdown(review: &ReviewResult, resolved: bool, limit: usize) -> String {
    let items = review
        .checklist
        .iter()
        .filter(|item| {
            if resolved {
                item.status == "resolved"
            } else {
                item.status == "open" || item.status == "mixed"
            }
        })
        .take(limit)
        .map(|item| {
            let prefix = if resolved { "- [x]" } else { "- [ ]" };
            let paths = if item.path_hints.is_empty() {
                String::new()
            } else {
                format!(" — `{}`", item.path_hints.join("`, `"))
            };
            format!(
                "{prefix} **{}**{} ({}/{})",
                item.title, paths, item.open_threads, item.comment_count
            )
        })
        .collect::<Vec<_>>();

    if items.is_empty() {
        if resolved {
            "- None yet.".into()
        } else {
            "- No active checklist items right now.".into()
        }
    } else {
        items.join("\n")
    }
}

fn render_comment_markdown(review: &ReviewResult) -> String {
    let details_url = details_url(review);
    let details_line = details_url
        .as_ref()
        .map(|url| format!("[Open ReviewBee details]({url})"))
        .unwrap_or_else(|| "ReviewBee details are local to the current PatchHive host.".into());
    let trigger = review
        .github
        .as_ref()
        .map(|value| value.trigger.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("manual");
    let open_section = checklist_markdown(review, false, 6);
    let resolved_section = checklist_markdown(review, true, 3);
    let prompt_section = if review.prompt_suggestions.is_empty() {
        String::new()
    } else {
        format!(
            "\n### Suggested Next Prompts\n{}\n",
            review
                .prompt_suggestions
                .iter()
                .take(3)
                .map(|prompt| format!("- {}", prompt))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    format!(
        "{COMMENT_MARKER}
## {emoji} ReviewBee checklist

**Status:** `{status}`  
**Summary:** {summary}

### Snapshot
- Repo: `{repo}`
- PR: #{pr_number}
- Open items: **{open_items}**
- Resolved items: **{resolved_items}**
- Actionable threads: **{threads}**
- Reviewers: **{reviewers}**
- Trigger: `{trigger}`

### Still Open
{open_section}

### Recently Resolved
{resolved_section}
{prompt_section}
### Recommendation
{next_move}

{details_line}",
        emoji = recommendation_emoji(&review.status),
        status = review.status,
        summary = review.summary,
        repo = review.repo,
        pr_number = review.pr_number,
        open_items = review.metrics.open_items,
        resolved_items = review.metrics.resolved_items,
        threads = review.metrics.actionable_threads,
        reviewers = review.metrics.reviewer_count,
        trigger = trigger,
        open_section = open_section,
        resolved_section = resolved_section,
        prompt_section = prompt_section,
        next_move = next_move(review),
        details_line = details_line,
    )
}

pub fn preview_review_outcome(review: &ReviewResult, message: &str) -> GitHubReportOutcome {
    GitHubReportOutcome {
        attempted: false,
        delivered: false,
        method: "comment".into(),
        state: "skipped".into(),
        message: message.into(),
        comment_url: String::new(),
        comment_mode: String::new(),
        report_markdown: render_comment_markdown(review),
    }
}

fn publish_success(
    mode: &str,
    result: GitHubManagedCommentResult,
    review: &ReviewResult,
) -> GitHubReportOutcome {
    GitHubReportOutcome {
        attempted: true,
        delivered: true,
        method: "comment".into(),
        state: "published".into(),
        message: format!("ReviewBee {mode} the maintained PR comment."),
        comment_url: result.html_url,
        comment_mode: result.mode,
        report_markdown: render_comment_markdown(review),
    }
}

pub async fn publish_review_outcome(client: &Client, review: &ReviewResult) -> GitHubReportOutcome {
    let Some(github) = review.github.as_ref() else {
        return preview_review_outcome(review, "This review was not tied to a GitHub pull request.");
    };

    if !github_token_configured() {
        return preview_review_outcome(
            review,
            "Configure BOT_GITHUB_TOKEN or GITHUB_TOKEN before ReviewBee can maintain PR comments.",
        );
    }

    let markdown = render_comment_markdown(review);
    let client = pr_client(client);
    match client
        .upsert_issue_comment(&github.repo, github.pr_number, COMMENT_MARKER, &markdown)
        .await
    {
        Ok(result) => {
            let mode = result.mode.clone();
            publish_success(&mode, result, review)
        }
        Err(err) => GitHubReportOutcome {
            attempted: true,
            delivered: false,
            method: "comment".into(),
            state: "failed".into(),
            message: format!("ReviewBee could not publish the maintained PR comment: {err}"),
            comment_url: String::new(),
            comment_mode: String::new(),
            report_markdown: markdown,
        },
    }
}

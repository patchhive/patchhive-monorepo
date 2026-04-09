use anyhow::Result;
use patchhive_github_pr::{
    env_value, github_token_from_env, GitHubCheckRunRequest, GitHubCommitStatusRequest,
    GitHubManagedCommentResult, GitHubPrClient, GitHubPullRequest,
};
use reqwest::Client;

use crate::{
    db,
    models::{GitHubReportOutcome, ReportTemplateSet, ReviewResult},
};

const STATUS_CONTEXT: &str = "trustgate/recommendation";
const CHECK_RUN_NAME: &str = "TrustGate";
const COMMENT_MARKER: &str = "<!-- patchhive-trustgate-report -->";

struct RenderedGitHubReport {
    check_title: String,
    check_summary: String,
    check_text: String,
    comment_markdown: String,
    template_scope: String,
    template_repo: String,
}

pub fn github_token_configured() -> bool {
    github_token_from_env().is_some()
}

pub fn webhook_secret() -> Option<String> {
    env_value(&["TRUST_GITHUB_WEBHOOK_SECRET"])
}

pub fn webhook_secret_configured() -> bool {
    webhook_secret().is_some()
}

fn pr_client(client: &Client) -> GitHubPrClient {
    GitHubPrClient::with_env_token(client.clone(), "trust-gate/0.1")
}

pub async fn fetch_pull_request(
    client: &Client,
    repo: &str,
    pr_number: i64,
) -> Result<GitHubPullRequest> {
    pr_client(client).fetch_pull_request(repo, pr_number).await
}

pub async fn fetch_pull_request_diff(
    client: &Client,
    repo: &str,
    pr_number: i64,
) -> Result<String> {
    pr_client(client).fetch_pull_request_diff(repo, pr_number).await
}

fn details_url(review: &ReviewResult) -> Option<String> {
    let base = std::env::var("TRUSTGATE_PUBLIC_URL").ok()?;
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    Some(format!("{trimmed}/history/{}", review.id))
}

fn report_templates_for_repo(repo: &str) -> (ReportTemplateSet, String, String) {
    match db::get_report_templates(repo) {
        Ok(Some(mut templates)) => {
            if templates.repo.trim().is_empty() {
                templates.repo = repo.into();
            }
            (templates, "repo".into(), repo.into())
        }
        _ => {
            let mut templates = ReportTemplateSet::default();
            templates.repo = repo.into();
            (templates, "default".into(), repo.into())
        }
    }
}

fn check_conclusion(review: &ReviewResult) -> &'static str {
    match review.recommendation.as_str() {
        "safe" => "success",
        "warn" => "action_required",
        _ => "failure",
    }
}

fn commit_state(review: &ReviewResult) -> &'static str {
    match review.recommendation.as_str() {
        "safe" => "success",
        "warn" => "pending",
        _ => "failure",
    }
}

fn recommendation_emoji(review: &ReviewResult) -> &'static str {
    match review.recommendation.as_str() {
        "safe" => "🟢",
        "warn" => "🟡",
        _ => "🔴",
    }
}

fn next_move(review: &ReviewResult) -> &'static str {
    match review.recommendation.as_str() {
        "safe" => "This patch is within the current repo rules. Review normally, but TrustGate did not find a reason to stop it.",
        "warn" => "A human should look at the flagged areas before merge. The patch may still be fine, but it no longer looks routine.",
        _ => "Do not move this patch forward yet. The repo rules say the current risk profile is too high without intervention.",
    }
}

fn plaintext_findings(review: &ReviewResult) -> String {
    if review.findings.is_empty() {
        return "TrustGate found no active warnings against the current repo rules.".into();
    }

    review
        .findings
        .iter()
        .take(10)
        .map(|finding| {
            if finding.evidence.is_empty() {
                format!("- [{}] {}: {}", finding.severity, finding.label, finding.detail)
            } else {
                format!(
                    "- [{}] {}: {} ({})",
                    finding.severity,
                    finding.label,
                    finding.detail,
                    finding.evidence.join("; ")
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_findings(review: &ReviewResult) -> String {
    if review.findings.is_empty() {
        return "- No active warnings.\n".into();
    }

    review
        .findings
        .iter()
        .take(8)
        .map(|finding| {
            let evidence = if finding.evidence.is_empty() {
                String::new()
            } else {
                format!(" Evidence: {}.", finding.evidence.join("; "))
            };
            format!(
                "- **{}** (`{}`): {}.{}",
                finding.label, finding.severity, finding.detail, evidence
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_top_files(review: &ReviewResult) -> String {
    let top = review
        .files
        .iter()
        .filter(|file| file.status != "safe" || file.generated || !file.path_policy.is_empty())
        .take(6)
        .map(|file| {
            let mut suffix = Vec::new();
            if file.generated {
                suffix.push("generated".to_string());
            }
            if !file.path_policy.is_empty() {
                suffix.push(file.path_policy.clone());
            }
            let extra = if suffix.is_empty() {
                String::new()
            } else {
                format!(" — {}", suffix.join(" | "))
            };
            format!("- `{}`: **{}**{}", file.path, file.status, extra)
        })
        .collect::<Vec<_>>();

    if top.is_empty() {
        "- No file-level hotspots beyond the current summary.".into()
    } else {
        top.join("\n")
    }
}

fn render_template(template: &str, variables: &[(&str, String)]) -> String {
    let mut rendered = template.to_string();
    for (key, value) in variables {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    rendered.trim().to_string()
}

fn render_github_report(review: &ReviewResult) -> RenderedGitHubReport {
    let (templates, template_scope, template_repo) = report_templates_for_repo(&review.repo);
    let details_url_value = details_url(review).unwrap_or_default();
    let details_markdown = if details_url_value.is_empty() {
        "TrustGate review details are local to the current PatchHive host.".into()
    } else {
        format!("[Open TrustGate review]({details_url_value})")
    };

    let github = review.github.as_ref();
    let variables = vec![
        ("repo", review.repo.clone()),
        (
            "pr_number",
            github
                .map(|value| value.pr_number.to_string())
                .unwrap_or_default(),
        ),
        (
            "pr_title",
            github.map(|value| value.pr_title.clone()).unwrap_or_default(),
        ),
        (
            "base_ref",
            github.map(|value| value.base_ref.clone()).unwrap_or_default(),
        ),
        (
            "head_ref",
            github.map(|value| value.head_ref.clone()).unwrap_or_default(),
        ),
        ("ai_source", review.ai_source.clone()),
        ("source_kind", review.source_kind.clone()),
        ("emoji", recommendation_emoji(review).into()),
        ("recommendation", review.recommendation.clone()),
        ("recommendation_upper", review.recommendation.to_uppercase()),
        ("summary", review.summary.clone()),
        ("risk_score", review.risk_score.to_string()),
        ("files_changed", review.metrics.files_changed.to_string()),
        ("additions", review.metrics.additions.to_string()),
        ("deletions", review.metrics.deletions.to_string()),
        ("tests_changed", review.metrics.tests_changed.to_string()),
        ("generated_files", review.metrics.generated_files.to_string()),
        (
            "blocked_findings",
            review.metrics.blocked_findings.to_string(),
        ),
        (
            "warning_findings",
            review.metrics.warning_findings.to_string(),
        ),
        ("findings_markdown", markdown_findings(review)),
        ("findings_plaintext", plaintext_findings(review)),
        ("file_hotspots_markdown", markdown_top_files(review)),
        ("next_move", next_move(review).into()),
        ("details_markdown", details_markdown),
        ("details_url", details_url_value),
    ];

    let check_title = render_template(&templates.check_title_template, &variables);
    let check_summary = render_template(&templates.check_summary_template, &variables);
    let check_text = render_template(&templates.check_text_template, &variables);
    let comment_body = render_template(&templates.comment_template, &variables);

    RenderedGitHubReport {
        check_title,
        check_summary,
        check_text,
        comment_markdown: format!("{COMMENT_MARKER}\n{comment_body}"),
        template_scope,
        template_repo,
    }
}

pub fn preview_review_outcome(review: &ReviewResult, message: &str) -> GitHubReportOutcome {
    let rendered = render_github_report(review);
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
        report_markdown: rendered.comment_markdown,
        template_scope: rendered.template_scope,
        template_repo: rendered.template_repo,
    }
}

pub async fn publish_review_outcome(client: &Client, review: &ReviewResult) -> GitHubReportOutcome {
    let Some(github) = review.github.as_ref() else {
        return GitHubReportOutcome {
            attempted: false,
            delivered: false,
            method: "none".into(),
            state: "skipped".into(),
            message: "This review was not tied to a GitHub pull request.".into(),
            details: Vec::new(),
            check_url: String::new(),
            status_url: String::new(),
            comment_url: String::new(),
            comment_mode: String::new(),
            report_markdown: String::new(),
            template_scope: String::new(),
            template_repo: String::new(),
        };
    };

    let rendered = render_github_report(review);

    if !github_token_configured() {
        return GitHubReportOutcome {
            attempted: true,
            delivered: false,
            method: "none".into(),
            state: "missing_token".into(),
            message:
                "BOT_GITHUB_TOKEN or GITHUB_TOKEN is required to report TrustGate results back to GitHub."
                    .into(),
            details: vec![
                "PR diff ingestion still works for public repos without a token.".into(),
                "GitHub status/check publishing is disabled until a token is configured.".into(),
            ],
            check_url: String::new(),
            status_url: String::new(),
            comment_url: String::new(),
            comment_mode: String::new(),
            report_markdown: rendered.comment_markdown,
            template_scope: rendered.template_scope,
            template_repo: rendered.template_repo,
        };
    }

    let target_repo = if github.head_repo.trim().is_empty() {
        review.repo.as_str()
    } else {
        github.head_repo.as_str()
    };

    let gh = pr_client(client);
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
                conclusion: check_conclusion(review).into(),
                external_id: review.id.clone(),
                details_url: details_url(review),
                title: rendered.check_title.clone(),
                summary: rendered.check_summary.clone(),
                text: rendered.check_text.clone(),
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
        Err(err) => details.push(format!("Check run failed, falling back to commit status: {err}")),
    }

    if !delivered {
        match gh
            .create_commit_status(
                target_repo,
                GitHubCommitStatusRequest {
                    sha: github.head_sha.clone(),
                    state: commit_state(review).into(),
                    context: STATUS_CONTEXT.into(),
                    description: match review.recommendation.as_str() {
                        "safe" => "TrustGate marked this diff safe.".into(),
                        "warn" => "TrustGate found warnings that need review.".into(),
                        _ => "TrustGate found blocking issues.".into(),
                    },
                    target_url: details_url(review),
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
        .upsert_issue_comment(
            &github.repo,
            github.pr_number,
            COMMENT_MARKER,
            &rendered.comment_markdown,
        )
        .await
    {
        Ok(GitHubManagedCommentResult { mode, html_url }) => {
            comment_mode = mode;
            comment_url = html_url.clone();
            if method == "none" {
                method = "pr_comment".into();
            }
            details.push(if html_url.is_empty() {
                format!("{} TrustGate PR comment.", comment_mode)
            } else {
                format!("{} TrustGate PR comment: {html_url}", comment_mode)
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
            review.recommendation.clone()
        } else {
            "report_failed".into()
        },
        message: if delivered {
            "TrustGate published its review back to GitHub with a maintained PR comment and status signal.".into()
        } else {
            "TrustGate reviewed the PR but could not publish the result back to GitHub.".into()
        },
        details,
        check_url,
        status_url,
        comment_url,
        comment_mode,
        report_markdown: rendered.comment_markdown,
        template_scope: rendered.template_scope,
        template_repo: rendered.template_repo,
    }
}

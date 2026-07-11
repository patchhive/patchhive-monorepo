// github.rs — GitHub PR review orchestration and webhook signature verification

use axum::http::StatusCode;

use patchhive_github_pr::verify_github_webhook_signature;

use crate::github;
use crate::models::{GitHubReviewContext, RepoRuleSet, ReviewResult};

use super::failguard::publish_failguard_candidate;
use super::review::review_diff;
use super::rules::resolve_rules;
use super::types::{api_error, normalize_ai_source, ApiError};

pub struct GitHubPrReviewInput {
    pub repo: String,
    pub pr_number: i64,
    pub ai_source: String,
    pub rules: Option<RepoRuleSet>,
    pub publish_status: bool,
    pub trigger: String,
    pub event: String,
    pub action: String,
}

pub async fn run_github_pr_review(
    client: &reqwest::Client,
    input: GitHubPrReviewInput,
) -> Result<ReviewResult, ApiError> {
    let Some(repo) = crate::db::normalize_repo_name(&input.repo) else {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "TrustGate expects repos in owner/repo format.",
        ));
    };

    if input.pr_number <= 0 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "TrustGate expects a positive pull request number.",
        ));
    }

    let pr = github::fetch_pull_request(client, &repo, input.pr_number)
        .await
        .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;
    let diff = github::fetch_pull_request_diff(client, &repo, input.pr_number)
        .await
        .map_err(|err| api_error(StatusCode::BAD_GATEWAY, err.to_string()))?;

    if diff.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_GATEWAY,
            "GitHub returned an empty pull request diff.",
        ));
    }

    let rules = resolve_rules(&repo, input.rules)?;
    let github_context = GitHubReviewContext {
        repo: repo.clone(),
        head_repo: if pr.head_repo.trim().is_empty() {
            repo.clone()
        } else {
            pr.head_repo.clone()
        },
        pr_number: input.pr_number,
        pr_title: pr.title,
        pr_url: pr.html_url,
        head_sha: pr.head_sha,
        head_ref: pr.head_ref,
        base_ref: pr.base_ref,
        event: input.event,
        action: input.action,
        trigger: input.trigger,
    };

    let mut review = review_diff(
        client,
        &repo,
        &diff,
        &normalize_ai_source(&input.ai_source, "github-pr"),
        rules,
        "github_pr",
        Some(github_context),
    )
    .await;

    review.github_report = Some(if input.publish_status {
        github::publish_review_outcome(client, &review).await
    } else {
        github::preview_review_outcome(
            &review,
            "GitHub status/check publishing was skipped for this run.",
        )
    });

    crate::db::save_review(&review)
        .map_err(|err| api_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    publish_failguard_candidate(client, &review).await;

    Ok(review)
}

pub fn verify_webhook_signature(
    headers: &axum::http::HeaderMap,
    body: &[u8],
) -> Result<(), ApiError> {
    let Some(secret) = github::webhook_secret() else {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Configure TRUST_GITHUB_WEBHOOK_SECRET before enabling the TrustGate GitHub webhook.",
        ));
    };

    verify_github_webhook_signature(headers, body, &secret).map_err(|err| {
        let text = err.to_string();
        let status = if text.contains("Could not initialize") {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::UNAUTHORIZED
        };
        api_error(status, text)
    })
}

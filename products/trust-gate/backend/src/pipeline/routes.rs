// routes.rs — HTTP route handlers

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use patchhive_product_core::contract;
use serde_json::{json, Value};
use std::collections::BTreeSet;

use crate::models::{GitHubPrReviewRequest, ReviewHistoryItem, ReviewRequest, ReviewResult};
use crate::state::AppState;

use super::github::{run_github_pr_review, verify_webhook_signature};
use super::review::review_diff;
use super::rules::{build_rule_packs, resolve_rules};
use super::types::{api_error, normalize_ai_source, ApiError};
use super::failguard::publish_failguard_candidate;

pub async fn capabilities() -> Json<contract::ProductCapabilities> {
    Json(contract::capabilities(
        "trust-gate",
        "TrustGate",
        vec![
            contract::action("review_diff", "Review diff", "POST", "/review",
                "Review a submitted diff against repo-specific safety and policy rules.", true),
            contract::action("review_github_pr", "Review GitHub PR", "POST", "/review/github/pr",
                "Review a GitHub pull request diff against TrustGate rules.", true),
            contract::action("github_webhook", "Receive GitHub webhook", "POST", "/webhooks/github",
                "Process a signed GitHub pull request webhook for diff review.", true),
        ],
        vec![
            contract::link("history", "History", "/history"),
            contract::link("rules", "Rules", "/rules"),
            contract::link("templates", "Templates", "/templates"),
        ],
    ))
}

pub async fn runs() -> Json<contract::ProductRunsResponse> {
    Json(contract::runs_from_history(
        "trust-gate",
        crate::db::list_reviews().unwrap_or_default(),
    ))
}

pub async fn rule_packs() -> Json<Value> {
    Json(json!({ "packs": build_rule_packs() }))
}

pub async fn review(
    State(state): State<AppState>,
    Json(body): Json<ReviewRequest>,
) -> Result<Json<ReviewResult>, ApiError> {
    let Some(repo) = crate::db::normalize_repo_name(&body.repo) else {
        return Err(api_error(axum::http::StatusCode::BAD_REQUEST, "TrustGate expects repos in owner/repo format."));
    };
    if body.diff.trim().is_empty() {
        return Err(api_error(axum::http::StatusCode::BAD_REQUEST, "Paste a unified diff before running TrustGate."));
    }

    let rules = resolve_rules(&repo, body.rules)?;
    let review = review_diff(
        &state.http, &repo, &body.diff,
        &normalize_ai_source(&body.ai_source, "manual"),
        rules, "manual", None,
    ).await;
    crate::db::save_review(&review)
        .map_err(|err| api_error(axum::http::StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    publish_failguard_candidate(&state.http, &review).await;

    Ok(Json(review))
}

pub async fn review_github_pr(
    State(state): State<AppState>,
    Json(body): Json<GitHubPrReviewRequest>,
) -> Result<Json<ReviewResult>, ApiError> {
    let review = run_github_pr_review(
        &state.http, body.repo, body.pr_number, body.ai_source, body.rules,
        body.publish_status, "manual_pr_lookup".into(), "pull_request".into(), "manual".into(),
    ).await?;
    Ok(Json(review))
}

pub async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    verify_webhook_signature(&headers, &body)?;

    let event = headers
        .get("X-GitHub-Event").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let payload: Value = serde_json::from_slice(&body).map_err(|_| {
        api_error(axum::http::StatusCode::BAD_REQUEST, "Could not decode GitHub webhook payload.")
    })?;

    if event != "pull_request" {
        return Ok(Json(json!({
            "triggered": false, "event": event,
            "reason": "TrustGate currently reviews pull_request webhooks only.",
        })));
    }

    let action = payload["action"].as_str().unwrap_or("").to_string();
    let supported = matches!(action.as_str(), "opened" | "reopened" | "synchronize" | "ready_for_review");
    if !supported {
        return Ok(Json(json!({
            "triggered": false, "event": event, "action": action,
            "reason": "This pull_request action does not trigger an automatic TrustGate review.",
        })));
    }

    let repo = payload["repository"]["full_name"].as_str().ok_or_else(|| {
        api_error(axum::http::StatusCode::BAD_REQUEST, "Webhook payload was missing repository.full_name.")
    })?.to_string();
    let pr_number = payload["pull_request"]["number"].as_i64().ok_or_else(|| {
        api_error(axum::http::StatusCode::BAD_REQUEST, "Webhook payload was missing pull_request.number.")
    })?;

    let review = run_github_pr_review(
        &state.http, repo, pr_number, "github-webhook".into(), None,
        true, "github_webhook".into(), event.clone(), action.clone(),
    ).await?;

    Ok(Json(json!({
        "triggered": true, "event": event, "action": action,
        "recommendation": review.recommendation, "review": review,
    })))
}

pub async fn history() -> Result<Json<Value>, ApiError> {
    let reviews = crate::db::list_reviews()
        .map_err(|err| api_error(axum::http::StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Json(json!({ "reviews": reviews })))
}

pub async fn history_detail(Path(id): Path<String>) -> Result<Json<ReviewResult>, ApiError> {
    match crate::db::get_review(&id)
        .map_err(|err| api_error(axum::http::StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
    {
        Some(review) => Ok(Json(review)),
        None => Err(api_error(axum::http::StatusCode::NOT_FOUND, "TrustGate review not found.")),
    }
}

pub fn unique_repos(reviews: &[ReviewHistoryItem]) -> usize {
    reviews.iter().map(|r| r.repo.clone()).collect::<BTreeSet<_>>().len()
}

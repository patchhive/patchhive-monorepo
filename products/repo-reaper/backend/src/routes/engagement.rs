use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use patchhive_product_core::hivecore_policy::{
    check_repository_policy, RepositoryPolicyDecisionRequest,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    db::tracked_pull_request,
    fix_worker::{run_follow_up, FollowUpRequest},
    github::{gh_post, repo_reaper_github_token},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/maintainer-engagements/follow-up", post(follow_up))
        .route("/maintainer-engagements/reply", post(reply))
}

type JsonResult<T> = Result<Json<T>, (StatusCode, Json<Value>)>;

fn api_error(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({"error": {"code": code, "message": message.into()}})),
    )
}

#[derive(Debug, Deserialize)]
pub struct FollowUpActionRequest {
    pub repository: String,
    pub pull_request_number: i64,
    pub pull_request_title: String,
    pub maintainer_message: String,
    pub maintainer_login: String,
}

async fn follow_up(
    State(state): State<AppState>,
    Json(request): Json<FollowUpActionRequest>,
) -> JsonResult<Value> {
    validate_repository(&request.repository)?;
    if request.pull_request_number <= 0 || request.maintainer_message.trim().is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_follow_up",
            "A positive pull-request number and non-empty maintainer message are required.",
        ));
    }
    run_follow_up(
        state,
        FollowUpRequest {
            repo: request.repository,
            pr_number: request.pull_request_number,
            issue_title: request.pull_request_title,
            comment_body: request.maintainer_message,
            comment_author: request.maintainer_login,
        },
    )
    .await
    .map_err(|refusal| api_error(StatusCode::CONFLICT, refusal.reason(), refusal.detail()))?;
    Ok(Json(json!({"status": "accepted"})))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplyArtifactKind {
    PullRequest,
    Issue,
}

#[derive(Debug, Deserialize)]
pub struct ReplyActionRequest {
    pub repository: String,
    pub artifact_kind: ReplyArtifactKind,
    pub number: i64,
    pub body: String,
}

async fn reply(
    State(state): State<AppState>,
    Json(request): Json<ReplyActionRequest>,
) -> JsonResult<Value> {
    validate_repository(&request.repository)?;
    let body = request.body.trim();
    if request.number <= 0 || body.is_empty() || body.len() > 10_000 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_reply",
            "A positive artifact number and reply between 1 and 10000 characters are required.",
        ));
    }

    let owned = match request.artifact_kind {
        ReplyArtifactKind::PullRequest => tracked_pull_request(&request.repository, request.number)
            .map_err(|error| {
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "ownership_unavailable",
                    error.to_string(),
                )
            })?
            .is_some_and(|tracked| tracked.state == "open" && !tracked.merged),
        // RepoReaper currently opens pull requests, not issues. An issue reply
        // becomes available only when an issue-opening product owns and tracks
        // its corresponding response action.
        ReplyArtifactKind::Issue => false,
    };
    if !owned {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "artifact_not_owned",
            "RepoReaper has no durable ownership evidence for this open GitHub artifact.",
        ));
    }
    if crate::db::repository_is_excluded(&request.repository).map_err(|error| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "repository_policy_unavailable",
            error.to_string(),
        )
    })? {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "repository_blocked",
            "Repository policy blocks this reply.",
        ));
    }

    let policy = check_repository_policy(
        &state.http,
        &RepositoryPolicyDecisionRequest {
            repository: request.repository.clone(),
            product: "repo-reaper".into(),
            operation: "maintainer_reply".into(),
        },
    )
    .await
    .map_err(|error| {
        api_error(
            StatusCode::BAD_GATEWAY,
            "policy_unavailable",
            error.to_string(),
        )
    })?;
    if policy.as_ref().is_some_and(|decision| !decision.allowed()) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "repository_blocked",
            "Repository policy blocks this reply.",
        ));
    }

    let token = repo_reaper_github_token().ok_or_else(|| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "github_write_not_configured",
            "RepoReaper's GitHub write credential is not configured.",
        )
    })?;
    let attributed = format!(
        "{body}\n\n---\nResponse from **RepoReaper by [PatchHive](https://github.com/patchhive)**, approved through Tendwright's maintainer-engagement workflow."
    );
    let response = gh_post(
        &state.http,
        &format!(
            "/repos/{}/issues/{}/comments",
            request.repository, request.number
        ),
        &json!({"body": attributed}),
        Some(&token),
    )
    .await
    .map_err(|error| {
        api_error(
            StatusCode::BAD_GATEWAY,
            "github_reply_failed",
            error.to_string(),
        )
    })?;
    let comment_url = response["html_url"]
        .as_str()
        .filter(|url| url.starts_with("https://github.com/"))
        .ok_or_else(|| {
            api_error(
                StatusCode::BAD_GATEWAY,
                "github_reply_evidence_invalid",
                "GitHub accepted the request without returning a canonical comment URL; do not replay the consumed approval.",
            )
        })?;
    Ok(Json(json!({
        "status": "replied",
        "comment_url": comment_url,
    })))
}

fn validate_repository(repository: &str) -> Result<(), (StatusCode, Json<Value>)> {
    if patchhive_product_core::scope_policy::normalize_repo_name(repository).as_deref()
        != Some(repository)
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "invalid_repository",
            "Repository must use normalized owner/repository form.",
        ));
    }
    Ok(())
}

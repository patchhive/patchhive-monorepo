use anyhow::{anyhow, Result};
use patchhive_github_pr::{github_token_from_env, GitHubPrClient};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

const GH_API: &str = "https://api.github.com";

fn bot_token() -> String {
    github_token_from_env().unwrap_or_default()
}

fn pr_client(http: &Client, token: Option<&str>) -> GitHubPrClient {
    GitHubPrClient::new(
        http.clone(),
        token
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(github_token_from_env),
        "repo-reaper/0.1",
    )
}

fn gh_headers(token: Option<&str>) -> reqwest::header::HeaderMap {
    let mut h = reqwest::header::HeaderMap::new();
    let tok = token.map(|s| s.to_string()).unwrap_or_else(bot_token);
    if !tok.is_empty() {
        h.insert("Authorization", format!("Bearer {tok}").parse().unwrap());
    }
    h.insert("Accept", "application/vnd.github+json".parse().unwrap());
    h.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
    h.insert("User-Agent", "repo-reaper/0.1".parse().unwrap());
    h
}

pub async fn gh_get(
    http: &Client,
    path: &str,
    params: &[(&str, &str)],
    token: Option<&str>,
) -> Result<Value> {
    let resp = http
        .get(format!("{GH_API}{path}"))
        .headers(gh_headers(token))
        .query(params)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        return Err(anyhow!("GitHub GET {path} -> {status}"));
    }
    Ok(resp.json().await?)
}

pub async fn gh_post(
    http: &Client,
    path: &str,
    body: &Value,
    token: Option<&str>,
) -> Result<Value> {
    let resp = http
        .post(format!("{GH_API}{path}"))
        .headers(gh_headers(token))
        .json(body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let accepted_permissions = resp
            .headers()
            .get("x-accepted-github-permissions")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let oauth_scopes = resp
            .headers()
            .get("x-oauth-scopes")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let text = resp.text().await.unwrap_or_default();
        let mut detail = format!("GitHub POST {path} -> {status}: {text}");
        if let Some(permissions) = accepted_permissions {
            detail.push_str(&format!("; accepted-permissions={permissions}"));
        }
        if let Some(scopes) = oauth_scopes {
            detail.push_str(&format!("; token-scopes={scopes}"));
        }
        return Err(anyhow!(detail));
    }
    Ok(resp.json().await?)
}

pub async fn gh_patch(
    http: &Client,
    path: &str,
    body: &Value,
    token: Option<&str>,
) -> Result<Value> {
    let resp = http
        .patch(format!("{GH_API}{path}"))
        .headers(gh_headers(token))
        .json(body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let accepted_permissions = resp
            .headers()
            .get("x-accepted-github-permissions")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let oauth_scopes = resp
            .headers()
            .get("x-oauth-scopes")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let text = resp.text().await.unwrap_or_default();
        let mut detail = format!("GitHub PATCH {path} -> {status}: {text}");
        if let Some(permissions) = accepted_permissions {
            detail.push_str(&format!("; accepted-permissions={permissions}"));
        }
        if let Some(scopes) = oauth_scopes {
            detail.push_str(&format!("; token-scopes={scopes}"));
        }
        return Err(anyhow!(detail));
    }
    Ok(resp.json().await?)
}

pub async fn gh_delete(http: &Client, path: &str, token: Option<&str>) -> Result<()> {
    let resp = http
        .delete(format!("{GH_API}{path}"))
        .headers(gh_headers(token))
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "gh_delete {path}: HTTP {} — {}",
            status.as_u16(),
            body.chars().take(200).collect::<String>(),
        );
    }
    Ok(())
}

pub async fn gh_fork(
    http: &Client,
    repo: &str,
    token: Option<&str>,
    bot_user: Option<&str>,
) -> Result<Value> {
    let user = bot_user
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::env::var("BOT_GITHUB_USER").unwrap_or_default());
    let tok = token.map(|s| s.to_string()).unwrap_or_else(bot_token);
    let repo_name = repo.split('/').nth(1).unwrap_or(repo);

    let expected = format!("{user}/{repo_name}");
    let fork_request = gh_post(
        http,
        &format!("/repos/{repo}/forks"),
        &serde_json::json!({}),
        token,
    )
    .await;
    let fork_request_full_name = match fork_request {
        Ok(fork) => fork["full_name"].as_str().map(str::to_string),
        Err(error) => {
            if let Ok(fork) = gh_get(http, &format!("/repos/{expected}"), &[], Some(&tok)).await {
                if fork["full_name"].is_string() {
                    return Ok(fork);
                }
            }
            return Err(anyhow!(
                "Fork request failed: {repo}; expected fork {expected}; {error}"
            ));
        }
    };

    let mut delay = Duration::from_secs(1);
    for attempt in 0..5 {
        sleep(delay).await;
        if let Ok(fork) = gh_get(http, &format!("/repos/{user}/{repo_name}"), &[], Some(&tok)).await
        {
            if fork["full_name"].is_string() {
                return Ok(fork);
            }
        }
        delay = delay.saturating_mul(2); // 1s, 2s, 4s, 8s, 16s
        tracing::debug!(
            "gh_fork {repo}: attempt {}/5 not ready, retrying in {delay:?}",
            attempt + 1
        );
    }
    let mut detail = format!("Fork timed out: {repo}; expected fork {expected} was not visible");
    if let Some(full_name) = fork_request_full_name {
        detail.push_str(&format!("; fork request returned {full_name}"));
    }
    Err(anyhow!(detail))
}

pub async fn gh_check_duplicate(
    http: &Client,
    repo: &str,
    branch: &str,
    bot_user: Option<&str>,
    token: Option<&str>,
) -> bool {
    let user = bot_user
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::env::var("BOT_GITHUB_USER").unwrap_or_default());
    let repo_name = repo.split('/').nth(1).unwrap_or(repo);
    let head = format!("{user}:{branch}");

    let prs = gh_get(
        http,
        &format!("/repos/{repo}/pulls"),
        &[("state", "open"), ("head", &head)],
        token,
    )
    .await;
    if let Ok(v) = prs {
        if v.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            return true;
        }
    }

    let branches = gh_get(
        http,
        &format!("/repos/{user}/{repo_name}/branches"),
        &[],
        token,
    )
    .await;
    if let Ok(v) = branches {
        if v.as_array()
            .into_iter()
            .flatten()
            .any(|b| b["name"].as_str() == Some(branch))
        {
            return true;
        }
    }
    false
}

pub async fn gh_comment_issue(
    http: &Client,
    repo: &str,
    number: i64,
    body: &str,
    token: Option<&str>,
) {
    let _ = gh_post(
        http,
        &format!("/repos/{repo}/issues/{number}/comments"),
        &serde_json::json!({"body": body}),
        token,
    )
    .await;
}

fn markdown_code_field(body: &str, label: &str) -> Option<String> {
    let needle = format!("**{label}:** `");
    let value_start = body.find(&needle)? + needle.len();
    let rest = &body[value_start..];
    let value_end = rest.find('`')?;
    Some(rest[..value_end].trim().to_string())
}

fn repo_reaper_issue_comment_stage(body: &str) -> &'static str {
    if body.contains("**Status:** attempting fix") {
        "attempt"
    } else {
        "outcome"
    }
}

fn repo_reaper_issue_marker(
    repo: &str,
    number: i64,
    stage: &str,
    run: &str,
    attempt: &str,
) -> String {
    format!("<!-- patchhive:repo-reaper:{repo}#{number}:{stage}:{run}:{attempt} -->")
}

const REPO_REAPER_OLD_COMMENT_FOOTER: &str = "Generated autonomously by **RepoReaper by [PatchHive](https://github.com/patchhive)**. This managed comment is updated instead of posting a new status comment on each retry.";
const REPO_REAPER_TIMELINE_COMMENT_FOOTER: &str = "Generated autonomously by **RepoReaper by [PatchHive](https://github.com/patchhive)**. This managed comment records RepoReaper status updates for this issue instead of posting a new comment on each retry.";
const REPO_REAPER_COMMENT_FOOTER: &str = "Generated autonomously by **RepoReaper by [PatchHive](https://github.com/patchhive)**. RepoReaper posts progress and outcome updates separately so maintainers can follow the run.";

fn is_repo_reaper_managed_comment(comment: &Value, marker: &str, stage: &str) -> bool {
    let body = comment["body"].as_str().unwrap_or("");
    if body.contains(marker) {
        return true;
    }
    let author = comment["user"]["login"].as_str().unwrap_or("");
    stage == "attempt"
        && author == "patchhive"
        && body.contains("RepoReaper")
        && body.contains("hunting this bug")
}

fn clean_repo_reaper_comment_body(body: &str, marker: &str) -> String {
    body.replace(marker, "")
        .replace(REPO_REAPER_OLD_COMMENT_FOOTER, "")
        .replace(REPO_REAPER_TIMELINE_COMMENT_FOOTER, "")
        .replace(REPO_REAPER_COMMENT_FOOTER, "")
        .trim()
        .to_string()
}

fn repo_reaper_managed_comment_body(marker: &str, body: &str) -> String {
    format!(
        "{marker}\n{}\n\n{REPO_REAPER_COMMENT_FOOTER}",
        clean_repo_reaper_comment_body(body, marker)
    )
}

pub async fn gh_upsert_repo_reaper_issue_comment(
    http: &Client,
    repo: &str,
    number: i64,
    body: &str,
    token: Option<&str>,
) -> Result<Value> {
    let stage = repo_reaper_issue_comment_stage(body);
    let run = markdown_code_field(body, "Run").unwrap_or_else(|| "unknown-run".to_string());
    let attempt =
        markdown_code_field(body, "Attempt").unwrap_or_else(|| "unknown-attempt".to_string());
    let marker = repo_reaper_issue_marker(repo, number, stage, &run, &attempt);
    let managed_body = repo_reaper_managed_comment_body(&marker, body);
    let comments = gh_get(
        http,
        &format!("/repos/{repo}/issues/{number}/comments"),
        &[("per_page", "100")],
        token,
    )
    .await?;

    if let Some(existing) = comments
        .as_array()
        .into_iter()
        .flatten()
        .find(|comment| is_repo_reaper_managed_comment(comment, &marker, stage))
    {
        let id = existing["id"]
            .as_i64()
            .ok_or_else(|| anyhow!("Existing RepoReaper issue comment was missing an id"))?;
        return gh_patch(
            http,
            &format!("/repos/{repo}/issues/comments/{id}"),
            &serde_json::json!({ "body": managed_body }),
            token,
        )
        .await;
    }

    gh_post(
        http,
        &format!("/repos/{repo}/issues/{number}/comments"),
        &serde_json::json!({ "body": managed_body }),
        token,
    )
    .await
}

pub async fn gh_get_issue_context(
    http: &Client,
    repo: &str,
    number: i64,
    token: Option<&str>,
) -> String {
    let Ok(comments) = gh_get(
        http,
        &format!("/repos/{repo}/issues/{number}/comments"),
        &[("per_page", "20")],
        token,
    )
    .await
    else {
        return String::new();
    };
    comments
        .as_array()
        .into_iter()
        .flatten()
        .take(10)
        .map(|c| {
            format!(
                "**@{}**: {}",
                c["user"]["login"].as_str().unwrap_or("?"),
                &c["body"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(600)
                    .collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub async fn gh_check_rate_limit(http: &Client, token: Option<&str>) -> Value {
    let Ok(data) = gh_get(http, "/rate_limit", &[], token).await else {
        return serde_json::json!({"remaining":-1,"limit":5000,"reset_in":0});
    };
    let core = &data["resources"]["core"];
    let remaining = core["remaining"].as_i64().unwrap_or(0);
    let limit = core["limit"].as_i64().unwrap_or(5000);
    let reset = core["reset"].as_i64().unwrap_or(0);
    let reset_in = (reset - chrono::Utc::now().timestamp()).max(0);
    serde_json::json!({
        "remaining": remaining, "limit": limit,
        "reset_at": reset, "reset_in": reset_in,
        "pct_used": (100.0 * (1.0 - remaining as f64 / limit.max(1) as f64)) as i64,
    })
}

pub async fn gh_poll_pr(http: &Client, repo: &str, pr_number: i64, token: Option<&str>) -> Value {
    let Ok(pr) = pr_client(http, token)
        .fetch_pull_request(repo, pr_number)
        .await
    else {
        return serde_json::json!({"state":"unknown","merged":false});
    };
    let reviews = gh_get(
        http,
        &format!("/repos/{repo}/pulls/{pr_number}/reviews"),
        &[],
        token,
    )
    .await
    .unwrap_or_default();
    let review_state = reviews
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .find(|r| {
            matches!(
                r["state"].as_str(),
                Some("APPROVED") | Some("CHANGES_REQUESTED") | Some("COMMENTED")
            )
        })
        .and_then(|r| r["state"].as_str())
        .unwrap_or("")
        .to_string();
    serde_json::json!({
        "state": pr.state, "merged": pr.merged, "draft": pr.draft,
        "review_state": review_state, "title": pr.title, "url": pr.html_url,
    })
}

pub async fn gh_delete_branch(
    http: &Client,
    repo: &str,
    branch: &str,
    bot_user: Option<&str>,
    token: Option<&str>,
) {
    let user = bot_user
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::env::var("BOT_GITHUB_USER").unwrap_or_default());
    let repo_name = repo.split('/').nth(1).unwrap_or(repo);
    let _ = gh_delete(
        http,
        &format!("/repos/{user}/{repo_name}/git/refs/heads/{branch}"),
        token,
    )
    .await;
}

pub async fn gh_default_branch(http: &Client, repo: &str, token: Option<&str>) -> Option<String> {
    gh_get(http, &format!("/repos/{repo}"), &[], token)
        .await
        .ok()?
        .get("default_branch")?
        .as_str()
        .map(|s| s.to_string())
}

pub async fn gh_pr_base_branch(
    http: &Client,
    repo: &str,
    pr_number: i64,
    token: Option<&str>,
) -> Option<String> {
    let pr = pr_client(http, token)
        .fetch_pull_request(repo, pr_number)
        .await
        .ok()?;
    if pr.base_ref.trim().is_empty() {
        None
    } else {
        Some(pr.base_ref)
    }
}

pub async fn search_repos(http: &Client, query: &str, max_repos: usize) -> Result<Vec<Value>> {
    // GitHub Search API caps per_page at 100. Paginate when max_repos exceeds the cap.
    let mut all_items = Vec::new();
    let per_page = max_repos.min(100);
    let pages_needed = (max_repos + per_page - 1) / per_page;

    for page in 1..=pages_needed {
        let data = gh_get(
            http,
            "/search/repositories",
            &[
                ("q", query),
                ("sort", "updated"),
                ("per_page", &per_page.to_string()),
                ("page", &page.to_string()),
            ],
            None,
        )
        .await?;
        if let Some(items) = data["items"].as_array() {
            all_items.extend(items.clone());
        }
        // Stop early if GitHub returned fewer items than requested (last page).
        if data["items"]
            .as_array()
            .map_or(true, |a| a.len() < per_page)
        {
            break;
        }
    }
    all_items.truncate(max_repos);
    Ok(all_items)
}

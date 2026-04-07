use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

const GH_API: &str = "https://api.github.com";

fn bot_token() -> String { std::env::var("BOT_GITHUB_TOKEN").unwrap_or_default() }
fn bot_user()  -> String { std::env::var("BOT_GITHUB_USER").unwrap_or_default() }

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

pub async fn gh_get(http: &Client, path: &str, params: &[(&str, &str)], token: Option<&str>) -> Result<Value> {
    let resp = http.get(format!("{GH_API}{path}"))
        .headers(gh_headers(token))
        .query(params)
        .send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        return Err(anyhow!("GitHub GET {path} -> {status}"));
    }
    Ok(resp.json().await?)
}

pub async fn gh_post(http: &Client, path: &str, body: &Value, token: Option<&str>) -> Result<Value> {
    let resp = http.post(format!("{GH_API}{path}"))
        .headers(gh_headers(token))
        .json(body)
        .send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("GitHub POST {path} -> {status}: {text}"));
    }
    Ok(resp.json().await?)
}

pub async fn gh_delete(http: &Client, path: &str, token: Option<&str>) -> Result<()> {
    http.delete(format!("{GH_API}{path}"))
        .headers(gh_headers(token))
        .send().await?;
    Ok(())
}

pub async fn gh_fork(http: &Client, repo: &str, token: Option<&str>, bot_user: Option<&str>) -> Result<Value> {
    let user = bot_user
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::env::var("BOT_GITHUB_USER").unwrap_or_default());
    let tok = token.map(|s| s.to_string()).unwrap_or_else(bot_token);
    let repo_name = repo.split('/').nth(1).unwrap_or(repo);

    gh_post(http, &format!("/repos/{repo}/forks"), &serde_json::json!({}), token).await.ok();

    for _ in 0..20 {
        sleep(Duration::from_secs(4)).await;
        if let Ok(fork) = gh_get(http, &format!("/repos/{user}/{repo_name}"), &[], Some(&tok)).await {
            if fork["full_name"].is_string() { return Ok(fork); }
        }
    }
    Err(anyhow!("Fork timed out: {repo}"))
}

pub async fn gh_check_duplicate(http: &Client, repo: &str, branch: &str, bot_user: Option<&str>, token: Option<&str>) -> bool {
    let user = bot_user
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::env::var("BOT_GITHUB_USER").unwrap_or_default());
    let repo_name = repo.split('/').nth(1).unwrap_or(repo);
    let head = format!("{user}:{branch}");

    let prs = gh_get(http, &format!("/repos/{repo}/pulls"), &[("state","open"),("head",&head)], token).await;
    if let Ok(v) = prs { if v.as_array().map(|a| !a.is_empty()).unwrap_or(false) { return true; } }

    let branches = gh_get(http, &format!("/repos/{user}/{repo_name}/branches"), &[], token).await;
    if let Ok(v) = branches {
        if v.as_array().into_iter().flatten().any(|b| b["name"].as_str() == Some(branch)) { return true; }
    }
    false
}

pub async fn gh_comment_issue(http: &Client, repo: &str, number: i64, body: &str, token: Option<&str>) {
    let _ = gh_post(http, &format!("/repos/{repo}/issues/{number}/comments"), &serde_json::json!({"body": body}), token).await;
}

pub async fn gh_get_issue_context(http: &Client, repo: &str, number: i64, token: Option<&str>) -> String {
    let Ok(comments) = gh_get(http, &format!("/repos/{repo}/issues/{number}/comments"), &[("per_page","20")], token).await else { return String::new() };
    comments.as_array().into_iter().flatten()
        .take(10)
        .map(|c| format!("**@{}**: {}", c["user"]["login"].as_str().unwrap_or("?"), &c["body"].as_str().unwrap_or("").chars().take(600).collect::<String>()))
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
    let Ok(pr) = gh_get(http, &format!("/repos/{repo}/pulls/{pr_number}"), &[], token).await else {
        return serde_json::json!({"state":"unknown","merged":false});
    };
    let reviews = gh_get(http, &format!("/repos/{repo}/pulls/{pr_number}/reviews"), &[], token).await.unwrap_or_default();
    let review_state = reviews
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .find(|r| matches!(r["state"].as_str(), Some("APPROVED") | Some("CHANGES_REQUESTED") | Some("COMMENTED")))
        .and_then(|r| r["state"].as_str())
        .unwrap_or("")
        .to_string();
    serde_json::json!({
        "state": pr["state"], "merged": pr["merged"], "draft": pr["draft"],
        "review_state": review_state, "title": pr["title"], "url": pr["html_url"],
    })
}

pub async fn gh_delete_branch(http: &Client, repo: &str, branch: &str, bot_user: Option<&str>, token: Option<&str>) {
    let user = bot_user
        .map(|s| s.to_string())
        .unwrap_or_else(|| std::env::var("BOT_GITHUB_USER").unwrap_or_default());
    let repo_name = repo.split('/').nth(1).unwrap_or(repo);
    let _ = gh_delete(http, &format!("/repos/{user}/{repo_name}/git/refs/heads/{branch}"), token).await;
}

pub async fn gh_default_branch(http: &Client, repo: &str, token: Option<&str>) -> Option<String> {
    gh_get(http, &format!("/repos/{repo}"), &[], token)
        .await
        .ok()?
        .get("default_branch")?
        .as_str()
        .map(|s| s.to_string())
}

pub async fn gh_pr_base_branch(http: &Client, repo: &str, pr_number: i64, token: Option<&str>) -> Option<String> {
    gh_get(http, &format!("/repos/{repo}/pulls/{pr_number}"), &[], token)
        .await
        .ok()?
        .get("base")?
        .get("ref")?
        .as_str()
        .map(|s| s.to_string())
}

pub async fn search_repos(http: &Client, query: &str, max_repos: usize) -> Result<Vec<Value>> {
    let data = gh_get(http, "/search/repositories", &[("q", query), ("sort","updated"), ("per_page", &max_repos.to_string())], None).await?;
    Ok(data["items"].as_array().cloned().unwrap_or_default())
}

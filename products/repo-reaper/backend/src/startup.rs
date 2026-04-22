use crate::db::*;
use patchhive_github_pr::github_token_from_env;
use patchhive_product_core::{repo_memory::repo_memory_url, startup::StartupCheck};
use reqwest::Client;

pub async fn validate_config(http: &Client) -> Vec<StartupCheck> {
    let mut results = Vec::new();
    let ai_local_url = crate::ai_local::configured_url();

    let required = [
        ("BOT_GITHUB_TOKEN", "GitHub PAT (repo + PR scopes)"),
        ("BOT_GITHUB_USER", "GitHub bot username"),
    ];
    for (key, desc) in required {
        if std::env::var(key).unwrap_or_default().is_empty() {
            results.push(StartupCheck::error(format!(
                "Missing {key} ({desc}) — set in .env or Config panel"
            )));
        } else {
            results.push(StartupCheck::ok(format!("{key} is set")));
        }
    }

    if std::env::var("PROVIDER_API_KEY")
        .unwrap_or_default()
        .is_empty()
    {
        if ai_local_url.is_some() {
            results.push(StartupCheck::ok(
                "PATCHHIVE_AI_URL is set — OpenAI-compatible agents can use the local Codex/Copilot gateway",
            ));
            results.push(StartupCheck::warn(
                "No PROVIDER_API_KEY set — Anthropic, Gemini, and Groq agents still need per-agent or global keys",
            ));
        } else {
            results.push(StartupCheck::warn(
                "No PROVIDER_API_KEY set — each agent must carry its own key",
            ));
        }
    }

    // Validate GitHub token
    let token = github_token_from_env().unwrap_or_default();
    if !token.is_empty() {
        match http
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "repo-reaper/0.1")
            .send()
            .await
        {
            Ok(r) if r.status() == 401 => {
                results.push(StartupCheck::error(
                    "BOT_GITHUB_TOKEN is invalid or expired",
                ));
            }
            Ok(r) if r.status().is_success() => {
                let data: serde_json::Value = r.json().await.unwrap_or_default();
                let login = data["login"].as_str().unwrap_or("?");
                results.push(StartupCheck::ok(format!(
                    "GitHub token valid — authenticated as @{login}"
                )));
            }
            Ok(r) => {
                results.push(StartupCheck::warn(format!(
                    "GitHub returned {}",
                    r.status()
                )));
            }
            Err(e) => {
                results.push(StartupCheck::warn(format!(
                    "Could not validate GitHub token: {e}"
                )));
            }
        }
    }

    if ai_local_url.is_some() {
        let status = crate::ai_local::fetch_status(http).await;
        if status["ok"].as_bool().unwrap_or(false) {
            let ready: Vec<String> = status["providers"]
                .as_object()
                .map(|providers| {
                    providers
                        .iter()
                        .filter(|(_, data)| {
                            data["ok"].as_bool().unwrap_or(false)
                                && data["logged_in"].as_bool().unwrap_or(false)
                        })
                        .map(|(name, _)| name.clone())
                        .collect()
                })
                .unwrap_or_default();
            if ready.is_empty() {
                results.push(StartupCheck::warn(
                    "PatchHive AI gateway is reachable, but no local providers are authenticated yet",
                ));
            } else {
                results.push(StartupCheck::ok(format!(
                    "PatchHive AI gateway reachable — ready providers: {}",
                    ready.join(", ")
                )));
            }
        } else {
            results.push(StartupCheck::warn(format!(
                "PATCHHIVE_AI_URL is set, but the local AI gateway is not ready: {}",
                status["error"].as_str().unwrap_or("unknown error")
            )));
        }
    }

    if repo_memory_url().is_some() {
        results.push(StartupCheck::info(
            "PATCHHIVE_REPO_MEMORY_URL is set — RepoReaper can enrich patch generation and queue FailGuard candidates when Smith rejects work",
        ));
    }

    if std::env::var("WEBHOOK_SECRET")
        .unwrap_or_default()
        .is_empty()
    {
        results.push(StartupCheck::warn(
            "WEBHOOK_SECRET is not set — the /webhook/github endpoint will reject webhook delivery until it is configured",
        ));
    } else {
        results.push(StartupCheck::ok(
            "WEBHOOK_SECRET is set — GitHub webhook signatures will be verified",
        ));
    }

    results
}

pub fn recover_orphaned() -> Vec<String> {
    crate::db::recover_orphaned_runs()
}

pub async fn pr_poll_loop(http: Client) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(4 * 3600)).await;
        poll_all_prs(&http).await;
    }
}

async fn poll_all_prs(http: &Client) {
    let prs: Vec<(i64, String, String)> = {
        let Ok(conn) = get_conn() else { return };
        conn.prepare(
            "SELECT pr_number, repo, run_id FROM pr_tracking WHERE state != 'closed' AND merged = 0"
        ).ok().and_then(|mut s| {
            let mapped = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).ok()?;
            Some(mapped.flatten().collect())
        })
         .unwrap_or_default()
    };

    for (pr_number, repo, run_id) in prs {
        let state = crate::github::gh_poll_pr(http, &repo, pr_number, None).await;
        let merged = state["merged"].as_bool().unwrap_or(false);
        if let Ok(conn) = get_conn() {
            let _ = conn.execute(
                "UPDATE pr_tracking SET state=?1, merged=?2, review_state=?3, last_checked=?4 WHERE pr_number=?5 AND repo=?6",
                rusqlite::params![
                    state["state"].as_str().unwrap_or("open"),
                    merged as i32,
                    state["review_state"].as_str(),
                    chrono::Utc::now().to_rfc3339(),
                    pr_number, repo,
                ],
            );
        }
        if merged {
            let issue_number: Option<i64> = get_conn()
                .ok()
                .and_then(|conn| {
                    conn.query_row(
                        "SELECT issue_number FROM issue_attempts WHERE run_id=?1 AND pr_number=?2 LIMIT 1",
                        rusqlite::params![run_id, pr_number],
                        |r| r.get(0),
                    ).ok()
                });
            let branch = format!("reaper/issue-{}", issue_number.unwrap_or(pr_number));
            crate::github::gh_delete_branch(http, &repo, &branch, None, None).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

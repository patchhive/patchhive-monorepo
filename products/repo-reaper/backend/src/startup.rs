use crate::db::get_conn;
use patchhive_product_core::{
    github_auth::{verify_github_write_token, REPO_REAPER_GITHUB_TOKEN_RW},
    github_permissions::GitHubPermissionProfile,
    hivecore_policy::hivecore_url,
    repo_memory::repo_memory_url,
    startup::{StartupCheck, StartupCheckLevel},
};
use reqwest::Client;

pub async fn validate_config(http: &Client) -> Vec<StartupCheck> {
    let mut results = Vec::new();
    let ai_local_url = crate::ai_local::configured_url();

    let required = [("BOT_GITHUB_USER", "GitHub bot username")];
    for (key, desc) in required {
        if std::env::var(key).unwrap_or_default().trim().is_empty() {
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

    let github_profile = GitHubPermissionProfile::AutonomousWrite;
    match verify_github_write_token(http, REPO_REAPER_GITHUB_TOKEN_RW).await {
        Ok(_) => results.push(github_profile.ready_check()),
        Err(err) => results.push(
            github_profile.validation_failed_check(err.to_string(), StartupCheckLevel::Error),
        ),
    }

    match std::env::var("REAPER_MAX_ACTIVE_WORKERS") {
        Ok(raw) => match raw.trim().parse::<usize>() {
            Ok(limit) if (1..=128).contains(&limit) => results.push(StartupCheck::ok(format!(
                "Process-wide patch/test worker capacity is {limit}"
            ))),
            _ => results.push(StartupCheck::error(
                "REAPER_MAX_ACTIVE_WORKERS must be an integer between 1 and 128",
            )),
        },
        Err(_) => results.push(StartupCheck::ok(
            "Process-wide patch/test worker capacity uses the default of 3",
        )),
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
    } else {
        results.push(StartupCheck::info(
            "RepoMemory is not configured — RepoReaper will skip automatic FailGuard candidate submission and promoted preflight constraints until PATCHHIVE_REPO_MEMORY_URL is set",
        ));
    }

    if let Some(url) = hivecore_url() {
        results.push(StartupCheck::ok(format!(
            "HiveCore repository policy and PR-budget enforcement is configured at {url}. RepoReaper will fail closed when that control plane is unavailable."
        )));
    } else {
        results.push(StartupCheck::info(
            "HiveCore repository policy and PR-budget enforcement is not configured. RepoReaper remains in standalone mode until PATCHHIVE_HIVECORE_URL is set.",
        ));
    }

    let encryption_secret = ["REAPER_ENCRYPTION_KEY", "PATCHHIVE_ENCRYPTION_KEY"]
        .iter()
        .find_map(|name| {
            std::env::var(name)
                .ok()
                .map(|value| ((*name).to_string(), value.trim().to_string()))
                .filter(|(_, value)| !value.is_empty())
        });
    if let Some((name, secret)) = encryption_secret {
        match patchhive_product_core::secrets::validate_encryption_secret(&secret) {
            Ok(()) => results.push(StartupCheck::ok(format!(
                "{name} is configured with sufficient key material; active-team secrets can be encrypted at rest."
            ))),
            Err(error) => results.push(StartupCheck::error(format!(
                "{name} is not safe encryption key material: {error}"
            ))),
        }
    } else {
        results.push(StartupCheck::warn(
            "REAPER_ENCRYPTION_KEY or PATCHHIVE_ENCRYPTION_KEY is not set. Active agent teams can persist, but per-agent API keys and bot token overrides remain memory-only and will not survive backend restarts.",
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
        let state_label = state["state"].as_str().unwrap_or("open");
        if let Ok(conn) = get_conn() {
            let _ = conn.execute(
                "UPDATE pr_tracking SET state=?1, merged=?2, review_state=?3, last_checked=?4 WHERE pr_number=?5 AND repo=?6",
                rusqlite::params![
                    state_label,
                    merged as i32,
                    state["review_state"].as_str(),
                    chrono::Utc::now().to_rfc3339(),
                    pr_number, repo,
                ],
            );
        }
        if merged || state_label.eq_ignore_ascii_case("closed") {
            let release = patchhive_product_core::hivecore_policy::release_pr_slots_for_run(
                http,
                &patchhive_product_core::hivecore_policy::PrRunReleaseRequest {
                    product: "repo-reaper".into(),
                    run_id: run_id.clone(),
                    reason: format!(
                        "RepoReaper observed pull request #{pr_number} as {}.",
                        if merged { "merged" } else { "closed" }
                    ),
                },
            )
            .await;
            if let Err(error) = release {
                tracing::warn!(
                    run_id,
                    repository = repo,
                    pr_number,
                    "could not release HiveCore PR budget after closure: {error}"
                );
            }
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

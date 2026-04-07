use axum::{
    extract::{State, Request},
    body::Body,
    http::StatusCode,
    Json,
    routing::{delete, get, post, patch},
    Router,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use chrono::{Utc, Duration};
use crate::db::{finish_run, get_conn, start_run};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/schedules",          get(list_schedules).post(create_schedule))
        .route("/schedules/:id",      delete(delete_schedule))
        .route("/schedules/:id/toggle", patch(toggle_schedule))
        .route("/webhook/github",     post(github_webhook))
}

fn cron_next(expr: &str) -> chrono::DateTime<Utc> {
    match expr {
        "hourly" => Utc::now() + Duration::hours(1),
        "nightly" => (Utc::now() + Duration::days(1)).date_naive().and_hms_opt(0,0,0)
            .map(|dt| dt.and_utc()).unwrap_or_else(|| Utc::now() + Duration::days(1)),
        "weekly" => Utc::now() + Duration::weeks(1),
        _ => Utc::now() + Duration::hours(24),
    }
}

async fn list_schedules(State(_): State<AppState>) -> Json<Value> {
    let Ok(conn) = get_conn() else { return Json(json!({"schedules":[]})) };
    let rows: Vec<Value> = conn.prepare("SELECT id,cron_expr,config_json,enabled,last_run,next_run FROM scheduled_runs ORDER BY next_run").ok()
        .and_then(|mut s| {
            let mapped = s.query_map([], |r| Ok(json!({
                "id":r.get::<_,String>(0)?,"cron_expr":r.get::<_,String>(1)?,"config_json":r.get::<_,String>(2)?,
                "enabled":r.get::<_,i32>(3)?,"last_run":r.get::<_,Option<String>>(4)?,"next_run":r.get::<_,String>(5)?,
            }))).ok()?;
            Some(mapped.flatten().collect())
        })
    .unwrap_or_default();
    Json(json!({"schedules": rows}))
}

#[derive(Deserialize)]
struct ScheduleCreate { cron_expr: String, config_json: Value, #[serde(default="yes")] enabled: bool }
fn yes() -> bool { true }

async fn create_schedule(State(_): State<AppState>, Json(body): Json<ScheduleCreate>) -> Json<Value> {
    let id = Uuid::new_v4().to_string()[..8].to_string();
    let nxt = cron_next(&body.cron_expr).to_rfc3339();
    let Ok(conn) = get_conn() else { return Json(json!({"error":"db"})) };
    let _ = conn.execute(
        "INSERT INTO scheduled_runs(id,cron_expr,config_json,enabled,next_run) VALUES(?1,?2,?3,?4,?5)",
        rusqlite::params![id, body.cron_expr, body.config_json.to_string(), body.enabled as i32, nxt],
    );
    Json(json!({"id": id, "next_run": nxt}))
}

async fn delete_schedule(State(_): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    let Ok(conn) = get_conn() else { return Json(json!({"ok":false})) };
    let _ = conn.execute("DELETE FROM scheduled_runs WHERE id=?1", [&id]);
    Json(json!({"ok": true}))
}

async fn toggle_schedule(State(_): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    let Ok(conn) = get_conn() else { return Json(json!({"error":"db"})) };
    let enabled: i32 = conn.query_row("SELECT enabled FROM scheduled_runs WHERE id=?1", [&id], |r| r.get(0)).unwrap_or(0);
    let new_val = if enabled == 0 { 1i32 } else { 0i32 };
    let _ = conn.execute("UPDATE scheduled_runs SET enabled=?1 WHERE id=?2", rusqlite::params![new_val, id]);
    Json(json!({"enabled": new_val == 1}))
}

// ── Webhook handler ────────────────────────────────────────────────────────────

async fn github_webhook(State(state): State<AppState>, req: Request<Body>) -> Result<Json<Value>, StatusCode> {
    let event = req.headers().get("X-GitHub-Event").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let signature = req.headers().get("X-Hub-Signature-256").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let body_bytes = axum::body::to_bytes(req.into_body(), 1024 * 1024).await.unwrap_or_default();

    // Verify signature
    let secret = std::env::var("WEBHOOK_SECRET").unwrap_or_default();
    if !secret.is_empty() {
        let sig = signature.ok_or(StatusCode::UNAUTHORIZED)?;
        let sig_hex = sig.strip_prefix("sha256=").ok_or(StatusCode::UNAUTHORIZED)?;
        let sig_bytes = hex::decode(sig_hex).map_err(|_| StatusCode::UNAUTHORIZED)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        mac.update(&body_bytes);
        mac.verify_slice(&sig_bytes).map_err(|_| StatusCode::UNAUTHORIZED)?;
    }

    let payload: Value = serde_json::from_slice(&body_bytes).unwrap_or_default();

    if event == "issues" && payload["action"].as_str() == Some("opened") {
        let issue = &payload["issue"];
        let labels: Vec<&str> = issue["labels"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|l| l["name"].as_str())
            .collect();

        if labels.contains(&"bug") && state.watch_mode.load(std::sync::atomic::Ordering::SeqCst) {
            let repo = payload["repository"]["full_name"].as_str().unwrap_or("").to_string();
            let issue_num = issue["number"].as_i64().unwrap_or(0);
            let state_clone = state.clone();
            let issue_clone = issue.clone();
            tokio::spawn(async move {
                webhook_single_fix(state_clone, &repo, issue_clone).await;
            });
            return Ok(Json(json!({"triggered":true,"type":"new_bug_issue","issue":issue_num,"watch_mode":true})));
        }
        return Ok(Json(json!({"triggered":false,"reason":"watch_mode_disabled"})));
    }

    if event == "issue_comment" && payload["action"].as_str() == Some("created") {
        let issue = &payload["issue"];
        let comment = &payload["comment"];
        let bot = std::env::var("BOT_GITHUB_USER").unwrap_or_default();
        if issue["pull_request"].is_object() && comment["user"]["login"].as_str() != Some(&bot) {
            let repo = payload["repository"]["full_name"].as_str().unwrap_or("").to_string();
            let state_clone = state.clone();
            let issue_c = issue.clone();
            let comment_c = comment.clone();
            tokio::spawn(async move {
                webhook_pr_comment(state_clone, &repo, issue_c, comment_c).await;
            });
            return Ok(Json(json!({"triggered":true,"type":"pr_comment"})));
        }
    }

    Ok(Json(json!({"triggered":false,"event":event})))
}

async fn webhook_single_fix(state: AppState, repo: &str, issue: Value) {
    let agents_snap = state.agents.read().await.clone();
    if agents_snap.is_empty() { return; }

    let scouts:     Vec<_> = agents_snap.values().filter(|a| a.role == "scout").cloned().collect();
    let judges:     Vec<_> = agents_snap.values().filter(|a| a.role == "judge").cloned().collect();
    let reapers:    Vec<_> = agents_snap.values().filter(|a| a.role == "reaper").cloned().collect();
    let smiths:     Vec<_> = agents_snap.values().filter(|a| a.role == "smith").cloned().collect();
    let gatekeepers:Vec<_> = agents_snap.values().filter(|a| a.role == "gatekeeper").cloned().collect();
    let reaper_list = if reapers.is_empty() { scouts.clone() } else { reapers };
    let gatekeeper_list = if gatekeepers.is_empty() { reaper_list.clone() } else { gatekeepers };

    let run_id = Uuid::new_v4().to_string()[..12].to_string();
    let run_cost = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(0));
    let min_conf = std::env::var("MIN_REVIEW_CONFIDENCE").ok().and_then(|s| s.parse().ok()).unwrap_or(40);
    let retry_count: usize = std::env::var("RETRY_COUNT").ok().and_then(|s| s.parse().ok()).unwrap_or(3);

    let iss = json!({
        "id": issue["id"], "number": issue["number"],
        "title": issue["title"], "body": issue["body"].as_str().unwrap_or("").chars().take(500).collect::<String>(),
        "labels": ["bug"], "comments": 0, "created": issue.get("created_at"),
        "url": issue.get("html_url"), "repo": repo, "repo_url": "",
        "status": "queued", "fixability_score": 70, "fixability_reason": "webhook",
    });

    let _ = start_run(&run_id, &json!({"source":"webhook","repo":repo,"issue":issue["number"]}), false);
    let (tx, _rx) = tokio::sync::mpsc::channel(32); // fire-and-forget channel
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(1));

    use crate::fix_worker::{fix_one, FixParams};
    let params = FixParams { retry_count, min_conf, budget: 0.0, run_id: run_id.clone() };
    fix_one(iss, 0, judges, reaper_list, smiths, gatekeeper_list, sem, params, run_cost.clone(), tx, state.http.clone()).await;

    let Ok(conn) = get_conn() else { return };
    let total_fixed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM issue_attempts WHERE run_id=?1 AND status='fixed'",
        [&run_id],
        |r| r.get(0),
    ).unwrap_or(0);
    let total_attempted: i64 = conn.query_row(
        "SELECT COUNT(*) FROM issue_attempts WHERE run_id=?1",
        [&run_id],
        |r| r.get(0),
    ).unwrap_or(0);
    let rc = run_cost.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1_000_000.0;
    let _ = finish_run(&run_id, total_fixed, total_attempted, rc, "done");
}

async fn webhook_pr_comment(state: AppState, repo: &str, issue: Value, comment: Value) {
    use crate::agents::agent_pr_comment_fix;
    use crate::github::{gh_fork, gh_comment_issue, gh_post};
    use crate::git_ops::{git_clone, git_branch, git_commit_push, collect_files_all, apply_patch};

    let agents_snap = state.agents.read().await.clone();
    let Some(reaper) = agents_snap.values().find(|a| a.role == "reaper").or_else(|| agents_snap.values().next()).cloned() else { return };

    let bot_token = reaper.bot_token.as_deref().map(|s| s.to_string()).unwrap_or_else(|| std::env::var("BOT_GITHUB_TOKEN").unwrap_or_default());
    let bot_user  = reaper.bot_user.as_deref().map(|s| s.to_string()).unwrap_or_else(|| std::env::var("BOT_GITHUB_USER").unwrap_or_default());
    let pr_number = issue["number"].as_i64().unwrap_or(0);
    let branch = format!("reaper/followup-{pr_number}");
    let work_dir = std::path::PathBuf::from(format!("/tmp/repo-reaper/followup-{pr_number}"));

    let Ok(fork) = gh_fork(&state.http, repo, Some(&bot_token), Some(&bot_user)).await else { return };
    if work_dir.exists() { let _ = tokio::fs::remove_dir_all(&work_dir).await; }
    if git_clone(fork["clone_url"].as_str().unwrap_or(""), &work_dir, Some(&bot_user), Some(&bot_token)).await.is_err() { return; }
    if git_branch(&work_dir, &branch).await.is_err() { return; }

    let codebase = collect_files_all(&work_dir, 60_000);
    let Ok((result, _)) = agent_pr_comment_fix(&state.http, issue["title"].as_str().unwrap_or(""), comment["body"].as_str().unwrap_or(""), &codebase, &reaper).await else { return };
    let Some(patch) = result["patch"].as_str() else { return };

    let (applied, _) = apply_patch(&work_dir, patch).await;
    if !applied { return; }

    let msg = format!("fix: follow-up based on maintainer feedback (re #{})", pr_number);
    if git_commit_push(&work_dir, &branch, &msg).await.is_err() { return; }

    let base_branch = if let Some(branch_name) =
        crate::github::gh_pr_base_branch(&state.http, repo, pr_number, Some(&bot_token)).await
    {
        branch_name
    } else {
        crate::github::gh_default_branch(&state.http, repo, Some(&bot_token))
            .await
            .unwrap_or_else(|| "main".to_string())
    };

    let _ = gh_post(&state.http, &format!("/repos/{repo}/pulls"), &json!({
        "title": msg,
        "body": format!("Follow-up fix based on maintainer feedback on #{}.\n\n**Maintainer:** {}\n\n**What changed:** {}\n\n*RepoReaper by PatchHive*",
            pr_number, comment["body"].as_str().unwrap_or("").chars().take(500).collect::<String>(), result["explanation"].as_str().unwrap_or("")),
        "head": format!("{bot_user}:{branch}"), "base": base_branch, "draft": false,
    }), Some(&bot_token)).await;

    gh_comment_issue(&state.http, repo, pr_number, "🔱 RepoReaper opened a follow-up PR based on your feedback. *by PatchHive*", Some(&bot_token)).await;
    if work_dir.exists() { let _ = tokio::fs::remove_dir_all(&work_dir).await; }
}

// ── Background scheduler ───────────────────────────────────────────────────────

pub async fn scheduler_loop(state: AppState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let now = Utc::now().to_rfc3339();
        let Ok(conn) = get_conn() else { continue };
        let due: Vec<(String, String, String)> = conn.prepare(
            "SELECT id, cron_expr, config_json FROM scheduled_runs WHERE enabled=1 AND next_run<=?"
        ).ok().and_then(|mut s| {
            let mapped = s.query_map([&now], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).ok()?;
            Some(mapped.flatten().collect())
        }).unwrap_or_default();

        for (id, cron, config_json) in due {
            let nxt = cron_next(&cron).to_rfc3339();
            let _ = conn.execute(
                "UPDATE scheduled_runs SET last_run=?1, next_run=?2 WHERE id=?3",
                rusqlite::params![now, nxt, id],
            );
            if let Ok(cfg) = serde_json::from_str::<Value>(&config_json) {
                let state_clone = state.clone();
                tokio::spawn(async move {
                    if let Ok(req) = serde_json::from_value::<crate::pipeline::RunRequest>(cfg) {
                        let _ = crate::pipeline::run(axum::extract::State(state_clone), axum::Json(req)).await;
                        tracing::info!("Scheduled run {id} triggered");
                    } else {
                        tracing::warn!("Scheduled run {id} has invalid config_json");
                    }
                });
            }
        }
    }
}

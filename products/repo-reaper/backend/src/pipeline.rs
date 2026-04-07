use axum::{extract::State, response::sse::{Event, KeepAlive, Sse}};
use std::convert::Infallible;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, atomic::{AtomicBool, AtomicI64, Ordering}};
use tokio::sync::{mpsc, Semaphore};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::agents::*;
use crate::db::*;
use crate::fix_worker::{fix_one, sse, alog, astatus, FixParams};
use crate::github::*;
use crate::state::{AgentConfig, AppState};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RunRequest {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_min_stars")]
    pub min_stars: u32,
    #[serde(default = "default_max_repos")]
    pub max_repos: usize,
    #[serde(default = "default_max_issues")]
    pub max_issues: usize,
    #[serde(default = "default_labels")]
    pub labels: Vec<String>,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default)]
    pub search_query: String,
    #[serde(default)]
    pub cost_budget_usd: f64,
    #[serde(default = "default_retry_count")]
    pub retry_count: usize,
}

fn default_language()    -> String       { "python".into() }
fn default_min_stars()   -> u32          { 50 }
fn default_max_repos()   -> usize        { 10 }
fn default_max_issues()  -> usize        { 10 }
fn default_labels()      -> Vec<String>  { vec!["bug".into()] }
fn default_concurrency() -> usize        { 3 }
fn default_retry_count() -> usize        { 3 }

fn cfg(k: &str) -> String { std::env::var(k).unwrap_or_default() }

fn load_lists() -> (
    std::collections::HashSet<String>,
    std::collections::HashSet<String>,
    std::collections::HashSet<String>,
) {
    let Ok(conn) = get_conn() else { return Default::default() };
    let rows: Vec<(String, String)> = conn.prepare("SELECT repo, list_type FROM repo_lists").ok()
        .and_then(|mut s| {
            let mapped = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).ok()?;
            Some(mapped.flatten().collect())
        })
        .unwrap_or_default();
    let allow: std::collections::HashSet<_> = rows.iter().filter(|(_, t)| t == "allowlist").map(|(r, _)| r.clone()).collect();
    let deny: std::collections::HashSet<_> = rows.iter()
        .filter(|(_, t)| t == "denylist" || t == "blocklist")
        .map(|(r, _)| r.clone())
        .collect();
    let opt_out: std::collections::HashSet<_> = rows.iter()
        .filter(|(_, t)| t == "opt_out")
        .map(|(r, _)| r.clone())
        .collect();
    (allow, deny, opt_out)
}

async fn discover(
    http: &reqwest::Client, req: &RunRequest, scout: &AgentConfig,
    allowlist: &std::collections::HashSet<String>,
    denylist: &std::collections::HashSet<String>,
    opt_out: &std::collections::HashSet<String>,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
) -> (Vec<Value>, Vec<Value>) {
    let query = if !req.search_query.is_empty() { req.search_query.clone() }
        else { format!("topic:machine-learning language:{} stars:>{} is:public", req.language, req.min_stars) };

    let mut repos = search_repos(http, &query, req.max_repos).await.unwrap_or_default();
    if !allowlist.is_empty() { repos.retain(|r| allowlist.contains(r["full_name"].as_str().unwrap_or(""))); }
    if !denylist.is_empty() { repos.retain(|r| !denylist.contains(r["full_name"].as_str().unwrap_or(""))); }
    if !opt_out.is_empty() { repos.retain(|r| !opt_out.contains(r["full_name"].as_str().unwrap_or(""))); }

    let _ = tx.send(sse("repos", json!({"repos": repos.iter().map(|r| json!({
        "id": r["id"], "full_name": r["full_name"], "description": r["description"],
        "stars": r["stargazers_count"], "language": r["language"],
        "url": r["html_url"], "open_issues": r["open_issues_count"],
    })).collect::<Vec<_>>()}))).await;

    let mut all_issues = Vec::new();
    for repo in &repos {
        let full_name = repo["full_name"].as_str().unwrap_or("");
        let labels = req.labels.join(",");
        match gh_get(http, &format!("/repos/{full_name}/issues"), &[("state","open"),("labels",&labels),("per_page","5")], None).await {
            Ok(items) => {
                for iss in items.as_array().into_iter().flatten() {
                    if iss["pull_request"].is_object() { continue; }
                    all_issues.push(json!({
                        "id": iss["id"], "number": iss["number"], "title": iss["title"],
                        "body": iss["body"].as_str().unwrap_or("").chars().take(500).collect::<String>(),
                        "labels": iss["labels"].as_array().into_iter().flatten().filter_map(|l| l["name"].as_str()).collect::<Vec<_>>(),
                        "comments": iss["comments"], "created": iss["created_at"],
                        "url": iss["html_url"], "repo": full_name, "repo_url": repo["html_url"],
                        "status": "queued", "fixability_score": 50, "fixability_reason": "",
                    }));
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            Err(e) => { let _ = tx.send(alog(scout, &format!("Skipped {full_name}: {e}"), "warn")).await; }
        }
    }
    (repos, all_issues)
}

pub async fn dry_run(State(state): State<AppState>, axum::Json(req): axum::Json<RunRequest>) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel(128);
    let http = state.http.clone();
    let agents = state.agents.clone();

    tokio::spawn(async move {
        let agents_snap = agents.read().await.clone();
        let scouts: Vec<_> = agents_snap.values().filter(|a| a.role == "scout").cloned().collect();
        let Some(scout) = scouts.first().or_else(|| agents_snap.values().next()) else {
            let _ = tx.send(sse("log", json!({"msg":"No agents configured","type":"error"}))).await;
            return;
        };
        let scout = scout.clone();
        let (allow, deny, opt_out) = load_lists();

        let _ = tx.send(sse("phase", json!({"phase":"scan"}))).await;
        let _ = tx.send(alog(&scout, "[DRY STALK] Scanning — no reaping will happen", "info")).await;

        let (repos, mut issues) = discover(&http, &req, &scout, &allow, &deny, &opt_out, &tx).await;

        if !issues.is_empty() {
            let _ = agent_score_issues(&http, &mut issues, &scout).await;
        }

        let fixable = issues.iter().take(req.max_issues).cloned().collect::<Vec<_>>();
        let _ = tx.send(sse("issues", json!({"issues": issues}))).await;
        let _ = tx.send(alog(&scout, &format!("[DRY STALK] Would reap {} issues — 0 changes made", fixable.len()), "success")).await;

        if let Ok((report, _)) = agent_dry_run_analysis(&http, &fixable, &repos, &scout).await {
            let _ = tx.send(sse("dry_run_report", json!({"report": report}))).await;
        }

        let _ = tx.send(sse("done", json!({"dry_run": true, "total_would_reap": fixable.len()}))).await;
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

pub async fn run(State(state): State<AppState>, axum::Json(req): axum::Json<RunRequest>) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel(256);
    let http = state.http.clone();
    let agents_arc = state.agents.clone();
    let run_active = state.run_active.clone();

    tokio::spawn(async move {
        if run_active.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            let _ = tx.send(sse("error", json!({"msg":"A hunt is already active"}))).await;
            return;
        }

        let run_id = Uuid::new_v4().to_string()[..12].to_string();
        let run_cost = Arc::new(AtomicI64::new(0));
        let budget = req.cost_budget_usd.max(cfg("COST_BUDGET_USD").parse().unwrap_or(0.0));
        let min_conf = cfg("MIN_REVIEW_CONFIDENCE").parse().unwrap_or(40i32);

        let agents_snap = agents_arc.read().await.clone();
        if agents_snap.is_empty() {
            let _ = tx.send(sse("log", json!({"msg":"No agents configured","type":"error"}))).await;
            let _ = tx.send(sse("done", json!({"total_fixed":0}))).await;
            run_active.store(false, Ordering::SeqCst);
            return;
        }

        let scouts:     Vec<_> = agents_snap.values().filter(|a| a.role == "scout").cloned().collect();
        let judges:     Vec<_> = agents_snap.values().filter(|a| a.role == "judge").cloned().collect();
        let reapers:    Vec<_> = agents_snap.values().filter(|a| a.role == "reaper").cloned().collect();
        let smiths:     Vec<_> = agents_snap.values().filter(|a| a.role == "smith").cloned().collect();
        let gatekeepers:Vec<_> = agents_snap.values().filter(|a| a.role == "gatekeeper").cloned().collect();

        let scout_list: Vec<_> = if scouts.is_empty() { agents_snap.values().take(1).cloned().collect() } else { scouts };
        let reaper_list = if reapers.is_empty() { scout_list.clone() } else { reapers };
        let gatekeeper_list = if gatekeepers.is_empty() { reaper_list.clone() } else { gatekeepers };
        let scout = scout_list[0].clone();

        let (allow, deny, opt_out) = load_lists();
        let _ = start_run(&run_id, &serde_json::to_value(&req).unwrap_or_default(), false);

        let _ = tx.send(sse("phase", json!({"phase":"scan"}))).await;
        let _ = tx.send(astatus(&scout.id, "working", "Hunting")).await;

        let (repos, mut all_issues) = discover(&http, &req, &scout, &allow, &deny, &opt_out, &tx).await;
        let _ = tx.send(alog(&scout, &format!("{} repos, {} bugs found", repos.len(), all_issues.len()), "success")).await;

        let _ = tx.send(sse("phase", json!({"phase":"triage"}))).await;
        let _ = tx.send(astatus(&scout.id, "working", "Judging issues")).await;

        if !all_issues.is_empty() {
            match agent_score_issues(&http, &mut all_issues, &scout).await {
                Ok(c) => { run_cost.fetch_add((c * 1_000_000.0) as i64, Ordering::Relaxed); }
                Err(e) => { let _ = tx.send(alog(&scout, &format!("Scoring failed: {e}"), "warn")).await; }
            }
        }

        let fixable: Vec<_> = all_issues.iter().take(req.max_issues).cloned().collect();
        let _ = tx.send(sse("issues", json!({"issues": all_issues}))).await;
        let _ = tx.send(alog(&scout, &format!("Queued {}/{} for reaping", fixable.len(), all_issues.len()), "success")).await;
        let _ = tx.send(astatus(&scout.id, "idle", "")).await;

        if fixable.is_empty() {
            let rc = run_cost.load(Ordering::Relaxed) as f64 / 1_000_000.0;
            let _ = finish_run(&run_id, 0, 0, rc, "done");
            let _ = tx.send(sse("done", json!({"total_fixed":0}))).await;
            run_active.store(false, Ordering::SeqCst);
            return;
        }

        let _ = tx.send(sse("phase", json!({"phase":"fix"}))).await;
        let sem = Arc::new(Semaphore::new(req.concurrency));
        let (done_tx, mut done_rx) = mpsc::channel::<()>(fixable.len());

        let total = fixable.len();
        let mut handles = Vec::new();

        for (idx, issue) in fixable.iter().enumerate() {
            let params = FixParams {
                retry_count: req.retry_count,
                min_conf, budget,
                run_id: run_id.clone(),
            };
            let handle = tokio::spawn(fix_one(
                issue.clone(), idx,
                judges.clone(), reaper_list.clone(),
                smiths.clone(), gatekeeper_list.clone(),
                sem.clone(), params,
                run_cost.clone(), tx.clone(), http.clone(),
            ));
            let dtx = done_tx.clone();
            handles.push(tokio::spawn(async move {
                handle.await.ok();
                let _ = dtx.send(()).await;
            }));
        }
        drop(done_tx);

        let mut completed = 0;
        while let Some(()) = done_rx.recv().await {
            completed += 1;
            let rc = run_cost.load(Ordering::Relaxed) as f64 / 1_000_000.0;
            let _ = tx.send(sse("cost_update", json!({"run_cost": rc, "lifetime_cost": get_lifetime_cost()}))).await;
            if budget > 0.0 && rc >= budget {
                let _ = tx.send(sse("log", json!({"msg":format!("Budget ${budget:.2} reached — stopping"),"type":"warn"}))).await;
                break;
            }
            if completed == total { break; }
        }

        for h in handles { h.abort(); }

        let Ok(conn) = get_conn() else { run_active.store(false, Ordering::SeqCst); return };
        let total_fixed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM issue_attempts WHERE run_id=? AND status='fixed'",
            [&run_id], |r| r.get(0)
        ).unwrap_or(0);
        let rc = run_cost.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let _ = finish_run(&run_id, total_fixed, fixable.len() as i64, rc, "done");

        let _ = tx.send(sse("phase", json!({"phase":"done"}))).await;
        let _ = tx.send(sse("log", json!({"msg":format!("Hunt complete — {total_fixed}/{} kills | ${rc:.4}",fixable.len()),"type":"success"}))).await;
        let _ = tx.send(sse("done", json!({"total_fixed":total_fixed,"total_attempted":fixable.len(),"run_id":run_id,"cost":rc}))).await;

        run_active.store(false, Ordering::SeqCst);
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

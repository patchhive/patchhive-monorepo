use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use patchhive_product_core::scope_policy::{
    normalize_repo_name, RepoScopeDecision, RepoScopePolicy,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::{
    atomic::{AtomicBool, AtomicI64, Ordering},
    Arc,
};
use tokio::sync::{mpsc, Semaphore};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::agents::{agent_dry_run_analysis, agent_score_issues};
use crate::db::{
    finish_run, get_conn, get_lifetime_cost, record_run_artifact, save_dry_stalk_run, start_run,
    RunArtifactInput, RunStart, RunStatus,
};
use crate::fix_worker::{
    alog, astatus, fix_one, sse, FixAgentPools, FixIssueJob, FixParams, FixRunContext,
};
use crate::github::{gh_get, search_repos};
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
    pub target_repo: String,
    #[serde(default)]
    pub cost_budget_usd: f64,
    #[serde(default = "default_retry_count")]
    pub retry_count: usize,
}

fn default_language() -> String {
    "python".into()
}
fn default_min_stars() -> u32 {
    50
}
fn default_max_repos() -> usize {
    10
}
fn default_max_issues() -> usize {
    10
}
fn default_labels() -> Vec<String> {
    vec!["bug".into()]
}
fn default_concurrency() -> usize {
    3
}
fn default_retry_count() -> usize {
    3
}

fn cfg(k: &str) -> String {
    std::env::var(k).unwrap_or_default()
}

fn persist_run_phase(run_id: &str, phase: &str, message: &str) {
    if let Err(error) = record_run_artifact(RunArtifactInput {
        run_id,
        attempt_id: None,
        phase,
        kind: "run.phase",
        status: "running",
        message,
        metadata: None,
    }) {
        tracing::warn!(run_id, phase, "could not persist run phase: {error:#}");
    }
}

fn normalized_target_repo(req: &RunRequest) -> Option<String> {
    normalize_repo_name(&req.target_repo)
}

struct RunTeam {
    scout: AgentConfig,
    judges: Vec<AgentConfig>,
    reapers: Vec<AgentConfig>,
    smiths: Vec<AgentConfig>,
    gatekeepers: Vec<AgentConfig>,
}

struct FixWaveInput<'a> {
    state: &'a AppState,
    req: &'a RunRequest,
    tx: &'a mpsc::Sender<Result<Event, Infallible>>,
    team: &'a RunTeam,
    fixable: &'a [Value],
    run_id: &'a str,
    run_cost: &'a Arc<AtomicI64>,
    cancel_requested: &'a Arc<AtomicBool>,
    budget: f64,
    min_conf: i32,
}

fn load_filters() -> RepoScopePolicy {
    let Ok(conn) = get_conn() else {
        return Default::default();
    };
    let rows: Vec<(String, String)> = conn
        .prepare("SELECT repo, list_type FROM repo_reaper_repo_lists")
        .ok()
        .and_then(|mut s| {
            let mapped = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).ok()?;
            Some(mapped.flatten().collect())
        })
        .unwrap_or_default();
    RepoScopePolicy::from_entries(rows)
}

fn select_run_team(
    agents_snap: &std::collections::HashMap<String, AgentConfig>,
) -> Option<RunTeam> {
    if agents_snap.is_empty() {
        return None;
    }

    let scouts: Vec<_> = agents_snap
        .values()
        .filter(|a| a.role == "scout")
        .cloned()
        .collect();
    let fallback: Vec<_> = if scouts.is_empty() {
        agents_snap.values().take(1).cloned().collect()
    } else {
        scouts
    };
    let scout = fallback.first()?.clone();
    let reapers: Vec<_> = agents_snap
        .values()
        .filter(|a| a.role == "reaper")
        .cloned()
        .collect();
    let reaper_list = if reapers.is_empty() {
        fallback.clone()
    } else {
        reapers
    };
    let gatekeepers: Vec<_> = agents_snap
        .values()
        .filter(|a| a.role == "gatekeeper")
        .cloned()
        .collect();

    Some(RunTeam {
        scout,
        judges: agents_snap
            .values()
            .filter(|a| a.role == "judge")
            .cloned()
            .collect(),
        reapers: reaper_list.clone(),
        smiths: agents_snap
            .values()
            .filter(|a| a.role == "smith")
            .cloned()
            .collect(),
        gatekeepers: if gatekeepers.is_empty() {
            reaper_list
        } else {
            gatekeepers
        },
    })
}

async fn emit_no_agents(tx: &mpsc::Sender<Result<Event, Infallible>>) {
    let _ = tx
        .send(sse(
            "log",
            json!({"msg":"No agents configured","type":"error"}),
        ))
        .await;
}

async fn score_discovered_issues(
    http: &reqwest::Client,
    issues: &mut [Value],
    scout: &AgentConfig,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
    run_cost: Option<&Arc<AtomicI64>>,
) -> bool {
    if issues.is_empty() {
        return true;
    }

    match agent_score_issues(http, issues, scout).await {
        Ok(cost) => {
            if let Some(run_cost) = run_cost {
                run_cost.fetch_add((cost * 1_000_000.0) as i64, Ordering::Relaxed);
            }
            true
        }
        Err(e) => {
            let _ = tx
                .send(alog(scout, &format!("Scoring failed: {e}"), "warn"))
                .await;
            false
        }
    }
}

async fn collect_targets(
    http: &reqwest::Client,
    req: &RunRequest,
    scout: &AgentConfig,
    filters: &RepoScopePolicy,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
    run_cost: Option<&Arc<AtomicI64>>,
) -> (Vec<Value>, Vec<Value>, Vec<Value>, bool) {
    let (repos, mut issues) = discover(http, req, scout, filters, tx).await;

    let scoring_available = score_discovered_issues(http, &mut issues, scout, tx, run_cost).await;
    let fixable = issues
        .iter()
        .take(req.max_issues)
        .cloned()
        .collect::<Vec<_>>();

    (repos, issues, fixable, scoring_available)
}

async fn emit_queued_targets(
    tx: &mpsc::Sender<Result<Event, Infallible>>,
    scout: &AgentConfig,
    repos: &[Value],
    all_issues: &[Value],
    fixable: &[Value],
) {
    let _ = tx.send(sse("issues", json!({"issues": all_issues}))).await;
    let _ = tx
        .send(alog(
            scout,
            &format!("{} repos, {} bugs found", repos.len(), all_issues.len()),
            "success",
        ))
        .await;
    let _ = tx
        .send(alog(
            scout,
            &format!("Queued {}/{} for reaping", fixable.len(), all_issues.len()),
            "success",
        ))
        .await;
}

async fn finalize_run_with_summary(
    tx: &mpsc::Sender<Result<Event, Infallible>>,
    run_id: &str,
    run_cost: &Arc<AtomicI64>,
    attempted: usize,
) {
    let (total_fixed, failed_attempts, nonfixed_attempt_cost): (i64, i64, f64) = {
        let Ok(conn) = get_conn() else {
            return;
        };
        conn
            .query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN status='fixed' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status IN ('error', 'failed', 'rejected', 'skipped') THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status!='fixed' THEN cost_usd ELSE 0 END), 0.0)
             FROM repo_reaper_issue_attempts WHERE run_id=?",
            [run_id],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, f64>(2)?)),
        ).unwrap_or((0, 0, 0.0))
    };

    let rc = (run_cost.load(Ordering::Relaxed) as f64 / 1_000_000.0) + nonfixed_attempt_cost;
    let run_status = if attempted == 0 || total_fixed == attempted as i64 {
        RunStatus::Done
    } else if total_fixed > 0 {
        RunStatus::Partial
    } else {
        RunStatus::Failed
    };
    let _ = finish_run(run_id, total_fixed, attempted as i64, rc, run_status);
    let status = run_status.as_str();
    let log_type = match run_status {
        RunStatus::Done => "success",
        RunStatus::Partial => "warn",
        RunStatus::Failed => "error",
    };
    let summary = match run_status {
        RunStatus::Done => format!("Hunt complete — {total_fixed}/{attempted} kills | ${rc:.4}"),
        RunStatus::Partial => format!("Hunt partial — {total_fixed}/{attempted} kills, {failed_attempts} failed or held | ${rc:.4}"),
        RunStatus::Failed => format!("Hunt failed — {total_fixed}/{attempted} kills, {failed_attempts} failed or held | ${rc:.4}"),
    };
    let _ = tx.send(sse("phase", json!({"phase": status}))).await;
    let _ = tx
        .send(sse("log", json!({"msg": summary, "type": log_type})))
        .await;
    let _ = tx
        .send(sse(
            "done",
            json!({
                "failed_attempts": failed_attempts,
                "status": status,
                "total_fixed": total_fixed,
                "total_attempted": attempted,
                "run_id": run_id,
                "cost": rc
            }),
        ))
        .await;
}

async fn run_fix_wave(input: FixWaveInput<'_>) {
    let FixWaveInput {
        state,
        req,
        tx,
        team,
        fixable,
        run_id,
        run_cost,
        cancel_requested,
        budget,
        min_conf,
    } = input;
    let sem = Arc::new(Semaphore::new(req.concurrency));
    let (done_tx, mut done_rx) = mpsc::channel::<()>(fixable.len());
    let mut handles = Vec::new();
    let context = FixRunContext {
        agents: Arc::new(FixAgentPools {
            judges: team.judges.clone(),
            reapers: team.reapers.clone(),
            smiths: team.smiths.clone(),
            gatekeepers: team.gatekeepers.clone(),
        }),
        sem,
        process_sem: state.process_worker_semaphore.clone(),
        params: FixParams {
            retry_count: req.retry_count,
            min_conf,
            run_id: run_id.to_string(),
            cancel_requested: cancel_requested.clone(),
        },
        run_cost: run_cost.clone(),
        tx: tx.clone(),
        http: state.http.clone(),
    };

    for (idx, issue) in fixable.iter().enumerate() {
        let handle = tokio::spawn(fix_one(FixIssueJob {
            issue: issue.clone(),
            idx,
            context: context.clone(),
        }));
        let done_tx = done_tx.clone();
        handles.push(tokio::spawn(async move {
            handle.await.ok();
            let _ = done_tx.send(()).await;
        }));
    }
    drop(done_tx);

    let total = fixable.len();
    let mut completed = 0;
    while let Some(()) = done_rx.recv().await {
        completed += 1;
        let rc = run_cost.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let _ = tx
            .send(sse(
                "cost_update",
                json!({"run_cost": rc, "lifetime_cost": get_lifetime_cost()}),
            ))
            .await;
        if budget > 0.0 && rc >= budget && !cancel_requested.load(Ordering::SeqCst) {
            cancel_requested.store(true, Ordering::SeqCst);
            let _ = tx.send(sse("log", json!({"msg":format!("Budget ${budget:.2} reached — finishing in-flight work and cancelling new hunts"),"type":"warn"}))).await;
        }
        if completed == total {
            break;
        }
    }

    for handle in handles {
        let _ = handle.await;
    }
}

async fn discover(
    http: &reqwest::Client,
    req: &RunRequest,
    scout: &AgentConfig,
    filters: &RepoScopePolicy,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
) -> (Vec<Value>, Vec<Value>) {
    if let Some(target_repo) = normalized_target_repo(req) {
        return discover_target_repo(http, req, scout, &target_repo, filters, tx).await;
    }

    if !req.target_repo.trim().is_empty() {
        let _ = tx
            .send(alog(
                scout,
                "Target repo must use owner/repo format; no autonomous hunt was started",
                "warn",
            ))
            .await;
        return (Vec::new(), Vec::new());
    }

    let query = if !req.search_query.is_empty() {
        req.search_query.clone()
    } else {
        format!(
            "topic:machine-learning language:{} stars:>{} is:public",
            req.language, req.min_stars
        )
    };

    let mut repos = search_repos(http, &query, req.max_repos)
        .await
        .unwrap_or_default();
    repos.retain(|repo| filters.allows(repo["full_name"].as_str().unwrap_or("")));

    let _ = tx
        .send(sse(
            "repos",
            json!({"repos": repos.iter().map(|r| json!({
        "id": r["id"], "full_name": r["full_name"], "description": r["description"],
        "stars": r["stargazers_count"], "language": r["language"],
        "url": r["html_url"], "open_issues": r["open_issues_count"],
    })).collect::<Vec<_>>()}),
        ))
        .await;

    let mut all_issues = Vec::new();
    for repo in &repos {
        let full_name = repo["full_name"].as_str().unwrap_or("");
        let labels = req.labels.join(",");
        match gh_get(
            http,
            &format!("/repos/{full_name}/issues"),
            &[("state", "open"), ("labels", &labels), ("per_page", "5")],
            None,
        )
        .await
        {
            Ok(items) => {
                for iss in items.as_array().into_iter().flatten() {
                    if iss["pull_request"].is_object() {
                        continue;
                    }
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
            Err(e) => {
                let _ = tx
                    .send(alog(scout, &format!("Skipped {full_name}: {e}"), "warn"))
                    .await;
            }
        }
    }
    (repos, all_issues)
}

async fn discover_target_repo(
    http: &reqwest::Client,
    req: &RunRequest,
    scout: &AgentConfig,
    target_repo: &str,
    filters: &RepoScopePolicy,
    tx: &mpsc::Sender<Result<Event, Infallible>>,
) -> (Vec<Value>, Vec<Value>) {
    match filters.decision(target_repo) {
        RepoScopeDecision::Allowed => {}
        decision => {
            let _ = tx
                .send(alog(scout, &decision.message(target_repo), "warn"))
                .await;
            return (Vec::new(), Vec::new());
        }
    }

    let repo = match gh_get(http, &format!("/repos/{target_repo}"), &[], None).await {
        Ok(repo) => repo,
        Err(e) => {
            let _ = tx
                .send(alog(
                    scout,
                    &format!("Target repo {target_repo} could not be read: {e}"),
                    "warn",
                ))
                .await;
            return (Vec::new(), Vec::new());
        }
    };
    let repos = vec![repo.clone()];

    let _ = tx
        .send(sse(
            "repos",
            json!({"repos": [{
                "id": repo["id"],
                "full_name": repo["full_name"],
                "description": repo["description"],
                "stars": repo["stargazers_count"],
                "language": repo["language"],
                "url": repo["html_url"],
                "open_issues": repo["open_issues_count"],
            }]}),
        ))
        .await;

    let labels = req.labels.join(",");
    let per_page = req.max_issues.clamp(5, 100).to_string();
    let mut all_issues = Vec::new();
    match gh_get(
        http,
        &format!("/repos/{target_repo}/issues"),
        &[
            ("state", "open"),
            ("labels", &labels),
            ("per_page", &per_page),
        ],
        None,
    )
    .await
    {
        Ok(items) => {
            for iss in items.as_array().into_iter().flatten() {
                if iss["pull_request"].is_object() {
                    continue;
                }
                all_issues.push(json!({
                    "id": iss["id"], "number": iss["number"], "title": iss["title"],
                    "body": iss["body"].as_str().unwrap_or("").chars().take(500).collect::<String>(),
                    "labels": iss["labels"].as_array().into_iter().flatten().filter_map(|l| l["name"].as_str()).collect::<Vec<_>>(),
                    "comments": iss["comments"], "created": iss["created_at"],
                    "url": iss["html_url"], "repo": target_repo, "repo_url": repo["html_url"],
                    "status": "queued", "fixability_score": 50, "fixability_reason": "",
                }));
            }
        }
        Err(e) => {
            let _ = tx
                .send(alog(scout, &format!("Skipped {target_repo}: {e}"), "warn"))
                .await;
        }
    }

    (repos, all_issues)
}

pub async fn dry_run(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<RunRequest>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel(128);
    let http = state.http.clone();
    let agents = state.agents.clone();

    tokio::spawn(async move {
        let agents_snap = agents.read().await.clone();
        let Some(team) = select_run_team(&agents_snap) else {
            emit_no_agents(&tx).await;
            return;
        };
        let filters = load_filters();
        let run_id = Uuid::new_v4().to_string()[..12].to_string();
        let run_cost = Arc::new(AtomicI64::new(0));
        let run_config_json = serde_json::to_string(&req).unwrap_or_else(|_| "{}".to_string());
        let _ = start_run(RunStart {
            run_id: &run_id,
            config_json: &run_config_json,
            dry_run: true,
        });

        let _ = tx.send(sse("phase", json!({"phase":"scan"}))).await;
        persist_run_phase(
            &run_id,
            "discover",
            "Dry Stalk repository discovery started",
        );
        let _ = tx
            .send(alog(
                &team.scout,
                "[DRY STALK] Scanning — no reaping will happen",
                "info",
            ))
            .await;

        let (repos, issues, fixable, scoring_available) =
            collect_targets(&http, &req, &team.scout, &filters, &tx, Some(&run_cost)).await;
        let _ = tx
            .send(sse("issues", json!({"issues": issues.clone()})))
            .await;
        let mut analysis_available = false;
        let mut report = None;
        if scoring_available {
            let _ = tx
                .send(alog(
                    &team.scout,
                    &format!(
                        "[DRY STALK] Would target {} scored issues — 0 changes made",
                        fixable.len()
                    ),
                    "success",
                ))
                .await;

            match agent_dry_run_analysis(&http, &fixable, &repos, &team.scout).await {
                Ok((next_report, cost)) => {
                    run_cost.fetch_add((cost * 1_000_000.0) as i64, Ordering::Relaxed);
                    let next_report = serde_json::to_value(next_report).unwrap_or_else(|error| {
                        json!({"error": format!("could not encode typed dry-run report: {error}")})
                    });
                    report = Some(next_report.clone());
                    analysis_available = true;
                    let _ = tx
                        .send(sse("dry_run_report", json!({"report": next_report})))
                        .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(alog(
                            &team.scout,
                            &format!("Dry-run analysis failed: {e}"),
                            "warn",
                        ))
                        .await;
                }
            }
        } else {
            let _ = tx
                .send(alog(
                    &team.scout,
                    &format!(
                        "[DRY STALK] Found {} candidates, but Scout scoring and analysis could not run — 0 changes made",
                        fixable.len()
                    ),
                    "warn",
                ))
                .await;
        }
        let rc = run_cost.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let _ = save_dry_stalk_run(
            &run_id,
            &repos,
            &issues,
            report.as_ref(),
            scoring_available,
            analysis_available,
        );
        let _ = finish_run(&run_id, 0, fixable.len() as i64, rc, RunStatus::Done);

        let _ = tx
            .send(sse(
                "done",
                json!({
                    "analysis_available": analysis_available,
                    "dry_run": true,
                    "scoring_available": scoring_available,
                    "total_fixed": 0,
                    "total_attempted": fixable.len(),
                    "total_would_reap": fixable.len(),
                    "run_id": run_id,
                    "cost": rc
                }),
            ))
            .await;
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

pub async fn run(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<RunRequest>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel(256);
    tokio::spawn(execute_run(state, req, tx.clone()));

    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

pub async fn execute_run(
    state: AppState,
    req: RunRequest,
    tx: mpsc::Sender<Result<Event, Infallible>>,
) {
    let http = state.http.clone();
    let agents_arc = state.agents.clone();
    let run_active = state.run_active.clone();

    if run_active
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        let _ = tx
            .send(sse("error", json!({"msg":"A hunt is already active"})))
            .await;
        return;
    }

    let run_id = Uuid::new_v4().to_string()[..12].to_string();
    let run_cost = Arc::new(AtomicI64::new(0));
    let cancel_requested = Arc::new(AtomicBool::new(false));
    let budget = req
        .cost_budget_usd
        .max(cfg("COST_BUDGET_USD").parse().unwrap_or(0.0));
    let min_conf = cfg("MIN_REVIEW_CONFIDENCE").parse().unwrap_or(40i32);

    let agents_snap = agents_arc.read().await.clone();
    let Some(team) = select_run_team(&agents_snap) else {
        emit_no_agents(&tx).await;
        let _ = tx.send(sse("done", json!({"total_fixed":0}))).await;
        run_active.store(false, Ordering::SeqCst);
        return;
    };

    let filters = load_filters();
    let run_config_json = serde_json::to_string(&req).unwrap_or_else(|_| "{}".to_string());
    let _ = start_run(RunStart {
        run_id: &run_id,
        config_json: &run_config_json,
        dry_run: false,
    });
    if budget <= 0.0 {
        let _ = tx.send(sse("log", json!({"msg":"No cost budget configured — run is currently uncapped","type":"warn"}))).await;
    }

    let _ = tx.send(sse("phase", json!({"phase":"scan"}))).await;
    persist_run_phase(
        &run_id,
        "discover",
        "Repository and issue discovery started",
    );
    let _ = tx.send(astatus(&team.scout.id, "working", "Hunting")).await;

    let (repos, all_issues, fixable, _scoring_available) =
        collect_targets(&http, &req, &team.scout, &filters, &tx, Some(&run_cost)).await;

    let _ = tx.send(sse("phase", json!({"phase":"triage"}))).await;
    persist_run_phase(&run_id, "triage", "Candidate issue triage started");
    let _ = tx
        .send(astatus(&team.scout.id, "working", "Judging issues"))
        .await;
    emit_queued_targets(&tx, &team.scout, &repos, &all_issues, &fixable).await;
    let _ = tx.send(astatus(&team.scout.id, "idle", "")).await;

    if fixable.is_empty() {
        let rc = run_cost.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        let _ = finish_run(&run_id, 0, 0, rc, RunStatus::Done);
        let _ = tx.send(sse("done", json!({"total_fixed":0}))).await;
        run_active.store(false, Ordering::SeqCst);
        return;
    }

    let _ = tx.send(sse("phase", json!({"phase":"fix"}))).await;
    persist_run_phase(&run_id, "patch", "Patch worker wave started");
    run_fix_wave(FixWaveInput {
        state: &state,
        req: &req,
        tx: &tx,
        team: &team,
        fixable: &fixable,
        run_id: &run_id,
        run_cost: &run_cost,
        cancel_requested: &cancel_requested,
        budget,
        min_conf,
    })
    .await;

    finalize_run_with_summary(&tx, &run_id, &run_cost, fixable.len()).await;

    run_active.store(false, Ordering::SeqCst);
}

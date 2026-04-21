use crate::db::get_conn;
use crate::github::{gh_check_rate_limit, gh_delete_branch, gh_poll_pr};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use patchhive_product_core::contract;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/history", get(get_history))
        .route("/history/:run_id", get(get_run))
        .route("/runs", get(get_runs_contract))
        .route("/runs/:run_id", get(get_run))
        .route("/diff/:run_id/:issue_number", get(get_diff))
        .route("/leaderboard", get(get_leaderboard))
        .route("/rejected", get(get_rejected))
        .route("/pr-tracking", get(get_tracked_prs))
        .route("/pr-tracking/:repo/:pr_number/refresh", post(refresh_pr))
        .route("/github/rate-limit", get(rate_limit_check))
}

async fn get_runs_contract(State(_): State<AppState>) -> Json<contract::ProductRunsResponse> {
    let Ok(conn) = get_conn() else {
        return Json(contract::runs_from_values("repo-reaper", Vec::new()));
    };
    let runs: Vec<Value> = conn.prepare(
        "SELECT id, started_at, finished_at, total_fixed, total_attempted, total_cost_usd, status FROM runs ORDER BY started_at DESC LIMIT 30"
    ).ok().and_then(|mut s| {
        let mapped = s.query_map([], |r| Ok(json!({
            "id": r.get::<_,String>(0)?,
            "started_at": r.get::<_,String>(1)?,
            "finished_at": r.get::<_,Option<String>>(2)?,
            "total_fixed": r.get::<_,i64>(3)?,
            "total_attempted": r.get::<_,i64>(4)?,
            "total_cost_usd": r.get::<_,f64>(5)?,
            "status": r.get::<_,String>(6)?,
        }))).ok()?;
        Some(mapped.flatten().collect())
    }).unwrap_or_default();

    Json(contract::runs_from_values("repo-reaper", runs))
}

async fn get_history(State(_): State<AppState>) -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"history":[]}));
    };
    let runs: Vec<Value> = conn.prepare(
        "SELECT id, started_at, finished_at, total_fixed, total_attempted, total_cost_usd, status FROM runs ORDER BY started_at DESC LIMIT 30"
    ).ok().and_then(|mut s| {
        let mapped = s.query_map([], |r| Ok(json!({
            "id": r.get::<_,String>(0)?, "started_at": r.get::<_,String>(1)?,
            "finished_at": r.get::<_,Option<String>>(2)?, "total_fixed": r.get::<_,i64>(3)?,
            "total_attempted": r.get::<_,i64>(4)?, "total_cost_usd": r.get::<_,f64>(5)?,
            "status": r.get::<_,String>(6)?,
        }))).ok()?;
        Some(mapped.flatten().collect())
    }).unwrap_or_default();

    let mut attempts_by_run: HashMap<String, Vec<Value>> = HashMap::new();
    let attempts: Vec<(String, Value)> = conn
        .prepare(
            "WITH recent_runs AS (
                SELECT id FROM runs ORDER BY started_at DESC LIMIT 30
             )
             SELECT
                ia.run_id,
                ia.id,
                ia.issue_number,
                ia.issue_title,
                ia.issue_url,
                ia.status,
                ia.skip_reason,
                ia.pr_url,
                ia.pr_number,
                ia.reaper_agent,
                ia.smith_agent,
                ia.gatekeeper_agent,
                ia.started_at,
                ia.finished_at,
                ia.cost_usd,
                ia.patch_diff,
                ia.confidence
             FROM issue_attempts ia
             JOIN recent_runs rr ON rr.id = ia.run_id
             ORDER BY ia.run_id, ia.started_at"
        )
        .ok()
        .and_then(|mut s| {
            let mapped = s.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    json!({
                        "id":r.get::<_,String>(1)?,"issue_number":r.get::<_,i64>(2)?,"issue_title":r.get::<_,String>(3)?,
                        "issue_url":r.get::<_,Option<String>>(4)?,"status":r.get::<_,String>(5)?,
                        "skip_reason":r.get::<_,Option<String>>(6)?,"pr_url":r.get::<_,Option<String>>(7)?,
                        "pr_number":r.get::<_,Option<i64>>(8)?,"reaper_agent":r.get::<_,String>(9)?,
                        "smith_agent":r.get::<_,Option<String>>(10)?,"gatekeeper_agent":r.get::<_,String>(11)?,
                        "started_at":r.get::<_,String>(12)?,"finished_at":r.get::<_,Option<String>>(13)?,
                        "cost_usd":r.get::<_,f64>(14)?,"patch_diff":r.get::<_,Option<String>>(15)?,"confidence":r.get::<_,i32>(16)?,
                    }),
                ))
            }).ok()?;
            Some(mapped.flatten().collect())
        })
        .unwrap_or_default();

    for (run_id, attempt) in attempts {
        attempts_by_run.entry(run_id).or_default().push(attempt);
    }

    let mut result = Vec::new();
    for run in runs {
        let run_id = run["id"].as_str().unwrap_or("").to_string();
        let mut run_obj = run.as_object().cloned().unwrap_or_default();
        run_obj.insert(
            "attempts".into(),
            json!(attempts_by_run.remove(&run_id).unwrap_or_default()),
        );
        result.push(Value::Object(run_obj));
    }
    Json(json!({"history": result}))
}

async fn get_run(Path(run_id): Path<String>, State(_): State<AppState>) -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({}));
    };
    let run: Option<Value> = conn.query_row(
        "SELECT id,started_at,finished_at,total_fixed,total_attempted,total_cost_usd,status FROM runs WHERE id=?",
        [&run_id], |r| Ok(json!({"id":r.get::<_,String>(0)?,"started_at":r.get::<_,String>(1)?,"finished_at":r.get::<_,Option<String>>(2)?,"total_fixed":r.get::<_,i64>(3)?,"total_attempted":r.get::<_,i64>(4)?,"total_cost_usd":r.get::<_,f64>(5)?,"status":r.get::<_,String>(6)?}))
    ).ok();
    if run.is_none() {
        return Json(json!({"error":"not found"}));
    }
    let attempts: Vec<Value> = conn.prepare(
        "SELECT id, issue_number, issue_title, status, skip_reason, pr_url, pr_number, cost_usd, patch_diff, confidence FROM issue_attempts WHERE run_id=? ORDER BY started_at"
    ).ok().and_then(|mut s| {
        let mapped = s.query_map([&run_id], |r| Ok(json!({
            "id": r.get::<_, String>(0)?,
            "issue_number": r.get::<_, i64>(1)?,
            "issue_title": r.get::<_, String>(2)?,
            "status": r.get::<_, String>(3)?,
            "skip_reason": r.get::<_, Option<String>>(4)?,
            "pr_url": r.get::<_, Option<String>>(5)?,
            "pr_number": r.get::<_, Option<i64>>(6)?,
            "cost_usd": r.get::<_, f64>(7)?,
            "patch_diff": r.get::<_, Option<String>>(8)?,
            "confidence": r.get::<_, i32>(9)?,
        }))).ok()?;
        Some(mapped.flatten().collect())
    }).unwrap_or_default();
    let mut run_obj = run.and_then(|v| v.as_object().cloned()).unwrap_or_default();
    run_obj.insert("attempts".into(), json!(attempts));
    Json(Value::Object(run_obj))
}

async fn get_diff(
    Path((run_id, issue_number)): Path<(String, i64)>,
    State(_): State<AppState>,
) -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"diff":null}));
    };
    let diff: Option<String> = conn
        .query_row(
            "SELECT patch_diff FROM issue_attempts WHERE run_id=? AND issue_number=?",
            rusqlite::params![run_id, issue_number],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    Json(json!({"diff": diff}))
}

async fn get_leaderboard(State(_): State<AppState>) -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"leaderboard":[]}));
    };
    let rows: Vec<Value> = conn.prepare(
        "SELECT agent_name, provider, model, role, total_fixed, total_skipped, total_errors, total_cost_usd,
         CASE WHEN (total_fixed+total_skipped+total_errors)>0
              THEN ROUND(100.0*total_fixed/(total_fixed+total_skipped+total_errors),1)
              ELSE 0 END AS fix_rate
         FROM agent_performance ORDER BY fix_rate DESC, total_fixed DESC"
    ).ok().and_then(|mut s| {
        let mapped = s.query_map([], |r| Ok(json!({
            "agent_name":r.get::<_,String>(0)?,"provider":r.get::<_,String>(1)?,"model":r.get::<_,String>(2)?,
            "role":r.get::<_,String>(3)?,"total_fixed":r.get::<_,i64>(4)?,"total_skipped":r.get::<_,i64>(5)?,
            "total_errors":r.get::<_,i64>(6)?,"total_cost_usd":r.get::<_,f64>(7)?,"fix_rate":r.get::<_,f64>(8)?,
        }))).ok()?;
        Some(mapped.flatten().collect())
    }).unwrap_or_default();
    Json(json!({"leaderboard": rows}))
}

async fn get_rejected(State(_): State<AppState>) -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"rejected":[]}));
    };
    let rows: Vec<Value> = conn.prepare(
        "SELECT id,run_id,repo,issue_number,issue_title,reason,smith_feedback,confidence,created_at FROM rejected_patches ORDER BY created_at DESC LIMIT 100"
    ).ok().and_then(|mut s| {
        let mapped = s.query_map([], |r| Ok(json!({
            "id":r.get::<_,String>(0)?,"run_id":r.get::<_,String>(1)?,"repo":r.get::<_,String>(2)?,
            "issue_number":r.get::<_,i64>(3)?,"issue_title":r.get::<_,String>(4)?,
            "reason":r.get::<_,String>(5)?,"smith_feedback":r.get::<_,String>(6)?,
            "confidence":r.get::<_,i32>(7)?,"created_at":r.get::<_,String>(8)?,
        }))).ok()?;
        Some(mapped.flatten().collect())
    }).unwrap_or_default();
    Json(json!({"rejected": rows}))
}

async fn get_tracked_prs(State(_): State<AppState>) -> Json<Value> {
    let Ok(conn) = get_conn() else {
        return Json(json!({"prs":[]}));
    };
    let rows: Vec<Value> = conn.prepare(
        "SELECT pr_number,repo,run_id,opened_at,last_checked,state,merged,review_state FROM pr_tracking ORDER BY opened_at DESC LIMIT 50"
    ).ok().and_then(|mut s| {
        let mapped = s.query_map([], |r| Ok(json!({
            "pr_number":r.get::<_,i64>(0)?,"repo":r.get::<_,String>(1)?,"run_id":r.get::<_,String>(2)?,
            "opened_at":r.get::<_,String>(3)?,"last_checked":r.get::<_,Option<String>>(4)?,
            "state":r.get::<_,String>(5)?,"merged":r.get::<_,i32>(6)?,"review_state":r.get::<_,Option<String>>(7)?,
        }))).ok()?;
        Some(mapped.flatten().collect())
    }).unwrap_or_default();
    Json(json!({"prs": rows}))
}

async fn refresh_pr(
    Path((repo, pr_number)): Path<(String, i64)>,
    State(state): State<AppState>,
) -> Json<Value> {
    let pr_state = gh_poll_pr(&state.http, &repo, pr_number, None).await;
    let merged = pr_state["merged"].as_bool().unwrap_or(false);
    let issue_number: Option<i64> = if let Ok(conn) = get_conn() {
        let issue_number = conn
            .query_row(
                "SELECT issue_number FROM issue_attempts WHERE run_id IN (
                 SELECT run_id FROM pr_tracking WHERE pr_number=?1 AND repo=?2
             ) AND pr_number=?1 LIMIT 1",
                rusqlite::params![pr_number, repo],
                |r| r.get(0),
            )
            .ok();
        let _ = conn.execute(
            "UPDATE pr_tracking SET state=?1,merged=?2,review_state=?3,last_checked=?4 WHERE pr_number=?5 AND repo=?6",
            rusqlite::params![
                pr_state["state"].as_str().unwrap_or("open"), merged as i32,
                pr_state["review_state"].as_str(), chrono::Utc::now().to_rfc3339(),
                pr_number, repo,
            ],
        );
        issue_number
    } else {
        return Json(pr_state);
    };
    if merged {
        let branch_issue = issue_number.unwrap_or(pr_number);
        gh_delete_branch(
            &state.http,
            &repo,
            &format!("reaper/issue-{branch_issue}"),
            None,
            None,
        )
        .await;
    }
    Json(pr_state)
}

async fn rate_limit_check(State(state): State<AppState>) -> Json<Value> {
    Json(gh_check_rate_limit(&state.http, None).await)
}

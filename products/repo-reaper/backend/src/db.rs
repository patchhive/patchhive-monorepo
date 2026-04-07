use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::Value;
use std::path::PathBuf;
use chrono::Utc;

pub fn db_path() -> PathBuf {
    std::env::var("REAPER_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("repo-reaper.db"))
}

pub fn get_conn() -> Result<Connection> {
    let conn = Connection::open(db_path())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(conn)
}

pub fn init_db() -> Result<()> {
    let conn = get_conn()?;
    conn.execute_batch(SCHEMA)?;
    Ok(())
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY, started_at TEXT, finished_at TEXT,
    total_fixed INTEGER DEFAULT 0, total_attempted INTEGER DEFAULT 0,
    total_cost_usd REAL DEFAULT 0.0, status TEXT DEFAULT 'running',
    config_json TEXT, dry_run INTEGER DEFAULT 0
);
CREATE TABLE IF NOT EXISTS issue_attempts (
    id TEXT PRIMARY KEY, run_id TEXT, repo TEXT, issue_number INTEGER,
    issue_title TEXT, issue_url TEXT, status TEXT, skip_reason TEXT,
    pr_url TEXT, pr_number INTEGER,
    reaper_agent TEXT, smith_agent TEXT, gatekeeper_agent TEXT,
    started_at TEXT, finished_at TEXT, duration_seconds REAL,
    cost_usd REAL DEFAULT 0.0, patch_diff TEXT, error_msg TEXT,
    confidence INTEGER DEFAULT 0,
    FOREIGN KEY(run_id) REFERENCES runs(id)
);
CREATE TABLE IF NOT EXISTS rejected_patches (
    id TEXT PRIMARY KEY,
    run_id TEXT,
    repo TEXT,
    issue_number INTEGER,
    issue_title TEXT,
    reason TEXT,
    smith_feedback TEXT,
    confidence INTEGER,
    patch_diff TEXT,
    created_at TEXT
);
CREATE TABLE IF NOT EXISTS agent_performance (
    agent_name TEXT, provider TEXT, model TEXT, role TEXT,
    total_fixed INTEGER DEFAULT 0, total_skipped INTEGER DEFAULT 0,
    total_errors INTEGER DEFAULT 0, total_cost_usd REAL DEFAULT 0.0,
    PRIMARY KEY(agent_name, provider, model, role)
);
CREATE TABLE IF NOT EXISTS team_presets (
    name TEXT PRIMARY KEY, agents_json TEXT, created_at TEXT
);
CREATE TABLE IF NOT EXISTS repo_lists (
    repo TEXT PRIMARY KEY, list_type TEXT, added_at TEXT
);
CREATE TABLE IF NOT EXISTS scheduled_runs (
    id TEXT PRIMARY KEY, cron_expr TEXT, config_json TEXT,
    enabled INTEGER DEFAULT 1, last_run TEXT, next_run TEXT
);
CREATE TABLE IF NOT EXISTS pr_tracking (
    pr_number INTEGER, repo TEXT, run_id TEXT, opened_at TEXT,
    last_checked TEXT, state TEXT DEFAULT 'open',
    merged INTEGER DEFAULT 0, review_state TEXT,
    PRIMARY KEY(pr_number, repo)
);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY, value TEXT
);
";

pub fn get_lifetime_cost() -> f64 {
    let Ok(conn) = get_conn() else { return 0.0 };
    conn.query_row(
        "SELECT COALESCE(SUM(total_cost_usd), 0.0) FROM runs WHERE status='done'",
        [],
        |r| r.get::<_, f64>(0),
    ).unwrap_or(0.0)
}

pub fn start_run(run_id: &str, config: &Value, dry_run: bool) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO runs(id, started_at, status, config_json, dry_run) VALUES(?1,?2,'running',?3,?4)",
        params![run_id, Utc::now().to_rfc3339(), config.to_string(), dry_run as i32],
    )?;
    Ok(())
}

pub fn finish_run(run_id: &str, fixed: i64, attempted: i64, cost: f64, status: &str) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "UPDATE runs SET finished_at=?1, total_fixed=?2, total_attempted=?3, total_cost_usd=?4, status=?5 WHERE id=?6",
        params![Utc::now().to_rfc3339(), fixed, attempted, cost, status, run_id],
    )?;
    Ok(())
}

pub fn start_attempt(attempt_id: &str, run_id: &str, issue: &Value, reaper: &str, smith: Option<&str>, gatekeeper: &str) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO issue_attempts(id,run_id,repo,issue_number,issue_title,issue_url,status,reaper_agent,smith_agent,gatekeeper_agent,started_at)
         VALUES(?1,?2,?3,?4,?5,?6,'running',?7,?8,?9,?10)",
        params![
            attempt_id, run_id,
            issue["repo"].as_str().unwrap_or(""),
            issue["number"].as_i64().unwrap_or(0),
            issue["title"].as_str().unwrap_or(""),
            issue["url"].as_str().unwrap_or(""),
            reaper, smith, gatekeeper,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn finish_attempt(
    attempt_id: &str, status: &str, pr_url: Option<&str>, pr_number: Option<i64>,
    cost: f64, patch_diff: Option<&str>, error_msg: Option<&str>,
    skip_reason: Option<&str>, duration: Option<f64>, confidence: i32,
) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "UPDATE issue_attempts SET status=?1,pr_url=?2,pr_number=?3,finished_at=?4,
         duration_seconds=?5,cost_usd=?6,patch_diff=?7,error_msg=?8,skip_reason=?9,confidence=?10
         WHERE id=?11",
        params![
            status, pr_url, pr_number, Utc::now().to_rfc3339(),
            duration, cost, patch_diff, error_msg, skip_reason, confidence, attempt_id,
        ],
    )?;
    Ok(())
}

pub fn save_rejected_patch(
    id: &str, run_id: &str, repo: &str, issue_number: i64,
    issue_title: &str, reason: &str, feedback: &str, confidence: i32, diff: &str,
) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO rejected_patches(id,run_id,repo,issue_number,issue_title,reason,smith_feedback,confidence,patch_diff,created_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![id, run_id, repo, issue_number, issue_title, reason, feedback, confidence, diff, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn track_pr(pr_number: i64, repo: &str, run_id: &str) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT OR REPLACE INTO pr_tracking(pr_number,repo,run_id,opened_at,state) VALUES(?1,?2,?3,?4,'open')",
        params![pr_number, repo, run_id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn update_perf(agent_name: &str, provider: &str, model: &str, role: &str, outcome: &str, cost: f64) -> Result<()> {
    let col = match outcome { "fixed" => "total_fixed", "skipped" => "total_skipped", _ => "total_errors" };
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO agent_performance(agent_name,provider,model,role,total_fixed,total_skipped,total_errors,total_cost_usd)
         VALUES(?1,?2,?3,?4,0,0,0,0) ON CONFLICT(agent_name,provider,model,role) DO NOTHING",
        params![agent_name, provider, model, role],
    )?;
    conn.execute(
        &format!("UPDATE agent_performance SET {col}={col}+1, total_cost_usd=total_cost_usd+?1 WHERE agent_name=?2 AND provider=?3 AND model=?4 AND role=?5"),
        params![cost, agent_name, provider, model, role],
    )?;
    Ok(())
}

pub fn recover_orphaned_runs() -> Vec<String> {
    let Ok(conn) = get_conn() else { return vec![] };
    let ids: Vec<String> = conn.prepare("SELECT id FROM runs WHERE status='running'")
        .and_then(|mut s| s.query_map([], |r| r.get(0)).map(|rows| rows.flatten().collect()))
        .unwrap_or_default();
    if !ids.is_empty() {
        let _ = conn.execute(
            "UPDATE runs SET status='crashed', finished_at=?1 WHERE status='running'",
            params![Utc::now().to_rfc3339()],
        );
    }
    ids
}

pub fn get_setting(key: &str, default: &str) -> String {
    let Ok(conn) = get_conn() else { return default.to_string() };
    conn.query_row("SELECT value FROM settings WHERE key=?1", params![key], |r| r.get::<_, String>(0))
        .unwrap_or_else(|_| default.to_string())
}

pub fn set_setting(key: &str, value: &str) -> Result<()> {
    let conn = get_conn()?;
    conn.execute("INSERT OR REPLACE INTO settings(key,value) VALUES(?1,?2)", params![key, value])?;
    Ok(())
}

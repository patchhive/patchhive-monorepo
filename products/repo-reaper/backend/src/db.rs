use anyhow::{Context, Result};
use chrono::Utc;
use once_cell::sync::Lazy;
use patchhive_product_core::secrets::TokenProtector;
use patchhive_product_core::sqlite::{PooledSqliteConnection, SqlitePool};
use rusqlite::params;
use std::path::PathBuf;

use crate::state::AgentConfig;

const ACTIVE_AGENTS_SETTING: &str = "active_agents_json";

static DB_POOL: Lazy<SqlitePool> = Lazy::new(|| {
    SqlitePool::new(db_path(), "RepoReaper").with_pool_size_env("REAPER_DB_POOL_SIZE")
});

pub fn db_path() -> PathBuf {
    std::env::var("REAPER_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("repo-reaper.db"))
}

pub fn get_conn() -> Result<PooledSqliteConnection<'static>> {
    Ok(DB_POOL.get()?)
}

pub fn init_db() -> Result<()> {
    let conn = get_conn()?;
    conn.execute_batch(SCHEMA)?;
    Ok(())
}

pub fn health_check() -> bool {
    get_conn()
        .and_then(|conn| {
            conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                .map_err(Into::into)
        })
        .is_ok()
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
    )
    .unwrap_or(0.0)
}

pub fn agent_token_protector() -> TokenProtector {
    TokenProtector::from_env_candidates(&["REAPER_ENCRYPTION_KEY", "PATCHHIVE_ENCRYPTION_KEY"])
}

pub fn load_active_agents() -> Result<Vec<AgentConfig>> {
    let raw = get_setting(ACTIVE_AGENTS_SETTING, "");
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    agents_from_storage_json(&raw)
}

pub fn save_active_agents(agents: &[AgentConfig]) -> Result<()> {
    let encoded = agents_to_storage_json(agents)?;
    set_setting(ACTIVE_AGENTS_SETTING, &encoded)
}

pub fn migrate_agent_secret_storage() -> Result<()> {
    let protector = agent_token_protector();
    if !protector.configured() {
        return Ok(());
    }

    let raw_active_agents = get_setting(ACTIVE_AGENTS_SETTING, "");
    if !raw_active_agents.trim().is_empty() {
        let active_agents: Vec<AgentConfig> = serde_json::from_str(&raw_active_agents)
            .context("failed to decode RepoReaper active agent team during migration")?;
        let migrated = agents_to_storage_json(&active_agents)?;
        set_setting(ACTIVE_AGENTS_SETTING, &migrated)?;
    }

    let conn = get_conn()?;
    let presets = {
        let mut stmt = conn.prepare("SELECT name, agents_json FROM team_presets")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.flatten().collect::<Vec<_>>()
    };

    for (name, raw_agents) in presets {
        let agents: Vec<AgentConfig> = serde_json::from_str(&raw_agents)
            .with_context(|| format!("failed to decode RepoReaper team preset {name}"))?;
        let migrated = agents_to_storage_json(&agents)?;
        conn.execute(
            "UPDATE team_presets SET agents_json=?1 WHERE name=?2",
            params![migrated, name],
        )?;
    }

    Ok(())
}

pub fn agents_to_storage_json(agents: &[AgentConfig]) -> Result<String> {
    let protector = agent_token_protector();
    let protected = agents
        .iter()
        .map(|agent| protect_agent_for_storage(agent, &protector))
        .collect::<Result<Vec<_>>>()?;
    serde_json::to_string(&protected).context("failed to encode RepoReaper agent team")
}

pub fn agents_from_storage_json(raw: &str) -> Result<Vec<AgentConfig>> {
    let protector = agent_token_protector();
    let stored: Vec<AgentConfig> =
        serde_json::from_str(raw).context("failed to decode RepoReaper agent team")?;
    stored
        .into_iter()
        .map(|agent| reveal_agent_from_storage(agent, &protector))
        .collect()
}

fn protect_agent_for_storage(
    agent: &AgentConfig,
    protector: &TokenProtector,
) -> Result<AgentConfig> {
    let mut stored = agent.clone();
    stored.api_key = protect_optional_secret(stored.api_key.as_deref(), protector)
        .with_context(|| format!("failed to protect API key for {}", stored.name))?;
    stored.bot_token = protect_optional_secret(stored.bot_token.as_deref(), protector)
        .with_context(|| format!("failed to protect bot token for {}", stored.name))?;
    Ok(stored)
}

fn reveal_agent_from_storage(
    mut agent: AgentConfig,
    protector: &TokenProtector,
) -> Result<AgentConfig> {
    agent.api_key = reveal_optional_secret(agent.api_key.as_deref(), protector)
        .with_context(|| format!("failed to reveal API key for {}", agent.name))?;
    agent.bot_token = reveal_optional_secret(agent.bot_token.as_deref(), protector)
        .with_context(|| format!("failed to reveal bot token for {}", agent.name))?;
    Ok(agent)
}

fn protect_optional_secret(
    value: Option<&str>,
    protector: &TokenProtector,
) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if TokenProtector::is_encrypted_value(value) {
        return Ok(Some(value.to_string()));
    }
    if !protector.configured() {
        return Ok(None);
    }
    protector.protect_for_storage(value).map(Some)
}

fn reveal_optional_secret(
    value: Option<&str>,
    protector: &TokenProtector,
) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    protector.reveal_from_storage(value).map(Some)
}

pub struct RunStart<'a> {
    pub run_id: &'a str,
    pub config_json: &'a str,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Done,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
        }
    }
}

pub fn start_run(input: RunStart<'_>) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO runs(id, started_at, status, config_json, dry_run) VALUES(?1,?2,'running',?3,?4)",
        params![
            input.run_id,
            Utc::now().to_rfc3339(),
            input.config_json,
            input.dry_run as i32
        ],
    )?;
    Ok(())
}

pub fn finish_run(
    run_id: &str,
    fixed: i64,
    attempted: i64,
    cost: f64,
    status: RunStatus,
) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "UPDATE runs SET finished_at=?1, total_fixed=?2, total_attempted=?3, total_cost_usd=?4, status=?5 WHERE id=?6",
        params![
            Utc::now().to_rfc3339(),
            fixed,
            attempted,
            cost,
            status.as_str(),
            run_id
        ],
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueAttemptTarget {
    pub repo: String,
    pub issue_number: i64,
    pub issue_title: String,
    pub issue_url: String,
}

pub struct IssueAttemptStart<'a> {
    pub attempt_id: &'a str,
    pub run_id: &'a str,
    pub target: &'a IssueAttemptTarget,
    pub reaper_agent: &'a str,
    pub smith_agent: Option<&'a str>,
    pub gatekeeper_agent: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueAttemptStatus {
    Fixed,
    Skipped,
    Error,
}

impl IssueAttemptStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Skipped => "skipped",
            Self::Error => "error",
        }
    }
}

pub struct IssueAttemptFinish<'a> {
    pub attempt_id: &'a str,
    pub status: IssueAttemptStatus,
    pub pr_url: Option<&'a str>,
    pub pr_number: Option<i64>,
    pub cost_usd: f64,
    pub patch_diff: Option<&'a str>,
    pub error_msg: Option<&'a str>,
    pub skip_reason: Option<&'a str>,
    pub duration_seconds: Option<f64>,
    pub confidence: i32,
}

pub fn start_attempt(input: IssueAttemptStart<'_>) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO issue_attempts(id,run_id,repo,issue_number,issue_title,issue_url,status,reaper_agent,smith_agent,gatekeeper_agent,started_at)
         VALUES(?1,?2,?3,?4,?5,?6,'running',?7,?8,?9,?10)",
        params![
            input.attempt_id,
            input.run_id,
            &input.target.repo,
            input.target.issue_number,
            &input.target.issue_title,
            &input.target.issue_url,
            input.reaper_agent,
            input.smith_agent,
            input.gatekeeper_agent,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn finish_attempt(input: IssueAttemptFinish<'_>) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "UPDATE issue_attempts SET status=?1,pr_url=?2,pr_number=?3,finished_at=?4,
         duration_seconds=?5,cost_usd=?6,patch_diff=?7,error_msg=?8,skip_reason=?9,confidence=?10
         WHERE id=?11",
        params![
            input.status.as_str(),
            input.pr_url,
            input.pr_number,
            Utc::now().to_rfc3339(),
            input.duration_seconds,
            input.cost_usd,
            input.patch_diff,
            input.error_msg,
            input.skip_reason,
            input.confidence,
            input.attempt_id,
        ],
    )?;
    Ok(())
}

pub fn save_rejected_patch(
    id: &str,
    run_id: &str,
    repo: &str,
    issue_number: i64,
    issue_title: &str,
    reason: &str,
    feedback: &str,
    confidence: i32,
    diff: &str,
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

pub fn update_perf(
    agent_name: &str,
    provider: &str,
    model: &str,
    role: &str,
    outcome: &str,
    cost: f64,
) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT INTO agent_performance(agent_name,provider,model,role,total_fixed,total_skipped,total_errors,total_cost_usd)
         VALUES(?1,?2,?3,?4,0,0,0,0) ON CONFLICT(agent_name,provider,model,role) DO NOTHING",
        params![agent_name, provider, model, role],
    )?;
    match outcome {
        "fixed" => {
            conn.execute(
                "UPDATE agent_performance
                 SET total_fixed=total_fixed+1, total_cost_usd=total_cost_usd+?1
                 WHERE agent_name=?2 AND provider=?3 AND model=?4 AND role=?5",
                params![cost, agent_name, provider, model, role],
            )?;
        }
        "skipped" => {
            conn.execute(
                "UPDATE agent_performance
                 SET total_skipped=total_skipped+1, total_cost_usd=total_cost_usd+?1
                 WHERE agent_name=?2 AND provider=?3 AND model=?4 AND role=?5",
                params![cost, agent_name, provider, model, role],
            )?;
        }
        _ => {
            conn.execute(
                "UPDATE agent_performance
                 SET total_errors=total_errors+1, total_cost_usd=total_cost_usd+?1
                 WHERE agent_name=?2 AND provider=?3 AND model=?4 AND role=?5",
                params![cost, agent_name, provider, model, role],
            )?;
        }
    }
    Ok(())
}

pub fn recover_orphaned_runs() -> Vec<String> {
    let Ok(conn) = get_conn() else { return vec![] };
    // Only mark as crashed if started >10 min ago — a freshly-started run
    // that survived a brief restart shouldn't be killed.
    let cutoff = (Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM runs WHERE status='running' AND started_at < ?1")
        .and_then(|mut s| {
            s.query_map(params![cutoff], |r| r.get(0))
                .map(|rows| rows.flatten().collect())
        })
        .unwrap_or_default();
    if !ids.is_empty() {
        let _ = conn.execute(
            "UPDATE runs SET status='crashed', finished_at=?1 WHERE status='running' AND started_at < ?2",
            params![Utc::now().to_rfc3339(), cutoff],
        );
    }
    ids
}

pub fn get_setting(key: &str, default: &str) -> String {
    let Ok(conn) = get_conn() else {
        return default.to_string();
    };
    conn.query_row(
        "SELECT value FROM settings WHERE key=?1",
        params![key],
        |r| r.get::<_, String>(0),
    )
    .unwrap_or_else(|_| default.to_string())
}

pub fn set_setting(key: &str, value: &str) -> Result<()> {
    let conn = get_conn()?;
    conn.execute(
        "INSERT OR REPLACE INTO settings(key,value) VALUES(?1,?2)",
        params![key, value],
    )?;
    Ok(())
}

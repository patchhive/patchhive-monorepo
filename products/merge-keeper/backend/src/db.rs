use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Mutex, MutexGuard};

use crate::models::{HistoryItem, MergeAssessment, OverviewCounts, OverviewPayload};

static DB_CONN: OnceCell<Mutex<Connection>> = OnceCell::new();

pub fn db_path() -> String {
    std::env::var("MERGE_KEEPER_DB_PATH").unwrap_or_else(|_| "merge-keeper.db".into())
}

fn open_connection() -> Result<Connection> {
    let conn = Connection::open(db_path()).context("Could not open MergeKeeper database")?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .context("Could not initialize MergeKeeper database pragmas")?;
    Ok(conn)
}

fn connect() -> Result<MutexGuard<'static, Connection>> {
    let mutex = DB_CONN.get_or_try_init(|| open_connection().map(Mutex::new))?;
    mutex
        .lock()
        .map_err(|_| anyhow!("MergeKeeper database mutex poisoned"))
}

pub fn health_check() -> bool {
    connect()
        .and_then(|conn| {
            conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                .context("Could not query MergeKeeper database")
        })
        .is_ok()
}

pub fn init_db() -> Result<()> {
    let conn = connect()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS merge_runs (
          id TEXT PRIMARY KEY,
          repo TEXT NOT NULL,
          pr_number INTEGER NOT NULL,
          pr_title TEXT NOT NULL,
          pr_url TEXT NOT NULL,
          readiness TEXT NOT NULL,
          summary TEXT NOT NULL,
          blockers_count INTEGER NOT NULL,
          warnings_count INTEGER NOT NULL,
          approvals_count INTEGER NOT NULL,
          failing_checks_count INTEGER NOT NULL,
          pending_checks_count INTEGER NOT NULL,
          created_at TEXT NOT NULL,
          payload TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_merge_runs_created_at
        ON merge_runs(created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_merge_runs_repo_created_at
        ON merge_runs(repo, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub fn save_assessment(assessment: &MergeAssessment) -> Result<()> {
    let conn = connect()?;
    let payload =
        serde_json::to_string(assessment).context("Could not encode MergeKeeper payload")?;
    conn.execute(
        r#"
        INSERT INTO merge_runs (
          id, repo, pr_number, pr_title, pr_url, readiness, summary,
          blockers_count, warnings_count, approvals_count,
          failing_checks_count, pending_checks_count, created_at, payload
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        "#,
        params![
            assessment.id,
            assessment.repo,
            assessment.pr_number,
            assessment.pr_title,
            assessment.pr_url,
            assessment.readiness,
            assessment.summary,
            assessment.blockers.len() as i64,
            assessment.warnings.len() as i64,
            assessment.metrics.approvals,
            assessment.metrics.failing_checks,
            assessment.metrics.pending_checks,
            assessment.created_at,
            payload,
        ],
    )
    .context("Could not persist MergeKeeper run")?;
    Ok(())
}

pub fn history(limit: usize) -> Vec<HistoryItem> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };

    let mut stmt = match conn.prepare(
        r#"
        SELECT id, repo, pr_number, pr_title, readiness, summary,
               blockers_count, warnings_count, approvals_count,
               failing_checks_count, pending_checks_count, created_at
        FROM merge_runs
        ORDER BY created_at DESC
        LIMIT ?1
        "#,
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };

    stmt.query_map([limit as i64], |row| {
        Ok(HistoryItem {
            id: row.get(0)?,
            repo: row.get(1)?,
            pr_number: row.get(2)?,
            pr_title: row.get(3)?,
            readiness: row.get(4)?,
            summary: row.get(5)?,
            blockers_count: row.get::<_, i64>(6)? as u32,
            warnings_count: row.get::<_, i64>(7)? as u32,
            approvals_count: row.get::<_, i64>(8)? as u32,
            failing_checks_count: row.get::<_, i64>(9)? as u32,
            pending_checks_count: row.get::<_, i64>(10)? as u32,
            created_at: row.get(11)?,
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

pub fn get_assessment(id: &str) -> Option<MergeAssessment> {
    let conn = connect().ok()?;
    let payload = conn
        .query_row(
            "SELECT payload FROM merge_runs WHERE id = ?1 LIMIT 1",
            [id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()?;
    serde_json::from_str(&payload).ok()
}

pub fn overview_counts() -> OverviewCounts {
    let Ok(conn) = connect() else {
        return OverviewCounts::default();
    };

    conn.query_row(
        r#"
        SELECT
          COUNT(*) AS runs,
          COUNT(DISTINCT repo) AS repos,
          COALESCE(SUM(CASE WHEN readiness = 'ready' THEN 1 ELSE 0 END), 0) AS ready_runs,
          COALESCE(SUM(CASE WHEN readiness = 'blocked' THEN 1 ELSE 0 END), 0) AS blocked_runs,
          COALESCE(SUM(CASE WHEN readiness = 'hold' THEN 1 ELSE 0 END), 0) AS hold_runs
        FROM merge_runs
        "#,
        [],
        |row| {
            Ok(OverviewCounts {
                runs: row.get::<_, i64>(0)? as u32,
                repos: row.get::<_, i64>(1)? as u32,
                ready_runs: row.get::<_, i64>(2)? as u32,
                blocked_runs: row.get::<_, i64>(3)? as u32,
                hold_runs: row.get::<_, i64>(4)? as u32,
            })
        },
    )
    .unwrap_or_default()
}

pub fn overview() -> OverviewPayload {
    OverviewPayload {
        product: "MergeKeeper by PatchHive".into(),
        tagline: "Read GitHub merge pressure and turn it into a clean readiness call before a PR turns into merge roulette.".into(),
        counts: overview_counts(),
        recent_runs: history(6),
    }
}

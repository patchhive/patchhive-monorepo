use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Mutex, MutexGuard};

use crate::models::{HistoryItem, OverviewCounts, OverviewPayload, VulnScanResult};

static DB_CONN: OnceCell<Mutex<Connection>> = OnceCell::new();

pub fn db_path() -> String {
    std::env::var("VULN_TRIAGE_DB_PATH").unwrap_or_else(|_| "vuln-triage.db".into())
}

fn open_connection() -> Result<Connection> {
    let conn = Connection::open(db_path()).context("Could not open VulnTriage database")?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .context("Could not initialize VulnTriage database pragmas")?;
    Ok(conn)
}

fn connect() -> Result<MutexGuard<'static, Connection>> {
    let mutex = DB_CONN.get_or_try_init(|| open_connection().map(Mutex::new))?;
    mutex
        .lock()
        .map_err(|_| anyhow!("VulnTriage database mutex poisoned"))
}

pub fn health_check() -> bool {
    connect()
        .and_then(|conn| {
            conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                .context("Could not query VulnTriage database")
        })
        .is_ok()
}

pub fn init_db() -> Result<()> {
    let conn = connect()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS vuln_triage_scans (
          id TEXT PRIMARY KEY,
          repo TEXT NOT NULL,
          summary TEXT NOT NULL,
          tracked_findings INTEGER NOT NULL,
          fix_now INTEGER NOT NULL,
          plan_next INTEGER NOT NULL,
          watch_count INTEGER NOT NULL,
          created_at TEXT NOT NULL,
          payload TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_vuln_triage_scans_created_at
        ON vuln_triage_scans(created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_vuln_triage_scans_repo_created_at
        ON vuln_triage_scans(repo, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub fn save_scan(scan: &VulnScanResult) -> Result<()> {
    let conn = connect()?;
    let payload = serde_json::to_string(scan).context("Could not encode VulnTriage payload")?;
    conn.execute(
        r#"
        INSERT INTO vuln_triage_scans (
          id, repo, summary, tracked_findings, fix_now,
          plan_next, watch_count, created_at, payload
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            scan.id,
            scan.repo,
            scan.summary,
            scan.metrics.tracked_findings,
            scan.metrics.fix_now,
            scan.metrics.plan_next,
            scan.metrics.watch,
            scan.created_at,
            payload,
        ],
    )
    .context("Could not persist VulnTriage scan")?;
    Ok(())
}

pub fn history(limit: usize) -> Vec<HistoryItem> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };

    let mut stmt = match conn.prepare(
        r#"
        SELECT id, repo, summary, tracked_findings, fix_now,
               plan_next, watch_count, created_at
        FROM vuln_triage_scans
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
            summary: row.get(2)?,
            tracked_findings: row.get::<_, i64>(3)? as u32,
            fix_now: row.get::<_, i64>(4)? as u32,
            plan_next: row.get::<_, i64>(5)? as u32,
            watch: row.get::<_, i64>(6)? as u32,
            created_at: row.get(7)?,
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

pub fn get_scan(id: &str) -> Option<VulnScanResult> {
    let conn = connect().ok()?;
    let payload = conn
        .query_row(
            "SELECT payload FROM vuln_triage_scans WHERE id = ?1 LIMIT 1",
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
          COUNT(*) AS scans,
          COUNT(DISTINCT repo) AS repos,
          COALESCE(SUM(tracked_findings), 0) AS tracked_findings,
          COALESCE(SUM(fix_now), 0) AS fix_now,
          COALESCE(SUM(plan_next), 0) AS plan_next,
          COALESCE(SUM(watch_count), 0) AS watch_count
        FROM vuln_triage_scans
        "#,
        [],
        |row| {
            Ok(OverviewCounts {
                scans: row.get::<_, i64>(0)? as u32,
                repos: row.get::<_, i64>(1)? as u32,
                tracked_findings: row.get::<_, i64>(2)? as u32,
                fix_now: row.get::<_, i64>(3)? as u32,
                plan_next: row.get::<_, i64>(4)? as u32,
                watch: row.get::<_, i64>(5)? as u32,
            })
        },
    )
    .unwrap_or_default()
}

pub fn overview() -> OverviewPayload {
    OverviewPayload {
        product: "VulnTriage by PatchHive".into(),
        tagline: "Turn vulnerability alerts into ranked engineering work.".into(),
        counts: overview_counts(),
        recent_scans: history(6),
    }
}

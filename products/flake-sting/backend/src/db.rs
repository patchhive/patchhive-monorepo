use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{compute_trend, FlakeScanResult, HistoryItem, OverviewCounts, OverviewPayload};

pub fn db_path() -> String {
    std::env::var("FLAKE_STING_DB_PATH").unwrap_or_else(|_| "flake-sting.db".into())
}

fn connect() -> Result<Connection> {
    Connection::open(db_path()).context("Could not open FlakeSting database")
}

pub fn health_check() -> bool {
    connect()
        .and_then(|conn| {
            conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                .context("Could not query FlakeSting database")
        })
        .is_ok()
}

pub fn init_db() -> Result<()> {
    let conn = connect()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS flake_scans (
          id TEXT PRIMARY KEY,
          repo TEXT NOT NULL,
          branch TEXT NOT NULL,
          workflow_name TEXT NOT NULL,
          summary TEXT NOT NULL,
          flaky_signals INTEGER NOT NULL,
          quarantine_candidates INTEGER NOT NULL,
          created_at TEXT NOT NULL,
          payload TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_flake_scans_created_at
        ON flake_scans(created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_flake_scans_repo_created_at
        ON flake_scans(repo, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub fn save_scan(scan: &FlakeScanResult) -> Result<()> {
    let conn = connect()?;
    let payload = serde_json::to_string(scan).context("Could not encode FlakeSting payload")?;
    conn.execute(
        r#"
        INSERT INTO flake_scans (
          id, repo, branch, workflow_name, summary, flaky_signals,
          quarantine_candidates, created_at, payload
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            scan.id,
            scan.repo,
            scan.branch,
            scan.workflow_name,
            scan.summary,
            scan.metrics.flaky_signals,
            scan.metrics.quarantine_candidates,
            scan.created_at,
            payload,
        ],
    )
    .context("Could not persist FlakeSting scan")?;
    Ok(())
}

pub fn history(limit: usize) -> Vec<HistoryItem> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };

    let mut stmt = match conn.prepare(
        r#"
        SELECT id, repo, branch, workflow_name, summary,
               flaky_signals, quarantine_candidates, created_at, payload
        FROM flake_scans
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
            branch: row.get(2)?,
            workflow_name: row.get(3)?,
            summary: row.get(4)?,
            flaky_signals: row.get::<_, i64>(5)? as u32,
            quarantine_candidates: row.get::<_, i64>(6)? as u32,
            created_at: row.get(7)?,
            trend: serde_json::from_str::<FlakeScanResult>(&row.get::<_, String>(8)?)
                .ok()
                .and_then(|scan| {
                    previous_comparable_scan_before(
                        &conn,
                        &scan.repo,
                        &scan.branch,
                        &scan.workflow_name,
                        &scan.created_at,
                    )
                    .and_then(|previous| compute_trend(&scan, Some(&previous)))
                }),
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

pub fn get_scan(id: &str) -> Option<FlakeScanResult> {
    let conn = connect().ok()?;
    let payload = conn
        .query_row(
            "SELECT payload FROM flake_scans WHERE id = ?1 LIMIT 1",
            [id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()?;
    let mut scan = serde_json::from_str::<FlakeScanResult>(&payload).ok()?;
    scan.trend = previous_comparable_scan_before(
        &conn,
        &scan.repo,
        &scan.branch,
        &scan.workflow_name,
        &scan.created_at,
    )
    .and_then(|previous| compute_trend(&scan, Some(&previous)));
    Some(scan)
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
          COALESCE(SUM(flaky_signals), 0) AS flaky_signals,
          COALESCE(SUM(quarantine_candidates), 0) AS quarantine_candidates
        FROM flake_scans
        "#,
        [],
        |row| {
            Ok(OverviewCounts {
                scans: row.get::<_, i64>(0)? as u32,
                repos: row.get::<_, i64>(1)? as u32,
                flaky_signals: row.get::<_, i64>(2)? as u32,
                quarantine_candidates: row.get::<_, i64>(3)? as u32,
            })
        },
    )
    .unwrap_or_default()
}

pub fn overview() -> OverviewPayload {
    OverviewPayload {
        product: "FlakeSting by PatchHive".into(),
        tagline: "Detect, isolate, and explain flaky CI patterns before unreliable checks erode team trust.".into(),
        counts: overview_counts(),
        recent_scans: history(6),
    }
}

pub fn latest_comparable_scan(
    repo: &str,
    branch: &str,
    workflow_name: &str,
) -> Option<FlakeScanResult> {
    let conn = connect().ok()?;
    latest_comparable_scan_conn(&conn, repo, branch, workflow_name)
}

fn latest_comparable_scan_conn(
    conn: &Connection,
    repo: &str,
    branch: &str,
    workflow_name: &str,
) -> Option<FlakeScanResult> {
    let payload = conn
        .query_row(
            r#"
            SELECT payload
            FROM flake_scans
            WHERE repo = ?1 AND branch = ?2 AND workflow_name = ?3
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            params![repo, branch, workflow_name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()?;

    serde_json::from_str(&payload).ok()
}

fn previous_comparable_scan_before(
    conn: &Connection,
    repo: &str,
    branch: &str,
    workflow_name: &str,
    created_at: &str,
) -> Option<FlakeScanResult> {
    let payload = conn
        .query_row(
            r#"
            SELECT payload
            FROM flake_scans
            WHERE repo = ?1
              AND branch = ?2
              AND workflow_name = ?3
              AND created_at < ?4
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            params![repo, branch, workflow_name, created_at],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()?;

    serde_json::from_str(&payload).ok()
}

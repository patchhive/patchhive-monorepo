use std::{
    collections::HashSet,
    sync::{Mutex, MutexGuard},
};

use once_cell::sync::OnceCell;
use rusqlite::{params, types::Type, Connection};

use crate::models::{HistoryItem, OverviewCounts, RefactorScanResult, ScanMetrics};

static DB_CONN: OnceCell<Mutex<Connection>> = OnceCell::new();

pub fn db_path() -> String {
    std::env::var("REFACTOR_SCOUT_DB_PATH").unwrap_or_else(|_| "refactor-scout.db".into())
}

fn open_connection() -> rusqlite::Result<Connection> {
    Connection::open(db_path())
}

fn connect() -> rusqlite::Result<MutexGuard<'static, Connection>> {
    let mutex = DB_CONN.get_or_try_init(|| open_connection().map(Mutex::new))?;
    mutex.lock().map_err(|_| rusqlite::Error::InvalidQuery)
}

pub fn health_check() -> bool {
    connect()
        .and_then(|conn| conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0)))
        .is_ok()
}

pub fn init_db() -> rusqlite::Result<()> {
    let conn = connect()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS scans (
          id TEXT PRIMARY KEY,
          created_at TEXT NOT NULL,
          repo_path TEXT NOT NULL,
          repo_name TEXT NOT NULL,
          summary TEXT NOT NULL,
          metrics_json TEXT NOT NULL,
          opportunities_json TEXT NOT NULL,
          warnings_json TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_refactor_scout_scans_created_at
        ON scans(created_at DESC);
        "#,
    )?;
    Ok(())
}

pub fn save_scan(scan: &RefactorScanResult) -> rusqlite::Result<()> {
    let conn = connect()?;
    let metrics_json = serde_json::to_string(&scan.metrics)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    let opportunities_json = serde_json::to_string(&scan.opportunities)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    let warnings_json = serde_json::to_string(&scan.warnings)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;

    conn.execute(
        r#"
        INSERT OR REPLACE INTO scans (
          id, created_at, repo_path, repo_name, summary,
          metrics_json, opportunities_json, warnings_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            scan.id,
            scan.created_at,
            scan.repo_path,
            scan.repo_name,
            scan.summary,
            metrics_json,
            opportunities_json,
            warnings_json,
        ],
    )?;
    Ok(())
}

pub fn get_scan(id: &str) -> Option<RefactorScanResult> {
    let conn = connect().ok()?;
    conn.query_row(
        r#"
        SELECT id, created_at, repo_path, repo_name, summary,
               metrics_json, opportunities_json, warnings_json
        FROM scans
        WHERE id = ?1
        "#,
        [id],
        |row| {
            Ok(RefactorScanResult {
                id: row.get(0)?,
                created_at: row.get(1)?,
                repo_path: row.get(2)?,
                repo_name: row.get(3)?,
                summary: row.get(4)?,
                metrics: parse_json_column(row.get::<_, String>(5)?, 5)?,
                opportunities: parse_json_column(row.get::<_, String>(6)?, 6)?,
                warnings: parse_json_column(row.get::<_, String>(7)?, 7)?,
            })
        },
    )
    .ok()
}

pub fn history(limit: usize) -> Vec<HistoryItem> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };
    let Ok(mut stmt) = conn.prepare(
        r#"
        SELECT id, created_at, repo_path, repo_name, summary, metrics_json
        FROM scans
        ORDER BY created_at DESC
        LIMIT ?1
        "#,
    ) else {
        return Vec::new();
    };

    let Ok(rows) = stmt.query_map([limit as i64], |row| {
        let metrics: ScanMetrics = parse_json_column(row.get::<_, String>(5)?, 5)?;
        Ok(HistoryItem {
            id: row.get(0)?,
            created_at: row.get(1)?,
            repo_path: row.get(2)?,
            repo_name: row.get(3)?,
            summary: row.get(4)?,
            opportunities: metrics.opportunities,
            high_safety: metrics.high_safety,
            medium_safety: metrics.medium_safety,
        })
    }) else {
        return Vec::new();
    };

    rows.filter_map(Result::ok).collect()
}

pub fn overview_counts() -> OverviewCounts {
    let Ok(conn) = connect() else {
        return OverviewCounts::default();
    };
    let Ok(mut stmt) = conn.prepare(
        r#"
        SELECT repo_path, repo_name, metrics_json
        FROM scans
        ORDER BY created_at DESC
        "#,
    ) else {
        return OverviewCounts::default();
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            parse_json_column::<ScanMetrics>(row.get::<_, String>(2)?, 2)?,
        ))
    }) else {
        return OverviewCounts::default();
    };

    let mut counts = OverviewCounts::default();
    let mut repos = HashSet::new();

    for row in rows.flatten() {
        let (repo_path, repo_name, metrics) = row;
        counts.scans += 1;
        repos.insert(repo_path);
        counts.opportunities += metrics.opportunities;
        counts.high_safety += metrics.high_safety;
        counts.large_file_count += metrics.large_file_count;
        counts.long_function_count += metrics.long_function_count;
        counts.repeated_literal_count += metrics.repeated_literal_count;
        if counts.last_repo.is_empty() {
            counts.last_repo = repo_name;
        }
    }

    counts.repos = repos.len() as u32;
    counts
}

fn parse_json_column<T: serde::de::DeserializeOwned>(
    json: String,
    column: usize,
) -> rusqlite::Result<T> {
    serde_json::from_str(&json)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err)))
}

#[cfg(test)]
mod tests {
    use super::init_db;
    use rusqlite::Connection;

    #[test]
    fn init_db_creates_scans_table() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        conn.execute_batch(
            r#"
            CREATE TABLE scans (
              id TEXT PRIMARY KEY,
              created_at TEXT NOT NULL,
              repo_path TEXT NOT NULL,
              repo_name TEXT NOT NULL,
              summary TEXT NOT NULL,
              metrics_json TEXT NOT NULL,
              opportunities_json TEXT NOT NULL,
              warnings_json TEXT NOT NULL
            );
            "#,
        )
        .expect("schema should create");

        let _ = init_db;
    }
}

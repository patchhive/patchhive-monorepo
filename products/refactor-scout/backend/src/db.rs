use std::collections::HashSet;

use once_cell::sync::Lazy;
use patchhive_product_core::sqlite::{PooledSqliteConnection, SqlitePool};
use rusqlite::{params, types::Type};

use crate::models::{HistoryItem, OverviewCounts, RefactorScanResult, ScanMetrics};

static DB_POOL: Lazy<SqlitePool> = Lazy::new(|| {
    SqlitePool::new(db_path(), "RefactorScout").with_pool_size_env("REFACTOR_SCOUT_DB_POOL_SIZE")
});

pub fn db_path() -> String {
    std::env::var("REFACTOR_SCOUT_DB_PATH").unwrap_or_else(|_| "refactor-scout.db".into())
}

fn connect() -> rusqlite::Result<PooledSqliteConnection<'static>> {
    DB_POOL.get()
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
    let warnings_json = serde_json::to_string(&normalize_warnings(scan.warnings.clone()))
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    let summary = normalize_summary(scan.summary.clone());

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
            summary,
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
            Ok(normalize_scan(RefactorScanResult {
                id: row.get(0)?,
                created_at: row.get(1)?,
                repo_path: row.get(2)?,
                repo_name: row.get(3)?,
                summary: row.get(4)?,
                metrics: parse_json_column(row.get::<_, String>(5)?, 5)?,
                opportunities: parse_json_column(row.get::<_, String>(6)?, 6)?,
                warnings: parse_json_column(row.get::<_, String>(7)?, 7)?,
            }))
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
            summary: normalize_summary(row.get(4)?),
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
        counts.medium_safety += metrics.medium_safety;
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

fn normalize_scan(mut scan: RefactorScanResult) -> RefactorScanResult {
    scan.summary = normalize_summary(scan.summary);
    scan.warnings = normalize_warnings(scan.warnings);
    scan
}

fn normalize_summary(summary: String) -> String {
    let trimmed = summary.trim_end();
    if trimmed.ends_with("..") {
        format!("{}.", trimmed.trim_end_matches('.'))
    } else {
        summary
    }
}

fn normalize_warnings(warnings: Vec<String>) -> Vec<String> {
    warnings
        .into_iter()
        .filter(|warning| !generated_cache_warning(warning))
        .collect()
}

fn generated_cache_warning(warning: &str) -> bool {
    warning.contains("/.vite/") || warning.contains("\\.vite\\")
}

#[cfg(test)]
mod tests {
    use super::{init_db, normalize_summary, normalize_warnings};
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

    #[test]
    fn normalize_summary_collapses_legacy_double_period() {
        assert_eq!(
            normalize_summary("Strongest lead: extract helper..".into()),
            "Strongest lead: extract helper."
        );
    }

    #[test]
    fn normalize_warnings_drops_generated_vite_cache_noise() {
        let warnings = normalize_warnings(vec![
            "Skipped /repo/.vite/deps/react-dom_client.js because it is larger than 341 KB.".into(),
            "Scan stopped after 250 supported files.".into(),
        ]);

        assert_eq!(warnings, vec!["Scan stopped after 250 supported files."]);
    }
}

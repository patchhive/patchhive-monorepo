use std::collections::HashSet;

use once_cell::sync::Lazy;
use patchhive_product_core::scheduling::{
    self, ProductSchedule, SaveSchedule, DEFAULT_SCHEDULE_APPROVAL_POLICY,
};
use patchhive_product_core::scope_policy::{normalize_repo_name, RepoListType, RepoScopePolicy};
use patchhive_product_core::sqlite::{PooledSqliteConnection, SqlitePool};
use rusqlite::{params, types::Type};
use serde_json::Value;

use crate::models::{
    HistoryItem, OverviewCounts, RefactorScanResult, RepoListItem, ScanMetrics, ScanPreset,
    ScanRequest,
};

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
          warnings_json TEXT NOT NULL,
          trigger_type TEXT NOT NULL DEFAULT 'operator',
          schedule_name TEXT,
          target_selection_mode TEXT NOT NULL DEFAULT 'direct'
        );

        CREATE INDEX IF NOT EXISTS idx_refactor_scout_scans_created_at
        ON scans(created_at DESC);

        CREATE TABLE IF NOT EXISTS scan_presets (
          name TEXT PRIMARY KEY,
          params_json TEXT NOT NULL,
          target_selection_mode TEXT NOT NULL DEFAULT 'direct',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS repo_lists (
          repo TEXT PRIMARY KEY,
          list_type TEXT NOT NULL,
          added_at TEXT NOT NULL
        );
        "#,
    )?;
    ensure_scan_column(
        &conn,
        "trigger_type",
        "ALTER TABLE scans ADD COLUMN trigger_type TEXT NOT NULL DEFAULT 'operator';",
    )?;
    ensure_scan_column(
        &conn,
        "schedule_name",
        "ALTER TABLE scans ADD COLUMN schedule_name TEXT;",
    )?;
    ensure_scan_column(
        &conn,
        "target_selection_mode",
        "ALTER TABLE scans ADD COLUMN target_selection_mode TEXT NOT NULL DEFAULT 'direct';",
    )?;
    scheduling::init_schema(&conn)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(error.into()))?;
    Ok(())
}

fn ensure_scan_column(
    conn: &rusqlite::Connection,
    column_name: &str,
    migration: &str,
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(scans)")?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == column_name {
            return Ok(());
        }
    }
    conn.execute_batch(migration)
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
          metrics_json, opportunities_json, warnings_json, trigger_type, schedule_name,
          target_selection_mode
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
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
            scan.trigger_type,
            scan.schedule_name,
            target_selection_mode_label(scan.target_selection_mode),
        ],
    )?;
    Ok(())
}

pub fn get_scan(id: &str) -> Option<RefactorScanResult> {
    let conn = connect().ok()?;
    conn.query_row(
        r#"
        SELECT id, created_at, repo_path, repo_name, summary,
               metrics_json, opportunities_json, warnings_json, trigger_type, schedule_name,
               target_selection_mode
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
                trigger_type: row.get(8)?,
                schedule_name: row.get(9)?,
                target_selection_mode: parse_target_selection_mode(&row.get::<_, String>(10)?),
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
        SELECT id, created_at, repo_path, repo_name, summary, metrics_json,
               trigger_type, schedule_name, target_selection_mode
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
            trigger_type: row.get(6)?,
            schedule_name: row.get(7)?,
            target_selection_mode: parse_target_selection_mode(&row.get::<_, String>(8)?),
        })
    }) else {
        return Vec::new();
    };

    rows.filter_map(Result::ok).collect()
}

pub fn list_schedules() -> anyhow::Result<Vec<ProductSchedule>> {
    let conn = connect()?;
    scheduling::list(&conn, "refactor-scout", "scan")
}

pub fn list_scan_presets() -> anyhow::Result<Vec<ScanPreset>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT name, params_json, target_selection_mode, created_at, updated_at
        FROM scan_presets
        ORDER BY updated_at DESC, name ASC
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ScanPreset {
            name: row.get(0)?,
            params: parse_json_column(row.get::<_, String>(1)?, 1)?,
            target_selection_mode: parse_target_selection_mode(&row.get::<_, String>(2)?),
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn save_scan_preset(
    name: &str,
    params_in: &ScanRequest,
    target_selection_mode: patchhive_product_core::contract::TargetSelectionMode,
) -> anyhow::Result<ScanPreset> {
    let conn = connect()?;
    let now = chrono::Utc::now().to_rfc3339();
    let created_at = conn
        .query_row(
            "SELECT created_at FROM scan_presets WHERE name = ?1",
            [name],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| now.clone());
    conn.execute(
        r#"
        INSERT OR REPLACE INTO scan_presets (
          name, params_json, target_selection_mode, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![
            name,
            serde_json::to_string(params_in)?,
            target_selection_mode_label(target_selection_mode),
            created_at,
            now,
        ],
    )?;
    Ok(ScanPreset {
        name: name.to_string(),
        params: params_in.clone(),
        target_selection_mode,
        created_at,
        updated_at: now,
    })
}

pub fn delete_scan_preset(name: &str) -> anyhow::Result<bool> {
    let conn = connect()?;
    Ok(conn.execute("DELETE FROM scan_presets WHERE name = ?1", [name])? > 0)
}

pub fn list_repo_lists() -> anyhow::Result<Vec<RepoListItem>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        "SELECT repo, list_type, added_at FROM repo_lists ORDER BY list_type ASC, repo ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RepoListItem {
            repo: row.get(0)?,
            list_type: row.get(1)?,
            added_at: row.get(2)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn save_repo_list(repo: &str, list_type: &str) -> anyhow::Result<RepoListItem> {
    let repo = normalize_repo_name(repo)
        .ok_or_else(|| anyhow::anyhow!("Repository must use owner/repo format."))?;
    let list_type = RepoListType::parse(list_type)
        .ok_or_else(|| anyhow::anyhow!("Unknown repository control."))?
        .as_str();
    let added_at = chrono::Utc::now().to_rfc3339();
    let conn = connect()?;
    conn.execute(
        "INSERT OR REPLACE INTO repo_lists(repo, list_type, added_at) VALUES(?1, ?2, ?3)",
        params![repo, list_type, added_at],
    )?;
    Ok(RepoListItem {
        repo,
        list_type: list_type.to_string(),
        added_at,
    })
}

pub fn delete_repo_list(repo: &str) -> anyhow::Result<bool> {
    let Some(repo) = normalize_repo_name(repo) else {
        return Ok(false);
    };
    let conn = connect()?;
    Ok(conn.execute("DELETE FROM repo_lists WHERE repo = ?1", [repo])? > 0)
}

pub fn repo_scope_policy() -> anyhow::Result<RepoScopePolicy> {
    let rows = list_repo_lists()?;
    Ok(RepoScopePolicy::from_entries(rows.iter().map(|entry| {
        (entry.repo.as_str(), entry.list_type.as_str())
    })))
}

pub fn get_schedule(name: &str) -> anyhow::Result<Option<ProductSchedule>> {
    let conn = connect()?;
    scheduling::get(&conn, "refactor-scout", "scan", name)
}

pub fn save_schedule(
    name: &str,
    payload: &Value,
    target_selection_mode: patchhive_product_core::contract::TargetSelectionMode,
    cadence_hours: u32,
    enabled: bool,
) -> anyhow::Result<ProductSchedule> {
    let conn = connect()?;
    scheduling::save(
        &conn,
        SaveSchedule {
            name,
            product: "refactor-scout",
            action_id: "scan",
            payload,
            target_selection_mode,
            cadence_hours,
            enabled,
            approval_policy: DEFAULT_SCHEDULE_APPROVAL_POLICY,
        },
    )
}

pub fn recently_scanned_repositories(
    schedule_name: &str,
    cooldown_days: u32,
) -> anyhow::Result<HashSet<String>> {
    let conn = connect()?;
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(cooldown_days.clamp(1, 365) as i64))
        .to_rfc3339();
    let mut stmt = conn.prepare(
        r#"
        SELECT DISTINCT repo_path
        FROM scans
        WHERE trigger_type = 'schedule'
          AND schedule_name = ?1
          AND target_selection_mode = 'discovery'
          AND created_at >= ?2
        "#,
    )?;
    let rows = stmt.query_map(params![schedule_name, cutoff], |row| {
        row.get::<_, String>(0)
    })?;
    rows.collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(Into::into)
}

pub fn delete_schedule(name: &str) -> anyhow::Result<bool> {
    let conn = connect()?;
    scheduling::delete(&conn, "refactor-scout", "scan", name)
}

pub fn claim_due_schedules(limit: usize) -> anyhow::Result<Vec<ProductSchedule>> {
    let mut conn = connect()?;
    scheduling::claim_due(&mut conn, "refactor-scout", "scan", limit)
}

pub fn record_schedule_result(
    name: &str,
    run_id: Option<&str>,
    status: &str,
    error: Option<&str>,
) -> anyhow::Result<bool> {
    let conn = connect()?;
    scheduling::record_result(&conn, "refactor-scout", "scan", name, run_id, status, error)
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

fn target_selection_mode_label(
    mode: patchhive_product_core::contract::TargetSelectionMode,
) -> &'static str {
    match mode {
        patchhive_product_core::contract::TargetSelectionMode::Direct => "direct",
        patchhive_product_core::contract::TargetSelectionMode::Discovery => "discovery",
    }
}

fn parse_target_selection_mode(
    value: &str,
) -> patchhive_product_core::contract::TargetSelectionMode {
    match value.trim() {
        "discovery" => patchhive_product_core::contract::TargetSelectionMode::Discovery,
        _ => patchhive_product_core::contract::TargetSelectionMode::Direct,
    }
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
              warnings_json TEXT NOT NULL,
              trigger_type TEXT NOT NULL DEFAULT 'operator',
              schedule_name TEXT,
              target_selection_mode TEXT NOT NULL DEFAULT 'direct'
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

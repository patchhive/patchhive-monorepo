use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use rusqlite::Connection;
use std::sync::{Mutex, MutexGuard};

static DB_CONN: OnceCell<Mutex<Connection>> = OnceCell::new();

pub fn db_path() -> String {
    std::env::var("SIGNAL_DB_PATH").unwrap_or_else(|_| "signal-hive.db".into())
}

pub(crate) fn open_connection() -> Result<Connection> {
    let conn = Connection::open(db_path()).context("failed to open SignalHive database")?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .context("failed to initialize SignalHive database pragmas")?;
    Ok(conn)
}

pub fn connect() -> Result<MutexGuard<'static, Connection>> {
    let mutex = DB_CONN.get_or_try_init(|| open_connection().map(Mutex::new))?;
    mutex
        .lock()
        .map_err(|_| anyhow!("SignalHive database mutex poisoned"))
}

pub fn health_check() -> bool {
    connect()
        .and_then(|conn| {
            conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                .context("failed to query SignalHive database")
        })
        .is_ok()
}

#[derive(Clone, Copy)]
pub(crate) enum MigrationColumn {
    ScansParamsSignature,
    ScansWarningsJson,
    ScansTriggerType,
    ScansScheduleName,
    RepoSignalsSampledIssues,
    RepoSignalsUnlabeledIssues,
    RepoSignalsStaleBugIssues,
    RepoSignalsStaleHighCommentIssues,
    RepoSignalsScoreBreakdownJson,
    RepoSignalsRecurringBugClustersJson,
    RepoSignalsTodoAvailable,
    RepoSignalsFixmeAvailable,
    RepoSignalsWarningsJson,
}

impl MigrationColumn {
    pub(crate) fn column_name(self) -> &'static str {
        match self {
            Self::ScansParamsSignature => "params_signature",
            Self::ScansWarningsJson | Self::RepoSignalsWarningsJson => "warnings_json",
            Self::ScansTriggerType => "trigger_type",
            Self::ScansScheduleName => "schedule_name",
            Self::RepoSignalsSampledIssues => "sampled_issues",
            Self::RepoSignalsUnlabeledIssues => "unlabeled_issues",
            Self::RepoSignalsStaleBugIssues => "stale_bug_issues",
            Self::RepoSignalsStaleHighCommentIssues => "stale_high_comment_issues",
            Self::RepoSignalsScoreBreakdownJson => "score_breakdown_json",
            Self::RepoSignalsRecurringBugClustersJson => "recurring_bug_clusters_json",
            Self::RepoSignalsTodoAvailable => "todo_available",
            Self::RepoSignalsFixmeAvailable => "fixme_available",
        }
    }

    pub(crate) fn table_info_sql(self) -> &'static str {
        match self {
            Self::ScansParamsSignature
            | Self::ScansWarningsJson
            | Self::ScansTriggerType
            | Self::ScansScheduleName => "PRAGMA table_info(scans)",
            Self::RepoSignalsSampledIssues
            | Self::RepoSignalsUnlabeledIssues
            | Self::RepoSignalsStaleBugIssues
            | Self::RepoSignalsStaleHighCommentIssues
            | Self::RepoSignalsScoreBreakdownJson
            | Self::RepoSignalsRecurringBugClustersJson
            | Self::RepoSignalsTodoAvailable
            | Self::RepoSignalsFixmeAvailable
            | Self::RepoSignalsWarningsJson => "PRAGMA table_info(repo_signals)",
        }
    }

    pub(crate) fn add_column_sql(self) -> &'static str {
        match self {
            Self::ScansParamsSignature => {
                "ALTER TABLE scans ADD COLUMN params_signature TEXT NOT NULL DEFAULT '';"
            }
            Self::ScansWarningsJson => {
                "ALTER TABLE scans ADD COLUMN warnings_json TEXT NOT NULL DEFAULT '[]';"
            }
            Self::ScansTriggerType => {
                "ALTER TABLE scans ADD COLUMN trigger_type TEXT NOT NULL DEFAULT 'manual';"
            }
            Self::ScansScheduleName => "ALTER TABLE scans ADD COLUMN schedule_name TEXT;",
            Self::RepoSignalsSampledIssues => {
                "ALTER TABLE repo_signals ADD COLUMN sampled_issues INTEGER NOT NULL DEFAULT 0;"
            }
            Self::RepoSignalsUnlabeledIssues => {
                "ALTER TABLE repo_signals ADD COLUMN unlabeled_issues INTEGER NOT NULL DEFAULT 0;"
            }
            Self::RepoSignalsStaleBugIssues => {
                "ALTER TABLE repo_signals ADD COLUMN stale_bug_issues INTEGER NOT NULL DEFAULT 0;"
            }
            Self::RepoSignalsStaleHighCommentIssues => {
                "ALTER TABLE repo_signals ADD COLUMN stale_high_comment_issues INTEGER NOT NULL DEFAULT 0;"
            }
            Self::RepoSignalsScoreBreakdownJson => {
                "ALTER TABLE repo_signals ADD COLUMN score_breakdown_json TEXT NOT NULL DEFAULT '[]';"
            }
            Self::RepoSignalsRecurringBugClustersJson => {
                "ALTER TABLE repo_signals ADD COLUMN recurring_bug_clusters_json TEXT NOT NULL DEFAULT '[]';"
            }
            Self::RepoSignalsTodoAvailable => {
                "ALTER TABLE repo_signals ADD COLUMN todo_available INTEGER NOT NULL DEFAULT 1;"
            }
            Self::RepoSignalsFixmeAvailable => {
                "ALTER TABLE repo_signals ADD COLUMN fixme_available INTEGER NOT NULL DEFAULT 1;"
            }
            Self::RepoSignalsWarningsJson => {
                "ALTER TABLE repo_signals ADD COLUMN warnings_json TEXT NOT NULL DEFAULT '[]';"
            }
        }
    }
}

fn column_exists(conn: &Connection, column: MigrationColumn) -> anyhow::Result<bool> {
    let mut stmt = conn.prepare(column.table_info_sql())?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for row in rows {
        if row? == column.column_name() {
            return Ok(true);
        }
    }

    Ok(false)
}

fn ensure_column(conn: &Connection, column: MigrationColumn) -> anyhow::Result<()> {
    if !column_exists(conn, column)? {
        conn.execute_batch(column.add_column_sql())?;
    }
    Ok(())
}

pub(crate) fn init_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS scans (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            params_signature TEXT NOT NULL DEFAULT '',
            warnings_json TEXT NOT NULL DEFAULT '[]',
            trigger_type TEXT NOT NULL DEFAULT 'manual',
            schedule_name TEXT,
            search_query TEXT NOT NULL,
            topics_json TEXT NOT NULL,
            languages_json TEXT NOT NULL,
            min_stars INTEGER NOT NULL,
            max_repos INTEGER NOT NULL,
            issues_per_repo INTEGER NOT NULL,
            stale_days INTEGER NOT NULL,
            total_repos INTEGER NOT NULL,
            total_signals INTEGER NOT NULL,
            top_repo TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS repo_signals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_id TEXT NOT NULL,
            repo_full_name TEXT NOT NULL,
            repo_url TEXT NOT NULL,
            description TEXT NOT NULL,
            language TEXT NOT NULL,
            stars INTEGER NOT NULL,
            open_issues INTEGER NOT NULL,
            sampled_issues INTEGER NOT NULL DEFAULT 0,
            stale_issues INTEGER NOT NULL,
            unlabeled_issues INTEGER NOT NULL DEFAULT 0,
            stale_bug_issues INTEGER NOT NULL DEFAULT 0,
            stale_high_comment_issues INTEGER NOT NULL DEFAULT 0,
            duplicate_candidates_json TEXT NOT NULL,
            recurring_bug_clusters_json TEXT NOT NULL DEFAULT '[]',
            todo_count INTEGER NOT NULL,
            fixme_count INTEGER NOT NULL,
            todo_available INTEGER NOT NULL DEFAULT 1,
            fixme_available INTEGER NOT NULL DEFAULT 1,
            priority_score REAL NOT NULL,
            score_breakdown_json TEXT NOT NULL DEFAULT '[]',
            summary TEXT NOT NULL,
            signals_json TEXT NOT NULL,
            issue_examples_json TEXT NOT NULL,
            warnings_json TEXT NOT NULL DEFAULT '[]'
        );

        CREATE TABLE IF NOT EXISTS repo_lists (
            repo TEXT PRIMARY KEY,
            list_type TEXT NOT NULL,
            added_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS scan_presets (
            name TEXT PRIMARY KEY,
            params_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS scan_schedules (
            name TEXT PRIMARY KEY,
            params_json TEXT NOT NULL,
            cadence_hours INTEGER NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            next_run_at TEXT NOT NULL,
            last_run_at TEXT,
            last_scan_id TEXT,
            last_status TEXT NOT NULL DEFAULT 'idle',
            last_error TEXT
        );
        "#,
    )?;

    ensure_column(conn, MigrationColumn::ScansParamsSignature)?;
    ensure_column(conn, MigrationColumn::ScansWarningsJson)?;
    ensure_column(conn, MigrationColumn::ScansTriggerType)?;
    ensure_column(conn, MigrationColumn::ScansScheduleName)?;
    ensure_column(conn, MigrationColumn::RepoSignalsSampledIssues)?;
    ensure_column(conn, MigrationColumn::RepoSignalsUnlabeledIssues)?;
    ensure_column(conn, MigrationColumn::RepoSignalsStaleBugIssues)?;
    ensure_column(conn, MigrationColumn::RepoSignalsStaleHighCommentIssues)?;
    ensure_column(conn, MigrationColumn::RepoSignalsScoreBreakdownJson)?;
    ensure_column(conn, MigrationColumn::RepoSignalsRecurringBugClustersJson)?;
    ensure_column(conn, MigrationColumn::RepoSignalsTodoAvailable)?;
    ensure_column(conn, MigrationColumn::RepoSignalsFixmeAvailable)?;
    ensure_column(conn, MigrationColumn::RepoSignalsWarningsJson)?;
    Ok(())
}

pub fn init_db() -> anyhow::Result<()> {
    let conn = connect()?;
    init_schema(&conn)
}

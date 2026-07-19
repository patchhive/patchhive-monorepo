use rusqlite::{ffi::ErrorCode, Connection, Error, Result};
use std::{
    fmt,
    path::{Path, PathBuf},
    sync::{Condvar, Mutex},
    time::Duration,
};

const DEFAULT_MAX_CONNECTIONS: usize = 4;
const DEFAULT_BUSY_TIMEOUT_SECS: u64 = 5;
const DEFAULT_PRAGMAS: &str = "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;";
const SUITE_DB_PATH_ENV: &str = "PATCHHIVE_DB_PATH";

struct PoolState {
    idle: Vec<Connection>,
    open_connections: usize,
}

pub struct SqlitePool {
    path: PathBuf,
    label: String,
    max_connections: usize,
    busy_timeout: Duration,
    pragmas: String,
    state: Mutex<PoolState>,
    available: Condvar,
}

pub struct PooledSqliteConnection<'a> {
    pool: &'a SqlitePool,
    conn: Option<Connection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteOperatorIssue {
    Busy,
    Locked,
    CannotOpen,
    PermissionDenied,
    ReadOnly,
    DiskFull,
    IoFailure,
    Corrupt,
    NotDatabase,
    SchemaChanged,
    Unknown,
}

impl SqliteOperatorIssue {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::Locked => "locked",
            Self::CannotOpen => "cannot_open",
            Self::PermissionDenied => "permission_denied",
            Self::ReadOnly => "read_only",
            Self::DiskFull => "disk_full",
            Self::IoFailure => "io_failure",
            Self::Corrupt => "corrupt",
            Self::NotDatabase => "not_database",
            Self::SchemaChanged => "schema_changed",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for SqliteOperatorIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl SqlitePool {
    pub fn new(path: impl Into<PathBuf>, label: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            label: label.into(),
            max_connections: configured_pool_size(None),
            busy_timeout: Duration::from_secs(DEFAULT_BUSY_TIMEOUT_SECS),
            pragmas: DEFAULT_PRAGMAS.into(),
            state: Mutex::new(PoolState {
                idle: Vec::new(),
                open_connections: 0,
            }),
            available: Condvar::new(),
        }
    }

    pub fn with_pool_size_env(mut self, env_var: impl AsRef<str>) -> Self {
        self.max_connections = configured_pool_size(Some(env_var.as_ref()));
        self
    }

    pub fn with_max_connections(mut self, max_connections: usize) -> Self {
        self.max_connections = max_connections.max(1);
        self
    }

    pub fn with_busy_timeout(mut self, busy_timeout: Duration) -> Self {
        self.busy_timeout = busy_timeout;
        self
    }

    pub fn with_pragmas(mut self, pragmas: impl Into<String>) -> Self {
        self.pragmas = pragmas.into();
        self
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn operator_error_message(&self, err: &Error) -> String {
        operator_error_message(&self.label, &self.path, err)
    }

    pub fn db_path_message(&self) -> String {
        db_path_message(&self.label, &self.path)
    }

    pub fn backup_guidance(&self) -> String {
        backup_guidance(&self.label, &self.path)
    }

    pub fn migration_guidance(&self) -> String {
        migration_guidance(&self.label, &self.path)
    }

    pub fn get(&self) -> Result<PooledSqliteConnection<'_>> {
        loop {
            let mut state = self.state.lock().map_err(|_| Error::InvalidQuery)?;
            if let Some(conn) = state.idle.pop() {
                return Ok(PooledSqliteConnection {
                    pool: self,
                    conn: Some(conn),
                });
            }

            if state.open_connections < self.max_connections {
                state.open_connections += 1;
                drop(state);

                match self.open_connection() {
                    Ok(conn) => {
                        return Ok(PooledSqliteConnection {
                            pool: self,
                            conn: Some(conn),
                        });
                    }
                    Err(err) => {
                        let mut state = self.state.lock().map_err(|_| Error::InvalidQuery)?;
                        state.open_connections = state.open_connections.saturating_sub(1);
                        self.available.notify_one();
                        return Err(err);
                    }
                }
            }

            drop(
                self.available
                    .wait(state)
                    .map_err(|_| Error::InvalidQuery)?,
            );
        }
    }

    fn open_connection(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.busy_timeout(self.busy_timeout)?;
        if !self.pragmas.trim().is_empty() {
            conn.execute_batch(&self.pragmas)?;
        }
        tracing::trace!(
            product = %self.label,
            path = %self.path.display(),
            "opened pooled sqlite connection"
        );
        Ok(conn)
    }
}

pub fn classify_error(err: &Error) -> SqliteOperatorIssue {
    match err.sqlite_error_code() {
        Some(ErrorCode::DatabaseBusy) => SqliteOperatorIssue::Busy,
        Some(ErrorCode::DatabaseLocked) => SqliteOperatorIssue::Locked,
        Some(ErrorCode::CannotOpen) => SqliteOperatorIssue::CannotOpen,
        Some(ErrorCode::PermissionDenied) => SqliteOperatorIssue::PermissionDenied,
        Some(ErrorCode::ReadOnly) => SqliteOperatorIssue::ReadOnly,
        Some(ErrorCode::DiskFull) => SqliteOperatorIssue::DiskFull,
        Some(ErrorCode::SystemIoFailure) => SqliteOperatorIssue::IoFailure,
        Some(ErrorCode::DatabaseCorrupt) => SqliteOperatorIssue::Corrupt,
        Some(ErrorCode::NotADatabase) => SqliteOperatorIssue::NotDatabase,
        Some(ErrorCode::SchemaChanged) => SqliteOperatorIssue::SchemaChanged,
        _ => SqliteOperatorIssue::Unknown,
    }
}

pub fn operator_error_message(
    label: impl AsRef<str>,
    path: impl AsRef<Path>,
    err: &Error,
) -> String {
    let label = label.as_ref();
    let path = path.as_ref();
    let issue = classify_error(err);
    let detail = err.to_string();
    match issue {
        SqliteOperatorIssue::Busy | SqliteOperatorIssue::Locked => format!(
            "{label} SQLite database is {issue} at {}. Another process is holding a write lock; let active runs finish, close duplicate dev servers, or restart the product if the lock is stale. SQLite detail: {detail}",
            path.display()
        ),
        SqliteOperatorIssue::CannotOpen
        | SqliteOperatorIssue::PermissionDenied
        | SqliteOperatorIssue::ReadOnly => format!(
            "{label} cannot write its SQLite database at {}. Check that the directory exists and the current user owns the database, -wal, and -shm files. SQLite detail: {detail}",
            path.display()
        ),
        SqliteOperatorIssue::DiskFull | SqliteOperatorIssue::IoFailure => format!(
            "{label} hit a disk or filesystem problem while using SQLite at {}. Check free space, mount health, and container volume permissions before rerunning. SQLite detail: {detail}",
            path.display()
        ),
        SqliteOperatorIssue::Corrupt | SqliteOperatorIssue::NotDatabase => format!(
            "{label} SQLite database at {} looks corrupt or is not a SQLite file. Stop the product, copy the database plus -wal/-shm files for inspection, then restore from backup or rebuild the local cache. SQLite detail: {detail}",
            path.display()
        ),
        SqliteOperatorIssue::SchemaChanged => format!(
            "{label} saw a SQLite schema change while using {}. Let startup migrations finish before running product actions. SQLite detail: {detail}",
            path.display()
        ),
        SqliteOperatorIssue::Unknown => format!(
            "{label} SQLite database error at {}: {detail}",
            path.display()
        ),
    }
}

pub fn db_path_message(label: impl AsRef<str>, path: impl AsRef<Path>) -> String {
    let label = label.as_ref();
    let path = path.as_ref();
    format!(
        "{label} SQLite database: {}. Back up this file with any matching -wal and -shm files before manual migrations or cleanup.",
        path.display()
    )
}

pub fn backup_guidance(label: impl AsRef<str>, path: impl AsRef<Path>) -> String {
    let label = label.as_ref();
    let path = path.as_ref();
    format!(
        "Before changing {label} storage, stop the product and copy {}, {}-wal, and {}-shm when those companion files exist.",
        path.display(),
        path.display(),
        path.display()
    )
}

pub fn migration_guidance(label: impl AsRef<str>, path: impl AsRef<Path>) -> String {
    let label = label.as_ref();
    let path = path.as_ref();
    format!(
        "Run {label} SQLite migrations during startup with one product process active. If migration fails, preserve {} plus -wal/-shm files before retrying.",
        path.display()
    )
}

/// Resolves a product database path without letting the unified backend create
/// a different database for every working directory.
///
/// `PATCHHIVE_DB_PATH` intentionally wins when it is configured because the
/// suite backend mounts multiple product engines into one process and one
/// backend-owned SQLite file. Standalone product processes retain their
/// product-specific path and historical filename fallback.
pub fn product_db_path(product_env_var: &str, standalone_default: &str) -> String {
    product_db_path_with(product_env_var, standalone_default, |key| {
        std::env::var(key).ok()
    })
}

fn product_db_path_with(
    product_env_var: &str,
    standalone_default: &str,
    read_env: impl Fn(&str) -> Option<String>,
) -> String {
    read_env(SUITE_DB_PATH_ENV)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| read_env(product_env_var).filter(|value| !value.trim().is_empty()))
        .unwrap_or_else(|| standalone_default.to_string())
}

impl Drop for PooledSqliteConnection<'_> {
    fn drop(&mut self) {
        let Some(conn) = self.conn.take() else {
            return;
        };

        if let Ok(mut state) = self.pool.state.lock() {
            state.idle.push(conn);
            self.pool.available.notify_one();
        }
    }
}

impl std::ops::Deref for PooledSqliteConnection<'_> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.conn
            .as_ref()
            .expect("pooled sqlite connection missing during deref")
    }
}

impl std::ops::DerefMut for PooledSqliteConnection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn
            .as_mut()
            .expect("pooled sqlite connection missing during mutable deref")
    }
}

fn configured_pool_size(product_env_var: Option<&str>) -> usize {
    product_env_var
        .and_then(read_pool_size)
        .or_else(|| read_pool_size("PATCHHIVE_DB_POOL_SIZE"))
        .unwrap_or(DEFAULT_MAX_CONNECTIONS)
        .max(1)
}

fn read_pool_size(env_var: &str) -> Option<usize> {
    std::env::var(env_var)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
}

#[cfg(test)]
mod tests {
    use super::{
        backup_guidance, classify_error, db_path_message, operator_error_message,
        product_db_path_with, SqliteOperatorIssue, SqlitePool,
    };
    use rusqlite::{ffi::ErrorCode, Error};

    #[test]
    fn returns_connections_to_the_pool() {
        let path = std::env::temp_dir().join(format!(
            "patchhive-core-pool-test-{}.db",
            uuid::Uuid::new_v4()
        ));
        let pool = SqlitePool::new(&path, "test").with_max_connections(2);

        {
            let conn = pool.get().expect("connection should open");
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS pool_test (id INTEGER PRIMARY KEY, name TEXT);",
            )
            .expect("schema should initialize");
            conn.execute("INSERT INTO pool_test (name) VALUES ('ok')", [])
                .expect("insert should work");
        }

        let conn = pool.get().expect("connection should be reused");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pool_test", [], |row| row.get(0))
            .expect("query should work");
        assert_eq!(count, 1);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn classifies_busy_errors_for_operator_copy() {
        let err = Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::DatabaseBusy,
                extended_code: 5,
            },
            Some("database is locked".into()),
        );

        assert_eq!(classify_error(&err), SqliteOperatorIssue::Busy);
        let message = operator_error_message("DepTriage", "dep-triage.db", &err);
        assert!(message.contains("DepTriage SQLite database is busy"));
        assert!(message.contains("duplicate dev servers"));
    }

    #[test]
    fn database_path_message_includes_backup_guidance() {
        let message = db_path_message("SignalHive", "signal-hive.db");
        assert!(message.contains("SignalHive SQLite database: signal-hive.db"));
        assert!(message.contains("-wal"));
        assert!(message.contains("-shm"));

        let backup = backup_guidance("SignalHive", "signal-hive.db");
        assert!(backup.contains("signal-hive.db-wal"));
    }

    #[test]
    fn suite_database_path_wins_over_product_and_default_paths() {
        let lookup = |key: &str| match key {
            "PATCHHIVE_DB_PATH" => Some("/tmp/patchhive-suite-test.db".to_string()),
            "PRODUCT_DB_PATH" => Some("/tmp/patchhive-product-test.db".to_string()),
            _ => None,
        };
        assert_eq!(
            product_db_path_with("PRODUCT_DB_PATH", "standalone.db", lookup),
            "/tmp/patchhive-suite-test.db"
        );

        let product_only = |key: &str| {
            (key == "PRODUCT_DB_PATH").then(|| "/tmp/patchhive-product-test.db".to_string())
        };
        assert_eq!(
            product_db_path_with("PRODUCT_DB_PATH", "standalone.db", product_only),
            "/tmp/patchhive-product-test.db"
        );
        assert_eq!(
            product_db_path_with("PRODUCT_DB_PATH", "standalone.db", |_| None),
            "standalone.db"
        );
    }
}

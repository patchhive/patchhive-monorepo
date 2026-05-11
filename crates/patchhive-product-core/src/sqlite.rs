use rusqlite::{Connection, Error, Result};
use std::{
    path::{Path, PathBuf},
    sync::{Condvar, Mutex},
    time::Duration,
};

const DEFAULT_MAX_CONNECTIONS: usize = 4;
const DEFAULT_BUSY_TIMEOUT_SECS: u64 = 5;
const DEFAULT_PRAGMAS: &str = "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;";

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
    use super::SqlitePool;

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
}

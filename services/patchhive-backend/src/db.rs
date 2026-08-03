use anyhow::{Context, Result as AnyResult};
use chrono::{DateTime, Utc};
use patchhive_product_core::sqlite::SqlitePool;
use rusqlite::params;
use std::sync::Arc;

use crate::models::{RunSummary, SuiteEvent};

#[derive(Clone)]
pub struct SharedDb {
    pool: Arc<SqlitePool>,
}

impl std::fmt::Debug for SharedDb {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SharedDb")
            .field("path", &self.pool.path())
            .finish()
    }
}

impl SharedDb {
    pub fn open(path: &std::path::Path) -> AnyResult<Self> {
        let db = Self {
            pool: Arc::new(SqlitePool::new(path, "PatchHive suite")),
        };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> AnyResult<()> {
        let conn = self.pool.get().with_context(|| {
            format!(
                "could not open shared PatchHive DB at {}",
                self.pool.path().display()
            )
        })?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS suite_events (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS suite_runs (
                id TEXT PRIMARY KEY,
                product_key TEXT NOT NULL,
                status TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS product_registry_overrides (
                product_key TEXT PRIMARY KEY,
                enabled INTEGER,
                route_prefix TEXT,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS shared_config (
                key TEXT PRIMARY KEY,
                value_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub async fn ping(&self) -> bool {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.ping_sync())
            .await
            .unwrap_or(false)
    }

    fn ping_sync(&self) -> bool {
        let Ok(conn) = self.pool.get() else {
            return false;
        };
        conn.query_row("SELECT 1", [], |_| Ok(())).is_ok()
    }

    pub async fn product_override_count(&self) -> usize {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.product_override_count_sync())
            .await
            .unwrap_or(0)
    }

    fn product_override_count_sync(&self) -> usize {
        let Ok(conn) = self.pool.get() else {
            return 0;
        };
        conn.query_row(
            "SELECT COUNT(*) FROM product_registry_overrides",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count.max(0) as usize)
        .unwrap_or(0)
    }

    pub fn record_event(&self, id: &str, kind: &str, message: &str, created_at: DateTime<Utc>) {
        let conn = match self.pool.get() {
            Ok(conn) => conn,
            Err(err) => {
                tracing::warn!(%err, "could not acquire suite database connection");
                return;
            }
        };
        if let Err(err) = conn.execute(
            "INSERT OR REPLACE INTO suite_events (id, kind, message, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, kind, message, created_at.to_rfc3339()],
        ) {
            tracing::warn!(%err, "could not record suite event");
        }
    }

    pub async fn runs(&self) -> Vec<RunSummary> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.runs_sync())
            .await
            .unwrap_or_default()
    }

    fn runs_sync(&self) -> Vec<RunSummary> {
        let conn = match self.pool.get() {
            Ok(conn) => conn,
            Err(err) => {
                tracing::warn!(%err, "could not acquire suite database connection");
                return Vec::new();
            }
        };
        let mut stmt = match conn.prepare(
            "SELECT id, product_key, status, message FROM suite_runs ORDER BY created_at DESC LIMIT 100",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                tracing::warn!(%err, "could not prepare suite run query");
                return Vec::new();
            }
        };

        let rows = match stmt.query_map([], |row| {
            Ok(RunSummary {
                id: row.get(0)?,
                product_key: row.get(1)?,
                status: row.get(2)?,
                message: row.get(3)?,
            })
        }) {
            Ok(rows) => rows.filter_map(|row| row.ok()).collect(),
            Err(err) => {
                tracing::warn!(%err, "could not read suite runs");
                Vec::new()
            }
        };
        rows
    }

    pub async fn events(&self) -> Vec<SuiteEvent> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.events_sync())
            .await
            .unwrap_or_default()
    }

    fn events_sync(&self) -> Vec<SuiteEvent> {
        let conn = match self.pool.get() {
            Ok(conn) => conn,
            Err(err) => {
                tracing::warn!(%err, "could not acquire suite database connection");
                return Vec::new();
            }
        };
        let mut stmt = match conn.prepare(
            "SELECT id, kind, message, created_at FROM suite_events ORDER BY created_at DESC LIMIT 100",
        ) {
            Ok(stmt) => stmt,
            Err(err) => {
                tracing::warn!(%err, "could not prepare suite event query");
                return Vec::new();
            }
        };

        let events = match stmt.query_map([], |row| {
            let created_at_raw: String = row.get(3)?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_raw)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok(SuiteEvent {
                id: row.get(0)?,
                kind: row.get(1)?,
                message: row.get(2)?,
                created_at,
            })
        }) {
            Ok(rows) => rows.filter_map(|row| row.ok()).collect(),
            Err(err) => {
                tracing::warn!(%err, "could not read suite events");
                Vec::new()
            }
        };
        events
    }
}

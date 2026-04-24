use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use once_cell::sync::OnceCell;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{ProductActionEvent, ProductOverride, SuiteSettings};

static DB_CONN: OnceCell<Mutex<Connection>> = OnceCell::new();

pub fn db_path() -> String {
    std::env::var("HIVE_CORE_DB_PATH").unwrap_or_else(|_| "hive-core.db".into())
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
    init_schema(&conn)?;
    seed_defaults(&conn)?;
    Ok(())
}

pub fn suite_settings() -> SuiteSettings {
    let Ok(conn) = connect() else {
        return SuiteSettings::default();
    };
    load_suite_settings(&conn).unwrap_or_default()
}

pub fn save_suite_settings(settings: &SuiteSettings) -> rusqlite::Result<()> {
    let conn = connect()?;
    write_suite_settings(&conn, settings)
}

pub fn product_override_count() -> usize {
    connect()
        .ok()
        .and_then(|conn| {
            conn.query_row("SELECT COUNT(*) FROM product_overrides", [], |row| {
                row.get::<_, i64>(0)
            })
            .ok()
        })
        .unwrap_or(0) as usize
}

pub fn product_overrides() -> HashMap<String, ProductOverride> {
    let Ok(conn) = connect() else {
        return HashMap::new();
    };
    load_product_overrides(&conn).unwrap_or_default()
}

pub fn replace_product_overrides(overrides: &[ProductOverride]) -> rusqlite::Result<()> {
    let mut conn = connect()?;
    replace_overrides(&mut conn, overrides)
}

pub fn record_action_event(event: &ProductActionEvent) -> rusqlite::Result<()> {
    let conn = connect()?;
    conn.execute(
        r#"
        INSERT INTO product_action_events (
          id, product_slug, action_id, action_label, method, path, target_url,
          status, remote_status, request_json, response_json, error, created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        params![
            &event.id,
            &event.product_slug,
            &event.action_id,
            &event.action_label,
            &event.method,
            &event.path,
            &event.target_url,
            &event.status,
            event.remote_status.map(i64::from),
            event.request_json.to_string(),
            event.response_json.to_string(),
            &event.error,
            &event.created_at,
        ],
    )?;
    Ok(())
}

pub fn recent_action_events(limit: u32) -> Vec<ProductActionEvent> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };
    load_action_events(&conn, limit).unwrap_or_default()
}

pub fn action_event(id: &str) -> Option<ProductActionEvent> {
    let Ok(conn) = connect() else {
        return None;
    };
    load_action_event(&conn, id).ok().flatten()
}

fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS suite_settings (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          operator_label TEXT NOT NULL,
          mission TEXT NOT NULL,
          default_topics TEXT NOT NULL,
          default_languages TEXT NOT NULL,
          repo_allowlist TEXT NOT NULL,
          repo_denylist TEXT NOT NULL,
          opt_out_notes TEXT NOT NULL,
          preferred_launch_product TEXT NOT NULL,
          notes TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS product_overrides (
          slug TEXT PRIMARY KEY,
          frontend_url TEXT NOT NULL,
          api_url TEXT NOT NULL,
          service_token TEXT NOT NULL DEFAULT '',
          api_key TEXT NOT NULL DEFAULT '',
          enabled INTEGER NOT NULL,
          notes TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS product_action_events (
          id TEXT PRIMARY KEY,
          product_slug TEXT NOT NULL,
          action_id TEXT NOT NULL,
          action_label TEXT NOT NULL,
          method TEXT NOT NULL,
          path TEXT NOT NULL,
          target_url TEXT NOT NULL,
          status TEXT NOT NULL,
          remote_status INTEGER,
          request_json TEXT NOT NULL,
          response_json TEXT NOT NULL,
          error TEXT NOT NULL,
          created_at TEXT NOT NULL
        );
        "#,
    )?;
    migrate_schema(conn)?;
    Ok(())
}

fn migrate_schema(conn: &Connection) -> rusqlite::Result<()> {
    let columns = conn
        .prepare("PRAGMA table_info(product_overrides)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .flatten()
        .collect::<Vec<_>>();

    let has_api_key = columns.iter().any(|column| column == "api_key");
    let has_service_token = columns.iter().any(|column| column == "service_token");

    if !has_api_key {
        conn.execute(
            "ALTER TABLE product_overrides ADD COLUMN api_key TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }

    if !has_service_token {
        conn.execute(
            "ALTER TABLE product_overrides ADD COLUMN service_token TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }

    Ok(())
}

fn seed_defaults(conn: &Connection) -> rusqlite::Result<()> {
    if load_suite_settings(conn)?.operator_label.is_empty() {
        write_suite_settings(conn, &SuiteSettings::default())?;
    }
    Ok(())
}

fn load_suite_settings(conn: &Connection) -> rusqlite::Result<SuiteSettings> {
    let row = conn
        .query_row(
            r#"
            SELECT operator_label, mission, default_topics, default_languages,
                   repo_allowlist, repo_denylist, opt_out_notes,
                   preferred_launch_product, notes, updated_at
            FROM suite_settings
            WHERE id = 1
            "#,
            [],
            |row| {
                Ok(SuiteSettings {
                    operator_label: row.get(0)?,
                    mission: row.get(1)?,
                    default_topics: row.get(2)?,
                    default_languages: row.get(3)?,
                    repo_allowlist: row.get(4)?,
                    repo_denylist: row.get(5)?,
                    opt_out_notes: row.get(6)?,
                    preferred_launch_product: row.get(7)?,
                    notes: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .optional()?;

    Ok(row.unwrap_or_default())
}

fn write_suite_settings(conn: &Connection, settings: &SuiteSettings) -> rusqlite::Result<()> {
    conn.execute(
        r#"
        INSERT INTO suite_settings (
          id, operator_label, mission, default_topics, default_languages,
          repo_allowlist, repo_denylist, opt_out_notes,
          preferred_launch_product, notes, updated_at
        )
        VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(id) DO UPDATE SET
          operator_label = excluded.operator_label,
          mission = excluded.mission,
          default_topics = excluded.default_topics,
          default_languages = excluded.default_languages,
          repo_allowlist = excluded.repo_allowlist,
          repo_denylist = excluded.repo_denylist,
          opt_out_notes = excluded.opt_out_notes,
          preferred_launch_product = excluded.preferred_launch_product,
          notes = excluded.notes,
          updated_at = excluded.updated_at
        "#,
        params![
            &settings.operator_label,
            &settings.mission,
            &settings.default_topics,
            &settings.default_languages,
            &settings.repo_allowlist,
            &settings.repo_denylist,
            &settings.opt_out_notes,
            &settings.preferred_launch_product,
            &settings.notes,
            &settings.updated_at,
        ],
    )?;
    Ok(())
}

fn load_product_overrides(conn: &Connection) -> rusqlite::Result<HashMap<String, ProductOverride>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT slug, frontend_url, api_url, service_token, api_key, enabled, notes, updated_at
        FROM product_overrides
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ProductOverride {
            slug: row.get(0)?,
            frontend_url: row.get(1)?,
            api_url: row.get(2)?,
            service_token: row.get(3)?,
            legacy_api_key: row.get(4)?,
            enabled: row.get::<_, i64>(5)? != 0,
            notes: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    let mut overrides = HashMap::new();
    for row in rows.flatten() {
        overrides.insert(row.slug.clone(), row);
    }
    Ok(overrides)
}

fn replace_overrides(conn: &mut Connection, overrides: &[ProductOverride]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM product_overrides", [])?;
    {
        let mut stmt = tx.prepare(
            r#"
            INSERT INTO product_overrides (
              slug, frontend_url, api_url, service_token, api_key, enabled, notes, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )?;
        for item in overrides {
            stmt.execute(params![
                &item.slug,
                &item.frontend_url,
                &item.api_url,
                &item.service_token,
                &item.legacy_api_key,
                if item.enabled { 1 } else { 0 },
                &item.notes,
                &item.updated_at,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn load_action_events(conn: &Connection, limit: u32) -> rusqlite::Result<Vec<ProductActionEvent>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, product_slug, action_id, action_label, method, path, target_url,
               status, remote_status, request_json, response_json, error, created_at
        FROM product_action_events
        ORDER BY created_at DESC
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map([limit.clamp(1, 100)], |row| {
        let request_json = row.get::<_, String>(9)?;
        let response_json = row.get::<_, String>(10)?;
        let remote_status = row.get::<_, Option<i64>>(8)?;
        Ok(ProductActionEvent {
            id: row.get(0)?,
            product_slug: row.get(1)?,
            action_id: row.get(2)?,
            action_label: row.get(3)?,
            method: row.get(4)?,
            path: row.get(5)?,
            target_url: row.get(6)?,
            status: row.get(7)?,
            remote_status: remote_status.map(|value| value as u16),
            request_json: serde_json::from_str(&request_json).unwrap_or(serde_json::Value::Null),
            response_json: serde_json::from_str(&response_json).unwrap_or(serde_json::Value::Null),
            error: row.get(11)?,
            created_at: row.get(12)?,
        })
    })?;

    Ok(rows.flatten().collect())
}

fn load_action_event(conn: &Connection, id: &str) -> rusqlite::Result<Option<ProductActionEvent>> {
    conn.query_row(
        r#"
        SELECT id, product_slug, action_id, action_label, method, path, target_url,
               status, remote_status, request_json, response_json, error, created_at
        FROM product_action_events
        WHERE id = ?1
        "#,
        [id],
        |row| {
            let request_json = row.get::<_, String>(9)?;
            let response_json = row.get::<_, String>(10)?;
            let remote_status = row.get::<_, Option<i64>>(8)?;
            Ok(ProductActionEvent {
                id: row.get(0)?,
                product_slug: row.get(1)?,
                action_id: row.get(2)?,
                action_label: row.get(3)?,
                method: row.get(4)?,
                path: row.get(5)?,
                target_url: row.get(6)?,
                status: row.get(7)?,
                remote_status: remote_status.map(|value| value as u16),
                request_json: serde_json::from_str(&request_json)
                    .unwrap_or(serde_json::Value::Null),
                response_json: serde_json::from_str(&response_json)
                    .unwrap_or(serde_json::Value::Null),
                error: row.get(11)?,
                created_at: row.get(12)?,
            })
        },
    )
    .optional()
}

#[cfg(test)]
mod tests {
    use super::{
        init_schema, load_action_event, load_action_events, load_product_overrides,
        load_suite_settings, replace_overrides, write_suite_settings,
    };
    use crate::models::{now_rfc3339, ProductActionEvent, ProductOverride, SuiteSettings};
    use rusqlite::Connection;
    use serde_json::json;

    #[test]
    fn suite_settings_round_trip_in_memory() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        init_schema(&conn).expect("schema should initialize");

        let mut settings = SuiteSettings::default();
        settings.operator_label = "Jeremy".into();
        settings.preferred_launch_product = "repo-reaper".into();
        settings.updated_at = now_rfc3339();
        write_suite_settings(&conn, &settings).expect("settings should save");

        let loaded = load_suite_settings(&conn).expect("settings should load");
        assert_eq!(loaded.operator_label, "Jeremy");
        assert_eq!(loaded.preferred_launch_product, "repo-reaper");
    }

    #[test]
    fn replacing_overrides_rewrites_rows() {
        let mut conn = Connection::open_in_memory().expect("in-memory db should open");
        init_schema(&conn).expect("schema should initialize");

        let first = vec![ProductOverride {
            slug: "signal-hive".into(),
            frontend_url: "https://signal.example.com".into(),
            api_url: "https://signal-api.example.com".into(),
            service_token: "svc_signal".into(),
            legacy_api_key: "sh_secret".into(),
            enabled: true,
            notes: "primary".into(),
            updated_at: now_rfc3339(),
        }];
        replace_overrides(&mut conn, &first).expect("first save should work");

        let second = vec![ProductOverride {
            slug: "repo-reaper".into(),
            frontend_url: "https://reaper.example.com".into(),
            api_url: "https://reaper-api.example.com".into(),
            service_token: "svc_reaper".into(),
            legacy_api_key: "rr_secret".into(),
            enabled: false,
            notes: "manual only".into(),
            updated_at: now_rfc3339(),
        }];
        replace_overrides(&mut conn, &second).expect("second save should work");

        let rows = load_product_overrides(&conn).expect("rows should load");
        assert_eq!(rows.len(), 1);
        assert!(rows.contains_key("repo-reaper"));
        assert!(!rows.contains_key("signal-hive"));
        assert_eq!(rows["repo-reaper"].service_token, "svc_reaper");
        assert_eq!(rows["repo-reaper"].legacy_api_key, "rr_secret");
    }

    #[test]
    fn action_events_round_trip_in_memory() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        init_schema(&conn).expect("schema should initialize");

        let event = ProductActionEvent {
            id: "evt_1".into(),
            product_slug: "signal-hive".into(),
            action_id: "scan".into(),
            action_label: "Run signal scan".into(),
            method: "POST".into(),
            path: "/scan".into(),
            target_url: "http://localhost:8010/scan".into(),
            status: "dispatched".into(),
            remote_status: Some(200),
            request_json: json!({"languages": ["rust"]}),
            response_json: json!({"ok": true}),
            error: String::new(),
            created_at: now_rfc3339(),
        };

        conn.execute(
            r#"
            INSERT INTO product_action_events (
              id, product_slug, action_id, action_label, method, path, target_url,
              status, remote_status, request_json, response_json, error, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            rusqlite::params![
                &event.id,
                &event.product_slug,
                &event.action_id,
                &event.action_label,
                &event.method,
                &event.path,
                &event.target_url,
                &event.status,
                event.remote_status.map(i64::from),
                event.request_json.to_string(),
                event.response_json.to_string(),
                &event.error,
                &event.created_at,
            ],
        )
        .expect("event should insert");

        let events = load_action_events(&conn, 10).expect("events should load");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].product_slug, "signal-hive");
        assert_eq!(events[0].response_json["ok"], true);

        let loaded = load_action_event(&conn, "evt_1")
            .expect("event lookup should work")
            .expect("event should exist");
        assert_eq!(loaded.action_id, "scan");
    }
}

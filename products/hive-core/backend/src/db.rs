use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use once_cell::sync::OnceCell;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{ProductOverride, SuiteSettings};

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
          enabled INTEGER NOT NULL,
          notes TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );
        "#,
    )?;
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
        SELECT slug, frontend_url, api_url, enabled, notes, updated_at
        FROM product_overrides
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ProductOverride {
            slug: row.get(0)?,
            frontend_url: row.get(1)?,
            api_url: row.get(2)?,
            enabled: row.get::<_, i64>(3)? != 0,
            notes: row.get(4)?,
            updated_at: row.get(5)?,
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
            INSERT INTO product_overrides (slug, frontend_url, api_url, enabled, notes, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )?;
        for item in overrides {
            stmt.execute(params![
                &item.slug,
                &item.frontend_url,
                &item.api_url,
                if item.enabled { 1 } else { 0 },
                &item.notes,
                &item.updated_at,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        init_schema, load_product_overrides, load_suite_settings, replace_overrides,
        write_suite_settings,
    };
    use crate::models::{now_rfc3339, ProductOverride, SuiteSettings};
    use rusqlite::Connection;

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
            enabled: true,
            notes: "primary".into(),
            updated_at: now_rfc3339(),
        }];
        replace_overrides(&mut conn, &first).expect("first save should work");

        let second = vec![ProductOverride {
            slug: "repo-reaper".into(),
            frontend_url: "https://reaper.example.com".into(),
            api_url: "https://reaper-api.example.com".into(),
            enabled: false,
            notes: "manual only".into(),
            updated_at: now_rfc3339(),
        }];
        replace_overrides(&mut conn, &second).expect("second save should work");

        let rows = load_product_overrides(&conn).expect("rows should load");
        assert_eq!(rows.len(), 1);
        assert!(rows.contains_key("repo-reaper"));
        assert!(!rows.contains_key("signal-hive"));
    }
}

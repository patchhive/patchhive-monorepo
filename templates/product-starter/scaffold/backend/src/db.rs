use std::sync::{Mutex, MutexGuard};

use once_cell::sync::OnceCell;
use rusqlite::Connection;

static DB_CONN: OnceCell<Mutex<Connection>> = OnceCell::new();

pub fn db_path() -> String {
    std::env::var("__ENV_PREFIX___DB_PATH").unwrap_or_else(|_| "__DB_FILE__".into())
}

fn open_connection() -> rusqlite::Result<Connection> {
    Connection::open(db_path())
}

fn connect() -> rusqlite::Result<MutexGuard<'static, Connection>> {
    let mutex = DB_CONN.get_or_try_init(|| open_connection().map(Mutex::new))?;
    mutex
        .lock()
        .map_err(|_| rusqlite::Error::InvalidQuery)
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
        CREATE TABLE IF NOT EXISTS product_meta (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );
        "#,
    )?;
    conn.execute(
        r#"
        INSERT INTO product_meta (key, value)
        VALUES ('starter', 'true')
        ON CONFLICT(key) DO NOTHING
        "#,
        [],
    )?;
    Ok(())
}

pub fn meta_count() -> usize {
    connect()
        .ok()
        .and_then(|conn| {
            conn.query_row("SELECT COUNT(*) FROM product_meta", [], |row| row.get::<_, i64>(0))
                .ok()
        })
        .unwrap_or(0) as usize
}

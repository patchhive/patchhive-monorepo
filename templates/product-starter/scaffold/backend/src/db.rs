use once_cell::sync::Lazy;
use patchhive_product_core::sqlite::{PooledSqliteConnection, SqlitePool};

static DB_POOL: Lazy<SqlitePool> = Lazy::new(|| {
    SqlitePool::new(db_path(), "__PRODUCT_NAME__").with_pool_size_env("__ENV_PREFIX___DB_POOL_SIZE")
});

pub fn db_path() -> String {
    std::env::var("__ENV_PREFIX___DB_PATH").unwrap_or_else(|_| "__DB_FILE__".into())
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
            conn.query_row("SELECT COUNT(*) FROM product_meta", [], |row| {
                row.get::<_, i64>(0)
            })
            .ok()
        })
        .unwrap_or(0) as usize
}

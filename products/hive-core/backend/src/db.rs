use anyhow::{Context, Result};
use std::collections::HashMap;

use once_cell::sync::Lazy;
use patchhive_product_core::secrets::TokenProtector;
use patchhive_product_core::sqlite::{PooledSqliteConnection, SqlitePool};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::models::{
    FirstStackSmokeRun, PrBudgetReservation, ProductActionEvent, ProductOverride, RepositoryPolicy,
    SuiteSettings,
};

static DB_POOL: Lazy<SqlitePool> = Lazy::new(|| {
    SqlitePool::new(db_path(), "HiveCore").with_pool_size_env("HIVE_CORE_DB_POOL_SIZE")
});

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServiceTokenStorageStats {
    pub total: usize,
    pub encrypted: usize,
    pub plaintext: usize,
}

#[derive(Debug, Clone)]
pub struct PrReservationAttempt {
    pub granted: bool,
    pub reason: String,
    pub limiting_layer: String,
    pub product_limit: u32,
    pub product_used: u32,
    pub suite_limit: u32,
    pub suite_used: u32,
    pub reservation: Option<PrBudgetReservation>,
}

pub fn db_path() -> String {
    std::env::var("HIVE_CORE_DB_PATH").unwrap_or_else(|_| "hive-core.db".into())
}

fn connect() -> rusqlite::Result<PooledSqliteConnection<'static>> {
    DB_POOL.get()
}

pub fn health_check() -> bool {
    connect()
        .and_then(|conn| conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0)))
        .is_ok()
}

pub fn init_db() -> Result<()> {
    let conn = connect()?;
    init_schema(&conn)?;
    seed_defaults(&conn)?;
    migrate_service_token_storage(&conn)?;
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
    match load_product_overrides(&conn, &TokenProtector::from_env("HIVECORE_ENCRYPTION_KEY")) {
        Ok(overrides) => overrides,
        Err(err) => {
            tracing::warn!("failed to load HiveCore product overrides: {err}");
            HashMap::new()
        }
    }
}

pub fn replace_product_overrides(overrides: &[ProductOverride]) -> Result<()> {
    let mut conn = connect()?;
    replace_overrides(
        &mut conn,
        overrides,
        &TokenProtector::from_env("HIVECORE_ENCRYPTION_KEY"),
    )
}

pub fn service_token_storage_stats() -> ServiceTokenStorageStats {
    let Ok(conn) = connect() else {
        return ServiceTokenStorageStats::default();
    };
    load_service_token_storage_stats(&conn).unwrap_or_default()
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

pub fn repository_policies() -> Vec<RepositoryPolicy> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };
    load_repository_policies(&conn).unwrap_or_default()
}

pub fn repository_policy_result(repository: &str) -> rusqlite::Result<Option<RepositoryPolicy>> {
    let conn = connect()?;
    load_repository_policy(&conn, repository)
}

pub fn replace_repository_policies(policies: &[RepositoryPolicy]) -> rusqlite::Result<()> {
    let mut conn = connect()?;
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM repository_policies", [])?;
    {
        let mut stmt = tx.prepare(
            r#"
            INSERT INTO repository_policies (
              repository, trusted, operator_excluded, notes, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )?;
        for policy in policies {
            stmt.execute(params![
                policy.repository,
                if policy.trusted { 1 } else { 0 },
                if policy.operator_excluded { 1 } else { 0 },
                policy.notes,
                policy.updated_at,
            ])?;
        }
    }
    tx.commit()
}

pub fn suite_pr_limit() -> u32 {
    let Ok(conn) = connect() else {
        return 10;
    };
    load_suite_pr_limit(&conn).unwrap_or(10)
}

pub fn product_pr_limits() -> HashMap<String, u32> {
    let Ok(conn) = connect() else {
        return HashMap::new();
    };
    load_product_pr_limits(&conn).unwrap_or_default()
}

pub fn save_pr_budget_settings(
    suite_limit: u32,
    products: &[(String, u32)],
    updated_at: &str,
) -> rusqlite::Result<()> {
    let mut conn = connect()?;
    let tx = conn.transaction()?;
    tx.execute(
        r#"
        INSERT INTO pr_budget_settings (id, suite_limit, updated_at)
        VALUES (1, ?1, ?2)
        ON CONFLICT(id) DO UPDATE SET
          suite_limit = excluded.suite_limit,
          updated_at = excluded.updated_at
        "#,
        params![suite_limit, updated_at],
    )?;
    tx.execute("DELETE FROM product_pr_budgets", [])?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO product_pr_budgets (product_slug, pr_limit, updated_at) VALUES (?1, ?2, ?3)",
        )?;
        for (product, limit) in products {
            stmt.execute(params![product, limit, updated_at])?;
        }
    }
    tx.commit()
}

pub fn pr_budget_reservations(limit: u32) -> Vec<PrBudgetReservation> {
    let Ok(mut conn) = connect() else {
        return Vec::new();
    };
    if expire_pr_reservations(&mut conn).is_err() {
        return Vec::new();
    }
    load_pr_reservations(&conn, limit).unwrap_or_default()
}

pub fn active_pr_usage() -> rusqlite::Result<(u32, HashMap<String, u32>)> {
    let mut conn = connect()?;
    expire_pr_reservations(&mut conn)?;
    let suite_used = active_pr_count(&conn, None)?;
    let mut stmt = conn.prepare(
        r#"
        SELECT product_slug, COUNT(*)
        FROM pr_budget_reservations
        WHERE status IN ('reserved', 'committed')
        GROUP BY product_slug
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
    })?;
    Ok((suite_used, rows.collect::<rusqlite::Result<_>>()?))
}

pub fn reserve_pr_slot(
    reservation: &PrBudgetReservation,
) -> rusqlite::Result<PrReservationAttempt> {
    let mut conn = connect()?;
    reserve_pr_slot_with_connection(&mut conn, reservation)
}

fn reserve_pr_slot_with_connection(
    conn: &mut Connection,
    reservation: &PrBudgetReservation,
) -> rusqlite::Result<PrReservationAttempt> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    expire_pr_reservations_in_transaction(&tx)?;

    let suite_limit = tx.query_row(
        "SELECT suite_limit FROM pr_budget_settings WHERE id = 1",
        [],
        |row| row.get::<_, i64>(0),
    )? as u32;
    let product_limit = tx
        .query_row(
            "SELECT pr_limit FROM product_pr_budgets WHERE product_slug = ?1",
            [&reservation.product],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .map(|value| value as u32)
        .unwrap_or_else(|| default_product_pr_limit(&reservation.product));
    let suite_used = active_pr_count(&tx, None)?;
    let product_used = active_pr_count(&tx, Some(&reservation.product))?;

    let denial = if product_limit == 0 {
        Some((
            "product",
            format!(
                "{} has no PR budget. Configure a positive product maximum in HiveCore.",
                reservation.product
            ),
        ))
    } else if product_used >= product_limit {
        Some((
            "product",
            format!(
                "{} has used all {product_limit} of its PR slots.",
                reservation.product
            ),
        ))
    } else if suite_limit == 0 {
        Some((
            "suite",
            "The PatchHive suite PR ceiling is zero.".to_string(),
        ))
    } else if suite_used >= suite_limit {
        Some((
            "suite",
            format!("The PatchHive suite has used all {suite_limit} PR slots."),
        ))
    } else {
        None
    };

    if let Some((limiting_layer, reason)) = denial {
        tx.execute(
            r#"
            INSERT INTO pr_budget_events (
              reservation_id, product_slug, repository, event_type, reason, created_at
            ) VALUES ('', ?1, ?2, 'denied', ?3, ?4)
            "#,
            params![
                reservation.product,
                reservation.repository,
                &reason,
                reservation.created_at
            ],
        )?;
        tx.commit()?;
        return Ok(PrReservationAttempt {
            granted: false,
            reason,
            limiting_layer: limiting_layer.into(),
            product_limit,
            product_used,
            suite_limit,
            suite_used,
            reservation: None,
        });
    }

    tx.execute(
        r#"
        INSERT INTO pr_budget_reservations (
          id, product_slug, repository, run_id, action, status, pr_url, reason,
          created_at, expires_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            reservation.id,
            reservation.product,
            reservation.repository,
            reservation.run_id,
            reservation.action,
            reservation.status,
            reservation.pr_url,
            reservation.reason,
            reservation.created_at,
            reservation.expires_at,
            reservation.updated_at,
        ],
    )?;
    record_pr_budget_event(
        &tx,
        reservation,
        "granted",
        "HiveCore reserved one PR slot.",
        &reservation.created_at,
    )?;
    tx.commit()?;

    Ok(PrReservationAttempt {
        granted: true,
        reason: "HiveCore reserved one PR slot.".into(),
        limiting_layer: String::new(),
        product_limit,
        product_used,
        suite_limit,
        suite_used,
        reservation: Some(reservation.clone()),
    })
}

pub fn commit_pr_reservation(
    id: &str,
    pr_url: &str,
    updated_at: &str,
) -> rusqlite::Result<Option<PrBudgetReservation>> {
    let mut conn = connect()?;
    expire_pr_reservations(&mut conn)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let changed = tx.execute(
        r#"
        UPDATE pr_budget_reservations
        SET status = 'committed', pr_url = ?2, updated_at = ?3
        WHERE id = ?1 AND status = 'reserved'
        "#,
        params![id, pr_url, updated_at],
    )?;
    let reservation = load_pr_reservation(&tx, id)?;
    if changed > 0 {
        if let Some(reservation) = &reservation {
            record_pr_budget_event(&tx, reservation, "committed", pr_url, updated_at)?;
        }
    }
    tx.commit()?;
    Ok(reservation)
}

pub fn release_pr_reservation(
    id: &str,
    reason: &str,
    updated_at: &str,
) -> rusqlite::Result<Option<PrBudgetReservation>> {
    let mut conn = connect()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let changed = tx.execute(
        r#"
        UPDATE pr_budget_reservations
        SET status = 'released', reason = ?2, updated_at = ?3
        WHERE id = ?1 AND status IN ('reserved', 'committed')
        "#,
        params![id, reason, updated_at],
    )?;
    let reservation = load_pr_reservation(&tx, id)?;
    if changed > 0 {
        if let Some(reservation) = &reservation {
            record_pr_budget_event(&tx, reservation, "released", reason, updated_at)?;
        }
    }
    tx.commit()?;
    Ok(reservation)
}

pub fn release_pr_reservations_for_run(
    product: &str,
    run_id: &str,
    reason: &str,
    updated_at: &str,
) -> rusqlite::Result<Vec<PrBudgetReservation>> {
    let mut conn = connect()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let ids = {
        let mut stmt = tx.prepare(
            r#"
            SELECT id
            FROM pr_budget_reservations
            WHERE product_slug = ?1 AND run_id = ?2
              AND status IN ('reserved', 'committed')
            "#,
        )?;
        let rows = stmt
            .query_map(params![product, run_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    tx.execute(
        r#"
        UPDATE pr_budget_reservations
        SET status = 'released', reason = ?3, updated_at = ?4
        WHERE product_slug = ?1 AND run_id = ?2
          AND status IN ('reserved', 'committed')
        "#,
        params![product, run_id, reason, updated_at],
    )?;
    let mut released = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(reservation) = load_pr_reservation(&tx, &id)? {
            record_pr_budget_event(&tx, &reservation, "released", reason, updated_at)?;
            released.push(reservation);
        }
    }
    tx.commit()?;
    Ok(released)
}

pub fn default_product_pr_limit(product: &str) -> u32 {
    if product == "repo-reaper" {
        5
    } else {
        0
    }
}

pub fn action_event(id: &str) -> Option<ProductActionEvent> {
    let Ok(conn) = connect() else {
        return None;
    };
    load_action_event(&conn, id).ok().flatten()
}

pub fn record_first_stack_smoke_run(run: &FirstStackSmokeRun) -> rusqlite::Result<()> {
    let conn = connect()?;
    conn.execute(
        r#"
        INSERT INTO first_stack_smoke_runs (
          id, tier, status, started_at, finished_at, summary, steps_json
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            &run.id,
            &run.tier,
            &run.status,
            &run.started_at,
            &run.finished_at,
            &run.summary,
            serde_json::to_string(&run.steps).unwrap_or_else(|_| "[]".into()),
        ],
    )?;
    Ok(())
}

pub fn latest_first_stack_smoke_run() -> Option<FirstStackSmokeRun> {
    let Ok(conn) = connect() else {
        return None;
    };
    load_latest_first_stack_smoke_run(&conn).ok().flatten()
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

        CREATE TABLE IF NOT EXISTS first_stack_smoke_runs (
          id TEXT PRIMARY KEY,
          tier TEXT NOT NULL DEFAULT 'first-stack',
          status TEXT NOT NULL,
          started_at TEXT NOT NULL,
          finished_at TEXT NOT NULL,
          summary TEXT NOT NULL,
          steps_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS repository_policies (
          repository TEXT PRIMARY KEY,
          trusted INTEGER NOT NULL DEFAULT 0,
          operator_excluded INTEGER NOT NULL DEFAULT 0,
          notes TEXT NOT NULL DEFAULT '',
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS pr_budget_settings (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          suite_limit INTEGER NOT NULL DEFAULT 10,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS product_pr_budgets (
          product_slug TEXT PRIMARY KEY,
          pr_limit INTEGER NOT NULL DEFAULT 0,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS pr_budget_reservations (
          id TEXT PRIMARY KEY,
          product_slug TEXT NOT NULL,
          repository TEXT NOT NULL,
          run_id TEXT NOT NULL,
          action TEXT NOT NULL,
          status TEXT NOT NULL,
          pr_url TEXT NOT NULL DEFAULT '',
          reason TEXT NOT NULL DEFAULT '',
          created_at TEXT NOT NULL,
          expires_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_pr_budget_reservations_status
          ON pr_budget_reservations (status, product_slug, updated_at DESC);

        CREATE TABLE IF NOT EXISTS pr_budget_events (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          reservation_id TEXT NOT NULL DEFAULT '',
          product_slug TEXT NOT NULL,
          repository TEXT NOT NULL,
          event_type TEXT NOT NULL,
          reason TEXT NOT NULL DEFAULT '',
          created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_pr_budget_events_created
          ON pr_budget_events (created_at DESC, product_slug);
        "#,
    )?;
    conn.execute(
        "INSERT INTO pr_budget_settings (id, suite_limit, updated_at) VALUES (1, 10, datetime('now')) ON CONFLICT(id) DO NOTHING",
        [],
    )?;
    migrate_schema(conn)?;
    Ok(())
}

fn migrate_schema(conn: &Connection) -> rusqlite::Result<()> {
    add_missing_column(
        conn,
        "first_stack_smoke_runs",
        "tier",
        "TEXT NOT NULL DEFAULT 'first-stack'",
    )?;

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

fn add_missing_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    let columns = conn
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |row| row.get::<_, String>(1))?
        .flatten()
        .collect::<Vec<_>>();

    if !columns.iter().any(|existing| existing == column) {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }

    Ok(())
}

fn migrate_service_token_storage(conn: &Connection) -> Result<()> {
    let protector = TokenProtector::from_env("HIVECORE_ENCRYPTION_KEY");
    if !protector.configured() {
        return Ok(());
    }

    let mut stmt = conn.prepare(
        r#"
        SELECT slug, service_token
        FROM product_overrides
        WHERE TRIM(service_token) != ''
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (slug, raw_service_token) = row?;
        if TokenProtector::is_encrypted_value(&raw_service_token) {
            continue;
        }

        let encrypted = protector
            .protect_for_storage(&raw_service_token)
            .with_context(|| format!("failed to encrypt HiveCore service token for {slug}"))?;
        conn.execute(
            "UPDATE product_overrides SET service_token = ?1 WHERE slug = ?2",
            params![encrypted, slug],
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

fn load_product_overrides(
    conn: &Connection,
    protector: &TokenProtector,
) -> Result<HashMap<String, ProductOverride>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT slug, frontend_url, api_url, service_token, api_key, enabled, notes, updated_at
        FROM product_overrides
        "#,
    )?;

    let mut overrides = HashMap::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let slug = row.get::<_, String>(0)?;
        let raw_service_token = row.get::<_, String>(3)?;
        let service_token = protector
            .reveal_from_storage(&raw_service_token)
            .with_context(|| format!("failed to reveal HiveCore service token for {slug}"))?;
        let override_item = ProductOverride {
            slug: slug.clone(),
            frontend_url: row.get(1)?,
            api_url: row.get(2)?,
            service_token,
            legacy_api_key: row.get(4)?,
            enabled: row.get::<_, i64>(5)? != 0,
            notes: row.get(6)?,
            updated_at: row.get(7)?,
        };
        overrides.insert(slug, override_item);
    }
    Ok(overrides)
}

fn replace_overrides(
    conn: &mut Connection,
    overrides: &[ProductOverride],
    protector: &TokenProtector,
) -> Result<()> {
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
            let protected_service_token = protector
                .protect_for_storage(&item.service_token)
                .with_context(|| {
                    format!("failed to protect HiveCore service token for {}", item.slug)
                })?;
            stmt.execute(params![
                &item.slug,
                &item.frontend_url,
                &item.api_url,
                &protected_service_token,
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

fn load_service_token_storage_stats(
    conn: &Connection,
) -> rusqlite::Result<ServiceTokenStorageStats> {
    let mut stmt = conn.prepare("SELECT service_token FROM product_overrides")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut stats = ServiceTokenStorageStats::default();
    for raw in rows.flatten() {
        if raw.trim().is_empty() {
            continue;
        }
        stats.total += 1;
        if TokenProtector::is_encrypted_value(&raw) {
            stats.encrypted += 1;
        } else {
            stats.plaintext += 1;
        }
    }
    Ok(stats)
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

fn load_latest_first_stack_smoke_run(
    conn: &Connection,
) -> rusqlite::Result<Option<FirstStackSmokeRun>> {
    conn.query_row(
        r#"
        SELECT id, tier, status, started_at, finished_at, summary, steps_json
        FROM first_stack_smoke_runs
        ORDER BY finished_at DESC
        LIMIT 1
        "#,
        [],
        |row| {
            let steps_json = row.get::<_, String>(6)?;
            Ok(FirstStackSmokeRun {
                id: row.get(0)?,
                tier: row.get(1)?,
                status: row.get(2)?,
                started_at: row.get(3)?,
                finished_at: row.get(4)?,
                summary: row.get(5)?,
                steps: serde_json::from_str(&steps_json).unwrap_or_default(),
            })
        },
    )
    .optional()
}

fn load_repository_policies(conn: &Connection) -> rusqlite::Result<Vec<RepositoryPolicy>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT repository, trusted, operator_excluded, notes, updated_at
        FROM repository_policies
        ORDER BY repository
        "#,
    )?;
    let rows = stmt.query_map([], decode_repository_policy)?;
    rows.collect()
}

fn load_repository_policy(
    conn: &Connection,
    repository: &str,
) -> rusqlite::Result<Option<RepositoryPolicy>> {
    conn.query_row(
        r#"
        SELECT repository, trusted, operator_excluded, notes, updated_at
        FROM repository_policies
        WHERE repository = ?1
        "#,
        [repository],
        decode_repository_policy,
    )
    .optional()
}

fn decode_repository_policy(row: &rusqlite::Row<'_>) -> rusqlite::Result<RepositoryPolicy> {
    Ok(RepositoryPolicy {
        repository: row.get(0)?,
        trusted: row.get::<_, i64>(1)? != 0,
        operator_excluded: row.get::<_, i64>(2)? != 0,
        notes: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn load_suite_pr_limit(conn: &Connection) -> rusqlite::Result<u32> {
    conn.query_row(
        "SELECT suite_limit FROM pr_budget_settings WHERE id = 1",
        [],
        |row| row.get::<_, i64>(0).map(|value| value as u32),
    )
}

fn load_product_pr_limits(conn: &Connection) -> rusqlite::Result<HashMap<String, u32>> {
    let mut stmt = conn
        .prepare("SELECT product_slug, pr_limit FROM product_pr_budgets ORDER BY product_slug")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
    })?;
    rows.collect()
}

fn active_pr_count(conn: &Connection, product: Option<&str>) -> rusqlite::Result<u32> {
    let count = if let Some(product) = product {
        conn.query_row(
            "SELECT COUNT(*) FROM pr_budget_reservations WHERE status IN ('reserved', 'committed') AND product_slug = ?1",
            [product],
            |row| row.get::<_, i64>(0),
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(*) FROM pr_budget_reservations WHERE status IN ('reserved', 'committed')",
            [],
            |row| row.get::<_, i64>(0),
        )?
    };
    Ok(count as u32)
}

fn record_pr_budget_event(
    conn: &Connection,
    reservation: &PrBudgetReservation,
    event_type: &str,
    reason: &str,
    created_at: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        r#"
        INSERT INTO pr_budget_events (
          reservation_id, product_slug, repository, event_type, reason, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            reservation.id,
            reservation.product,
            reservation.repository,
            event_type,
            reason,
            created_at,
        ],
    )?;
    Ok(())
}

fn expire_pr_reservations(conn: &mut Connection) -> rusqlite::Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    expire_pr_reservations_in_transaction(&tx)?;
    tx.commit()
}

fn expire_pr_reservations_in_transaction(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute(
        r#"
        INSERT INTO pr_budget_events (
          reservation_id, product_slug, repository, event_type, reason, created_at
        )
        SELECT id, product_slug, repository, 'expired',
               'Reservation lease expired before PR creation.', datetime('now')
        FROM pr_budget_reservations
        WHERE status = 'reserved' AND datetime(expires_at) <= datetime('now')
        "#,
        [],
    )?;
    tx.execute(
        r#"
        UPDATE pr_budget_reservations
        SET status = 'expired', reason = 'Reservation lease expired before PR creation.',
            updated_at = datetime('now')
        WHERE status = 'reserved' AND datetime(expires_at) <= datetime('now')
        "#,
        [],
    )?;
    Ok(())
}

fn load_pr_reservations(
    conn: &Connection,
    limit: u32,
) -> rusqlite::Result<Vec<PrBudgetReservation>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, product_slug, repository, run_id, action, status, pr_url,
               reason, created_at, expires_at, updated_at
        FROM pr_budget_reservations
        ORDER BY updated_at DESC
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map([limit.clamp(1, 200)], decode_pr_reservation)?;
    rows.collect()
}

fn load_pr_reservation(
    conn: &Connection,
    id: &str,
) -> rusqlite::Result<Option<PrBudgetReservation>> {
    conn.query_row(
        r#"
        SELECT id, product_slug, repository, run_id, action, status, pr_url,
               reason, created_at, expires_at, updated_at
        FROM pr_budget_reservations
        WHERE id = ?1
        "#,
        [id],
        decode_pr_reservation,
    )
    .optional()
}

fn decode_pr_reservation(row: &rusqlite::Row<'_>) -> rusqlite::Result<PrBudgetReservation> {
    Ok(PrBudgetReservation {
        id: row.get(0)?,
        product: row.get(1)?,
        repository: row.get(2)?,
        run_id: row.get(3)?,
        action: row.get(4)?,
        status: row.get(5)?,
        pr_url: row.get(6)?,
        reason: row.get(7)?,
        created_at: row.get(8)?,
        expires_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        init_schema, load_action_event, load_action_events, load_latest_first_stack_smoke_run,
        load_product_overrides, load_service_token_storage_stats, load_suite_settings,
        replace_overrides, reserve_pr_slot_with_connection, write_suite_settings,
        ServiceTokenStorageStats,
    };
    use crate::models::{
        now_rfc3339, FirstStackSmokeRun, FirstStackSmokeStep, PrBudgetReservation,
        ProductActionEvent, ProductOverride, SuiteSettings,
    };
    use patchhive_product_core::secrets::TokenProtector;
    use rusqlite::Connection;
    use serde_json::json;

    #[test]
    fn suite_settings_round_trip_in_memory() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        init_schema(&conn).expect("schema should initialize");

        let settings = SuiteSettings {
            operator_label: "Jeremy".into(),
            preferred_launch_product: "repo-reaper".into(),
            updated_at: now_rfc3339(),
            ..SuiteSettings::default()
        };
        write_suite_settings(&conn, &settings).expect("settings should save");

        let loaded = load_suite_settings(&conn).expect("settings should load");
        assert_eq!(loaded.operator_label, "Jeremy");
        assert_eq!(loaded.preferred_launch_product, "repo-reaper");
    }

    #[test]
    fn pr_reservations_enforce_product_and_suite_limits_atomically() {
        let mut conn = Connection::open_in_memory().expect("in-memory db should open");
        init_schema(&conn).expect("schema should initialize");
        conn.execute(
            "UPDATE pr_budget_settings SET suite_limit = 1 WHERE id = 1",
            [],
        )
        .expect("suite limit should update");
        conn.execute(
            "INSERT INTO product_pr_budgets (product_slug, pr_limit, updated_at) VALUES ('repo-reaper', 2, datetime('now'))",
            [],
        )
        .expect("product limit should insert");

        let first = sample_reservation("prr_1", "run_1");
        let granted = reserve_pr_slot_with_connection(&mut conn, &first)
            .expect("first reservation should evaluate");
        assert!(granted.granted);

        let second = sample_reservation("prr_2", "run_2");
        let denied = reserve_pr_slot_with_connection(&mut conn, &second)
            .expect("second reservation should evaluate");
        assert!(!denied.granted);
        assert_eq!(denied.limiting_layer, "suite");
        assert_eq!(denied.suite_used, 1);

        let grants: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pr_budget_events WHERE event_type = 'granted'",
                [],
                |row| row.get(0),
            )
            .expect("grant audit count should load");
        let denials: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pr_budget_events WHERE event_type = 'denied'",
                [],
                |row| row.get(0),
            )
            .expect("denial audit count should load");
        assert_eq!(grants, 1);
        assert_eq!(denials, 1);
    }

    fn sample_reservation(id: &str, run_id: &str) -> PrBudgetReservation {
        PrBudgetReservation {
            id: id.into(),
            product: "repo-reaper".into(),
            repository: "patchhive/example".into(),
            run_id: run_id.into(),
            action: "open_pull_request".into(),
            status: "reserved".into(),
            pr_url: String::new(),
            reason: String::new(),
            created_at: "2026-07-13T12:00:00Z".into(),
            expires_at: "2099-07-13T12:10:00Z".into(),
            updated_at: "2026-07-13T12:00:00Z".into(),
        }
    }

    #[test]
    fn replacing_overrides_rewrites_rows() {
        let mut conn = Connection::open_in_memory().expect("in-memory db should open");
        init_schema(&conn).expect("schema should initialize");
        let protector = TokenProtector::default();

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
        replace_overrides(&mut conn, &first, &protector).expect("first save should work");

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
        replace_overrides(&mut conn, &second, &protector).expect("second save should work");

        let rows = load_product_overrides(&conn, &protector).expect("rows should load");
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

    #[test]
    fn first_stack_smoke_runs_round_trip_in_memory() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        init_schema(&conn).expect("schema should initialize");

        let run = FirstStackSmokeRun {
            id: "smoke_1".into(),
            tier: "first-stack".into(),
            status: "ready".into(),
            started_at: now_rfc3339(),
            finished_at: now_rfc3339(),
            summary: "First stack is ready.".into(),
            steps: vec![FirstStackSmokeStep {
                slug: "signal-hive".into(),
                title: "SignalHive".into(),
                check: "health".into(),
                status: "pass".into(),
                message: "SignalHive responded.".into(),
                remote_status: Some(200),
                evidence: json!({"status": "ok"}),
            }],
        };

        conn.execute(
            r#"
            INSERT INTO first_stack_smoke_runs (
              id, tier, status, started_at, finished_at, summary, steps_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            rusqlite::params![
                &run.id,
                &run.tier,
                &run.status,
                &run.started_at,
                &run.finished_at,
                &run.summary,
                serde_json::to_string(&run.steps).expect("steps serialize"),
            ],
        )
        .expect("smoke run should insert");

        let loaded = load_latest_first_stack_smoke_run(&conn)
            .expect("smoke run should load")
            .expect("smoke run should exist");
        assert_eq!(loaded.status, "ready");
        assert_eq!(loaded.steps[0].slug, "signal-hive");
    }

    #[test]
    fn replacing_overrides_encrypts_service_tokens_when_key_is_configured() {
        let mut conn = Connection::open_in_memory().expect("in-memory db should open");
        init_schema(&conn).expect("schema should initialize");
        let protector = TokenProtector::from_secret(Some("test-secret"));

        let rows = vec![ProductOverride {
            slug: "signal-hive".into(),
            frontend_url: "https://signal.example.com".into(),
            api_url: "https://signal-api.example.com".into(),
            service_token: "svc_signal".into(),
            legacy_api_key: String::new(),
            enabled: true,
            notes: String::new(),
            updated_at: now_rfc3339(),
        }];
        replace_overrides(&mut conn, &rows, &protector).expect("save should work");

        let raw: String = conn
            .query_row(
                "SELECT service_token FROM product_overrides WHERE slug = 'signal-hive'",
                [],
                |row| row.get(0),
            )
            .expect("encrypted token should exist");
        assert!(TokenProtector::is_encrypted_value(&raw));

        let loaded = load_product_overrides(&conn, &protector).expect("rows should decrypt");
        assert_eq!(loaded["signal-hive"].service_token, "svc_signal");

        let stats = load_service_token_storage_stats(&conn).expect("stats should load");
        assert_eq!(
            stats,
            ServiceTokenStorageStats {
                total: 1,
                encrypted: 1,
                plaintext: 0,
            }
        );
    }
}

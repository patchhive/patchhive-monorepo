use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};

use once_cell::sync::Lazy;
use patchhive_product_core::repo_policy;
use patchhive_product_core::secrets::TokenProtector;
use patchhive_product_core::sqlite::{product_db_path, PooledSqliteConnection, SqlitePool};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::models::{
    FirstStackSmokeRun, PrBudgetReservation, ProbeSample, ProductActionEvent, ProductOverride,
    RepositoryPolicy, RunbookRun, SuiteSettings,
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

/// Suite-first, exactly as every other integrated product resolves it.
///
/// In suite mode the tables belong in PATCHHIVE_DB_PATH alongside the rest;
/// HIVE_CORE_DB_PATH remains the standalone compatibility override. Reading only the
/// product variable left a bare relative default, which wrote hive-core.db into
/// whatever directory the process started from — a second database beside the
/// suite's own.
pub fn db_path() -> String {
    product_db_path("HIVE_CORE_DB_PATH", "hive-core.db")
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
    migrate_repository_policy(&conn)?;
    Ok(())
}

/// Fold HiveCore's two legacy stores into the shared suite-wide policy table.
///
/// HiveCore held repository rules in two places that did not agree with each other:
/// a structured `repository_policies` table and two free-text fields on
/// `suite_settings`. Both fed one evaluator, which made them look like one store
/// while behaving as two. The shared table is now the only thing consulted; these
/// remain on disk, read once, as the migration source.
///
/// Conflicts resolve toward exclusion and are logged rather than swallowed. An
/// operator who denied a repository in one place and allowed it in another needs to
/// know which way it landed.
fn migrate_repository_policy(conn: &Connection) -> Result<()> {
    repo_policy::init_schema(conn)?;
    let report = repo_policy::migrate_legacy_tables(conn)?;
    if report.imported > 0 {
        tracing::info!(
            "repository policy: imported {} entries from {}",
            report.imported,
            report.sources.join(", ")
        );
    }
    for conflict in &report.conflicts {
        tracing::warn!(
            "repository policy conflict for {}: {} — resolved to {}",
            conflict.repository,
            conflict.claims.join(" / "),
            conflict.resolved_to.as_str()
        );
    }
    Ok(())
}

pub fn suite_settings() -> SuiteSettings {
    let Ok(conn) = connect() else {
        return SuiteSettings::default();
    };
    let stored = load_suite_settings(&conn).unwrap_or_default();
    // The allow/deny fields are rendered from the shared policy store, never from
    // the stored text. The stored copy is migration residue; reading it back would
    // resurrect the second store this change exists to remove.
    // Reuses the connection already held. Calling repo_list_text() here would take
    // a second one from the pool while this one is still checked out, which starves
    // the pool under concurrency instead of merely being wasteful.
    let (repo_allowlist, repo_denylist) = repo_list_text_from(&conn);
    SuiteSettings {
        repo_allowlist,
        repo_denylist,
        ..stored
    }
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

/// Every repository the shared store knows about, as one row per repository.
///
/// The shared table stores one row per (repository, kind); the operator thinks in
/// repositories. Collapsing happens here so nothing in the store is invisible to
/// the editor — an allowlist entry or a public opt-out that the UI could not see
/// would be silently dropped the next time an operator pressed save.
pub fn repository_policies() -> Vec<RepositoryPolicy> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };
    collapse_policies(&repo_policy::list(&conn).unwrap_or_default())
}

fn collapse_policies(entries: &[repo_policy::RepoPolicyEntry]) -> Vec<RepositoryPolicy> {
    let mut by_repo: BTreeMap<String, RepositoryPolicy> = BTreeMap::new();
    for entry in entries {
        let row = by_repo
            .entry(entry.repository.clone())
            .or_insert_with(|| RepositoryPolicy {
                repository: entry.repository.clone(),
                ..RepositoryPolicy::default()
            });
        match entry.kind {
            repo_policy::PolicyKind::OptOut => row.public_opt_out = true,
            repo_policy::PolicyKind::Denylist => row.operator_excluded = true,
            repo_policy::PolicyKind::Allowlist => row.allowlisted = true,
            repo_policy::PolicyKind::Trusted => row.trusted = true,
        }
        // Notes and provenance come from whichever entry carries them; the most
        // recently updated wins so an edit is what the operator sees next.
        if entry.updated_at > row.updated_at {
            row.updated_at = entry.updated_at.clone();
            row.source = entry.source.clone();
        }
        if !entry.notes.is_empty() && row.notes.is_empty() {
            row.notes = entry.notes.clone();
        }
    }
    by_repo.into_values().collect()
}

/// The allow/deny text fields, rendered from the shared store.
///
/// These two settings fields used to be their own store, parsed at evaluation time
/// and disagreeing with `repository_policies` whenever the two were edited apart.
/// They are kept as an operator convenience — pasting a list of repositories is
/// faster than a row-by-row editor — but they are now a *view*: read from the shared
/// table, written straight back into it. There is nothing left to drift.
pub fn repo_list_text() -> (String, String) {
    let Ok(conn) = connect() else {
        return (String::new(), String::new());
    };
    repo_list_text_from(&conn)
}

fn repo_list_text_from(conn: &Connection) -> (String, String) {
    let entries = repo_policy::list(conn).unwrap_or_default();
    let join = |kind| {
        entries
            .iter()
            .filter(|entry| entry.kind == kind)
            .map(|entry| entry.repository.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    (
        join(repo_policy::PolicyKind::Allowlist),
        join(repo_policy::PolicyKind::Denylist),
    )
}

/// Replace the allow/deny listings from the settings text fields.
///
/// Touches only the two kinds these fields represent. Trust is granted elsewhere and
/// verified opt-outs belong to repository owners; neither is the settings form's to
/// revoke, and both would otherwise vanish the moment someone edited a text box.
pub fn save_repo_list_text(allowlist: &str, denylist: &str, now: &str) -> Result<()> {
    let mut conn = connect()?;
    let tx = conn.transaction()?;
    tx.execute(
        &format!(
            "DELETE FROM {} WHERE kind IN (?1, ?2)",
            patchhive_product_core::repo_policy::TABLE
        ),
        params![
            repo_policy::PolicyKind::Allowlist.as_str(),
            repo_policy::PolicyKind::Denylist.as_str()
        ],
    )?;
    for (raw, kind) in [
        (allowlist, repo_policy::PolicyKind::Allowlist),
        (denylist, repo_policy::PolicyKind::Denylist),
    ] {
        for candidate in raw.split([',', ';', '\n', '\r']) {
            let Some(repository) =
                patchhive_product_core::scope_policy::normalize_repo_name(candidate)
            else {
                continue;
            };
            repo_policy::upsert(
                &tx,
                &repo_policy::RepoPolicyEntry {
                    repository,
                    kind,
                    source: "operator".into(),
                    notes: "Set from HiveCore suite settings.".into(),
                    verified: false,
                    updated_at: now.to_string(),
                },
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// One repository, one product, one operation — answered by the shared evaluator.
pub fn evaluate_repository_policy(
    repository: &str,
    product: &str,
    operation: &str,
) -> Result<repo_policy::Decision> {
    let conn = connect()?;
    repo_policy::evaluate(&conn, repository, product, operation)
}

pub fn repository_policy_result(repository: &str) -> Result<Option<RepositoryPolicy>> {
    let conn = connect()?;
    let entries = repo_policy::entries_for(&conn, repository)?;
    Ok(collapse_policies(&entries).into_iter().next())
}

/// Replace the operator-editable policy set.
///
/// Only the three kinds an operator owns are replaced. Verified public opt-outs are
/// deliberately untouched: the repository owner asked to be left alone through the
/// public flow, and no operator edit — including one that simply omits the row —
/// may revoke that. Omission is the dangerous case, which is why it is handled here
/// rather than trusted to the caller.
pub fn replace_repository_policies(policies: &[RepositoryPolicy]) -> Result<()> {
    let mut conn = connect()?;
    replace_repository_policies_with_connection(&mut conn, policies)
}

fn replace_repository_policies_with_connection(
    conn: &mut Connection,
    policies: &[RepositoryPolicy],
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        &format!(
            "DELETE FROM {} WHERE kind <> ?1",
            patchhive_product_core::repo_policy::TABLE
        ),
        params![repo_policy::PolicyKind::OptOut.as_str()],
    )?;
    for policy in policies {
        for (active, kind) in [
            (policy.operator_excluded, repo_policy::PolicyKind::Denylist),
            (policy.allowlisted, repo_policy::PolicyKind::Allowlist),
            (policy.trusted, repo_policy::PolicyKind::Trusted),
        ] {
            if !active {
                continue;
            }
            repo_policy::upsert(
                &tx,
                &repo_policy::RepoPolicyEntry {
                    repository: policy.repository.clone(),
                    kind,
                    source: "operator".into(),
                    notes: policy.notes.clone(),
                    verified: false,
                    updated_at: policy.updated_at.clone(),
                },
            )?;
        }
    }
    tx.commit()?;
    Ok(())
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
        SET status = 'committed', pr_url = ?2, updated_at = ?3,
            expires_at = datetime(?3, '+' || ?4 || ' days')
        WHERE id = ?1 AND status = 'reserved'
        "#,
        params![id, pr_url, updated_at, committed_pr_lease_days()],
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

pub fn pr_budget_reservation(id: &str) -> rusqlite::Result<Option<PrBudgetReservation>> {
    let conn = connect()?;
    load_pr_reservation(&conn, id)
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

/// How many probe samples to retain per product.
///
/// Bounded because the overview polls continuously: unbounded retention turns a
/// dashboard into a slowly growing disk-usage problem. 240 samples is enough for a
/// readable sparkline and an uptime figure with a stated denominator, which is the
/// most an operator should read into it anyway.
const PROBE_RETENTION: usize = 240;

/// Record one health-probe observation.
///
/// Deliberately infallible from the caller's perspective: a metrics write must never
/// turn a successful probe into a failed one. A lost sample is a gap in a sparkline;
/// a propagated error would be a product reported as down because HiveCore could not
/// write to its own database.
pub fn record_product_probe(slug: &str, latency_ms: u64, healthy: bool, observed_at: &str) {
    let Ok(conn) = connect() else {
        return;
    };
    let result = conn.execute(
        "INSERT INTO hive_core_product_probes (product_slug, observed_at, latency_ms, healthy)
         VALUES (?1, ?2, ?3, ?4)",
        params![slug, observed_at, latency_ms as i64, i64::from(healthy)],
    );
    if let Err(error) = result {
        tracing::debug!("could not record probe sample for {slug}: {error}");
        return;
    }
    // Prune inline rather than on a timer: the write that grows the table is the
    // natural place to bound it, and it keeps the retention rule in one spot.
    let _ = conn.execute(
        "DELETE FROM hive_core_product_probes
         WHERE product_slug = ?1
           AND id NOT IN (
             SELECT id FROM hive_core_product_probes
             WHERE product_slug = ?1 ORDER BY id DESC LIMIT ?2
           )",
        params![slug, PROBE_RETENTION as i64],
    );
}

/// Retained probe samples for one product, oldest first.
pub fn product_probes(slug: &str) -> rusqlite::Result<Vec<ProbeSample>> {
    let conn = connect()?;
    load_product_probes(&conn, slug)
}

fn load_product_probes(conn: &Connection, slug: &str) -> rusqlite::Result<Vec<ProbeSample>> {
    let mut stmt = conn.prepare(
        "SELECT observed_at, latency_ms, healthy FROM hive_core_product_probes
         WHERE product_slug = ?1 ORDER BY id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![slug, PROBE_RETENTION as i64], |row| {
        Ok(ProbeSample {
            observed_at: row.get(0)?,
            latency_ms: row.get::<_, i64>(1)? as u64,
            healthy: row.get::<_, i64>(2)? != 0,
        })
    })?;
    let mut samples = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    samples.reverse();
    Ok(samples)
}

/// Runbook history is server-side, not browser state.
///
/// It was React state: a record of what an operator did that did not survive a page
/// reload. A history that forgets is not a history, and this one is meant to answer
/// "who checked this product, and what did it say" after the fact.
pub fn record_runbook_run(run: &RunbookRun) -> rusqlite::Result<()> {
    let conn = connect()?;
    conn.execute(
        r#"
        INSERT OR REPLACE INTO hive_core_runbook_runs (
          id, product_slug, product_title, status, started_at, finished_at, summary, steps_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            run.id,
            run.product_slug,
            run.product_title,
            run.status,
            run.started_at,
            run.finished_at,
            run.summary,
            serde_json::to_string(&run.steps).unwrap_or_else(|_| "[]".into()),
        ],
    )?;
    Ok(())
}

pub fn runbook_runs(limit: u32) -> Vec<RunbookRun> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };
    load_runbook_runs(&conn, limit).unwrap_or_default()
}

fn load_runbook_runs(conn: &Connection, limit: u32) -> rusqlite::Result<Vec<RunbookRun>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, product_slug, product_title, status, started_at, finished_at, summary, steps_json
        FROM hive_core_runbook_runs
        ORDER BY started_at DESC
        LIMIT ?1
        "#,
    )?;
    let rows = stmt.query_map([limit], |row| {
        let steps_json: String = row.get(7)?;
        Ok(RunbookRun {
            id: row.get(0)?,
            product_slug: row.get(1)?,
            product_title: row.get(2)?,
            status: row.get(3)?,
            started_at: row.get(4)?,
            finished_at: row.get(5)?,
            summary: row.get(6)?,
            steps: serde_json::from_str(&steps_json).unwrap_or_default(),
        })
    })?;
    rows.collect()
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

        -- Namespaced: patchhive-backend already owns a suite-level `suite_runs`
        -- table with a different schema, and the suite database is shared. New
        -- product tables must be product-namespaced (CLAUDE.md § SQLite).
        CREATE TABLE IF NOT EXISTS hive_core_product_probes (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          product_slug TEXT NOT NULL,
          observed_at TEXT NOT NULL,
          latency_ms INTEGER NOT NULL,
          healthy INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_hive_core_product_probes_slug
        ON hive_core_product_probes(product_slug, id DESC);

        CREATE TABLE IF NOT EXISTS hive_core_runbook_runs (
          id TEXT PRIMARY KEY,
          product_slug TEXT NOT NULL,
          product_title TEXT NOT NULL,
          status TEXT NOT NULL,
          started_at TEXT NOT NULL,
          finished_at TEXT NOT NULL,
          summary TEXT NOT NULL,
          steps_json TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_hive_core_runbook_runs_started_at
        ON hive_core_runbook_runs(started_at DESC);

        CREATE TABLE IF NOT EXISTS hive_core_suite_runs (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          status TEXT NOT NULL,
          started_at TEXT NOT NULL,
          finished_at TEXT NOT NULL DEFAULT '',
          summary TEXT NOT NULL DEFAULT '',
          steps_json TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_hive_core_suite_runs_started
          ON hive_core_suite_runs (started_at DESC);

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
        let raw_legacy_api_key = row.get::<_, String>(4)?;
        let legacy_api_key = protector
            .reveal_from_storage(&raw_legacy_api_key)
            .with_context(|| format!("failed to reveal HiveCore legacy API key for {slug}"))?;
        let override_item = ProductOverride {
            slug: slug.clone(),
            frontend_url: row.get(1)?,
            api_url: row.get(2)?,
            service_token,
            legacy_api_key,
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
            let protected_legacy_api_key = protector
                .protect_for_storage(&item.legacy_api_key)
                .with_context(|| {
                    format!(
                        "failed to protect HiveCore legacy API key for {}",
                        item.slug
                    )
                })?;
            stmt.execute(params![
                &item.slug,
                &item.frontend_url,
                &item.api_url,
                &protected_service_token,
                &protected_legacy_api_key,
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

// The legacy `repository_policies` loaders are gone: that table is now migration
// input only, read once by migrate_repository_policy. Leaving readers behind would
// have recreated the exact problem the shared store exists to end — two tables that
// look like one because a single evaluator consults both.

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
    tx.execute(
        r#"
        INSERT INTO pr_budget_events (
          reservation_id, product_slug, repository, event_type, reason, created_at
        )
        SELECT id, product_slug, repository, 'committed_lease_expired',
               'Committed PR lease expired before GitHub state reconciliation.', datetime('now')
        FROM pr_budget_reservations
        WHERE status = 'committed' AND datetime(expires_at) <= datetime('now')
        "#,
        [],
    )?;
    tx.execute(
        r#"
        UPDATE pr_budget_reservations
        SET status = 'expired',
            reason = 'Committed PR lease expired before GitHub state reconciliation.',
            updated_at = datetime('now')
        WHERE status = 'committed' AND datetime(expires_at) <= datetime('now')
        "#,
        [],
    )?;
    Ok(())
}

fn committed_pr_lease_days() -> u32 {
    std::env::var("HIVECORE_COMMITTED_PR_LEASE_DAYS")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|days| *days > 0)
        .unwrap_or(30)
        .clamp(1, 365)
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

pub fn record_suite_run(run: &crate::models::SuiteRun) -> rusqlite::Result<()> {
    let conn = connect()?;
    conn.execute(
        r#"
        INSERT INTO hive_core_suite_runs (id, name, status, started_at, finished_at, summary, steps_json)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(id) DO UPDATE SET
          status = excluded.status,
          finished_at = excluded.finished_at,
          summary = excluded.summary,
          steps_json = excluded.steps_json
        "#,
        params![
            &run.id,
            &run.name,
            &run.status,
            &run.started_at,
            &run.finished_at,
            &run.summary,
            serde_json::to_string(&run.steps).unwrap_or_else(|_| "[]".into()),
        ],
    )?;
    Ok(())
}

pub fn suite_runs(limit: u32) -> Vec<crate::models::SuiteRun> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };
    let Ok(mut stmt) = conn.prepare(
        r#"
        SELECT id, name, status, started_at, finished_at, summary, steps_json
        FROM hive_core_suite_runs
        ORDER BY started_at DESC
        LIMIT ?1
        "#,
    ) else {
        return Vec::new();
    };
    let rows = stmt.query_map([limit.clamp(1, 200)], decode_suite_run);
    rows.map(|items| items.flatten().collect())
        .unwrap_or_default()
}

pub fn suite_run(id: &str) -> Option<crate::models::SuiteRun> {
    let conn = connect().ok()?;
    conn.query_row(
        r#"
        SELECT id, name, status, started_at, finished_at, summary, steps_json
        FROM hive_core_suite_runs
        WHERE id = ?1
        "#,
        [id],
        decode_suite_run,
    )
    .optional()
    .ok()
    .flatten()
}

fn decode_suite_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<crate::models::SuiteRun> {
    let steps_json: String = row.get(6)?;
    Ok(crate::models::SuiteRun {
        id: row.get(0)?,
        name: row.get(1)?,
        status: row.get(2)?,
        started_at: row.get(3)?,
        finished_at: row.get(4)?,
        summary: row.get(5)?,
        steps: serde_json::from_str(&steps_json).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        collapse_policies, init_schema, load_action_event, load_action_events,
        load_latest_first_stack_smoke_run, load_product_overrides,
        load_service_token_storage_stats, load_suite_settings, replace_overrides,
        replace_repository_policies_with_connection, reserve_pr_slot_with_connection,
        write_suite_settings, ServiceTokenStorageStats,
    };
    use crate::models::{
        now_rfc3339, FirstStackSmokeRun, FirstStackSmokeStep, PrBudgetReservation,
        ProductActionEvent, ProductOverride, RepositoryPolicy, SuiteSettings,
    };
    use patchhive_product_core::repo_policy;
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
    fn replacing_overrides_encrypts_all_stored_credentials_when_key_is_configured() {
        let mut conn = Connection::open_in_memory().expect("in-memory db should open");
        init_schema(&conn).expect("schema should initialize");
        let protector = TokenProtector::from_secret(Some("test-secret"));

        let rows = vec![ProductOverride {
            slug: "signal-hive".into(),
            frontend_url: "https://signal.example.com".into(),
            api_url: "https://signal-api.example.com".into(),
            service_token: "svc_signal".into(),
            legacy_api_key: "legacy_signal".into(),
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
        let raw_legacy: String = conn
            .query_row(
                "SELECT api_key FROM product_overrides WHERE slug = 'signal-hive'",
                [],
                |row| row.get(0),
            )
            .expect("encrypted legacy key should exist");
        assert!(TokenProtector::is_encrypted_value(&raw_legacy));

        let loaded = load_product_overrides(&conn, &protector).expect("rows should decrypt");
        assert_eq!(loaded["signal-hive"].service_token, "svc_signal");
        assert_eq!(loaded["signal-hive"].legacy_api_key, "legacy_signal");

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

    fn policy_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        repo_policy::init_schema(&conn).expect("policy schema should initialize");
        conn
    }

    fn opt_out(repository: &str) -> repo_policy::RepoPolicyEntry {
        repo_policy::RepoPolicyEntry {
            repository: repository.into(),
            kind: repo_policy::PolicyKind::OptOut,
            source: "patchhive.dev".into(),
            notes: "Owner opted out.".into(),
            verified: true,
            updated_at: crate::models::now_rfc3339(),
        }
    }

    #[test]
    fn operator_save_cannot_clear_a_verified_public_opt_out() {
        // The dangerous shape is omission, not an explicit clear: the operator saves
        // a list that simply does not mention the repository. If that deleted the
        // opt-out, a repository owner's request to be left alone would evaporate the
        // next time anyone edited an unrelated row.
        let mut conn = policy_conn();
        repo_policy::upsert(&conn, &opt_out("owner/quiet")).expect("opt-out should save");

        replace_repository_policies_with_connection(
            &mut conn,
            &[RepositoryPolicy {
                repository: "owner/other".into(),
                trusted: true,
                ..RepositoryPolicy::default()
            }],
        )
        .expect("save should succeed");

        let decision = repo_policy::evaluate(&conn, "owner/quiet", "repo-reaper", "scan")
            .expect("evaluation should succeed");
        assert!(!decision.allowed, "opt-out survived the save");
    }

    #[test]
    fn operator_save_replaces_the_kinds_it_owns() {
        // The other half: entries the operator *does* own must actually go away when
        // they are dropped from the list, or the editor would only ever add.
        let mut conn = policy_conn();
        replace_repository_policies_with_connection(
            &mut conn,
            &[RepositoryPolicy {
                repository: "owner/blocked".into(),
                operator_excluded: true,
                ..RepositoryPolicy::default()
            }],
        )
        .expect("first save should succeed");
        assert!(
            !repo_policy::evaluate(&conn, "owner/blocked", "repo-reaper", "scan")
                .unwrap()
                .allowed
        );

        replace_repository_policies_with_connection(&mut conn, &[])
            .expect("second save should succeed");
        assert!(
            repo_policy::evaluate(&conn, "owner/blocked", "repo-reaper", "scan")
                .unwrap()
                .allowed,
            "operator denial was not removed"
        );
    }

    #[test]
    fn collapsing_keeps_every_kind_visible_on_one_row() {
        // The editor saves what it renders. A kind the UI cannot see would be
        // silently dropped by the next save, so collapsing must lose nothing.
        let entry = |repository: &str, kind| repo_policy::RepoPolicyEntry {
            repository: repository.into(),
            kind,
            source: "operator".into(),
            notes: String::new(),
            verified: false,
            updated_at: "2026-07-26T00:00:00Z".into(),
        };
        let rows = collapse_policies(&[
            entry("owner/one", repo_policy::PolicyKind::Allowlist),
            entry("owner/one", repo_policy::PolicyKind::Trusted),
            opt_out("owner/two"),
        ]);

        assert_eq!(rows.len(), 2);
        assert!(rows[0].allowlisted && rows[0].trusted);
        assert!(!rows[0].public_opt_out);
        assert!(rows[1].public_opt_out);
    }
}

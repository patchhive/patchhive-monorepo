use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::contract::{
    cadence_from_hours, interval_cron_label, DispatchActionInput, ScheduleExecutionState,
    SuiteScheduleRecord, TargetSelectionMode,
};

pub const SCHEDULE_TABLE: &str = "patchhive_product_schedules";
pub const DEFAULT_SCHEDULE_APPROVAL_POLICY: &str = "read_only_auto";
pub const MAX_SCHEDULE_NAME_CHARS: usize = 80;
pub const MAX_CADENCE_HOURS: u32 = 8_760;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveProductScheduleRequest<T> {
    pub name: String,
    #[serde(alias = "params")]
    pub payload: T,
    #[serde(default)]
    pub target_selection_mode: TargetSelectionMode,
    pub cadence_hours: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductSchedule {
    pub id: String,
    pub name: String,
    pub product: String,
    pub action_id: String,
    pub payload: Value,
    #[serde(default)]
    pub target_selection_mode: TargetSelectionMode,
    pub cadence_hours: u32,
    pub enabled: bool,
    pub approval_policy: String,
    pub created_at: String,
    pub updated_at: String,
    pub next_run_at: String,
    pub last_execution: ScheduleExecutionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleExecutionResult {
    Completed {
        run_id: Option<String>,
        outcome: String,
    },
    Failed {
        run_id: Option<String>,
        error: String,
    },
    Cancelled {
        run_id: Option<String>,
        reason: Option<String>,
    },
}

impl ScheduleExecutionResult {
    pub fn completed(run_id: Option<impl Into<String>>, outcome: impl Into<String>) -> Self {
        Self::Completed {
            run_id: run_id.map(Into::into),
            outcome: outcome.into(),
        }
    }

    pub fn failed(run_id: Option<impl Into<String>>, error: impl Into<String>) -> Self {
        Self::Failed {
            run_id: run_id.map(Into::into),
            error: error.into(),
        }
    }
}

impl ProductSchedule {
    pub fn decode_payload<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_value(self.payload.clone()).with_context(|| {
            format!(
                "schedule `{}` has an invalid payload for {}:{}",
                self.name, self.product, self.action_id
            )
        })
    }

    pub fn to_suite_schedule_record(&self) -> SuiteScheduleRecord {
        let mut record = SuiteScheduleRecord::new(
            self.id.clone(),
            self.name.clone(),
            self.product.clone(),
            self.action_id.clone(),
        );
        record.cadence = cadence_from_hours(self.cadence_hours);
        record.cron = interval_cron_label(self.cadence_hours);
        record.timezone = "UTC".into();
        record.enabled = self.enabled;
        let mut target_scope = self.payload.clone();
        if let Value::Object(fields) = &mut target_scope {
            fields.insert(
                "target_selection_mode".into(),
                serde_json::to_value(self.target_selection_mode).unwrap_or(Value::Null),
            );
        }
        let mut dispatch = DispatchActionInput {
            payload: target_scope.clone(),
            ..DispatchActionInput::default()
        };
        dispatch
            .path_params
            .insert("name".into(), self.name.clone());
        record.target_scope = target_scope;
        record.approval_policy = self.approval_policy.clone();
        record.next_run_at = self.next_run_at.clone();
        record.last_execution = self.last_execution.clone();
        record.dispatch = dispatch;
        record
    }
}

#[derive(Debug, Clone)]
pub struct SaveSchedule<'a> {
    pub name: &'a str,
    pub product: &'a str,
    pub action_id: &'a str,
    pub payload: &'a Value,
    pub target_selection_mode: TargetSelectionMode,
    pub cadence_hours: u32,
    pub enabled: bool,
    pub approval_policy: &'a str,
}

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS patchhive_product_schedules (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            product TEXT NOT NULL,
            action_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            target_selection_mode TEXT NOT NULL DEFAULT 'direct',
            cadence_hours INTEGER NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            approval_policy TEXT NOT NULL DEFAULT 'read_only_auto',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            next_run_at TEXT NOT NULL,
            last_run_at TEXT,
            last_run_id TEXT,
            last_status TEXT NOT NULL DEFAULT 'idle',
            last_error TEXT,
            UNIQUE(product, action_id, name)
        );

        CREATE INDEX IF NOT EXISTS idx_patchhive_product_schedules_due
        ON patchhive_product_schedules(product, action_id, enabled, next_run_at);
        "#,
    )
    .context("failed to initialize shared product schedules")?;
    ensure_target_selection_mode_column(conn)?;
    Ok(())
}

fn ensure_target_selection_mode_column(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(patchhive_product_schedules)")?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == "target_selection_mode" {
            return Ok(());
        }
    }
    conn.execute_batch(
        "ALTER TABLE patchhive_product_schedules ADD COLUMN target_selection_mode TEXT NOT NULL DEFAULT 'direct';",
    )?;
    Ok(())
}

pub fn list(conn: &Connection, product: &str, action_id: &str) -> Result<Vec<ProductSchedule>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, name, product, action_id, payload_json, target_selection_mode, cadence_hours, enabled,
               approval_policy, created_at, updated_at, next_run_at, last_run_at,
               last_run_id, last_status, last_error
        FROM patchhive_product_schedules
        WHERE product = ?1 AND action_id = ?2
        ORDER BY enabled DESC, next_run_at ASC, name ASC
        "#,
    )?;
    let rows = stmt.query_map(params![product, action_id], schedule_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn get(
    conn: &Connection,
    product: &str,
    action_id: &str,
    name: &str,
) -> Result<Option<ProductSchedule>> {
    conn.query_row(
        r#"
        SELECT id, name, product, action_id, payload_json, target_selection_mode, cadence_hours, enabled,
               approval_policy, created_at, updated_at, next_run_at, last_run_at,
               last_run_id, last_status, last_error
        FROM patchhive_product_schedules
        WHERE product = ?1 AND action_id = ?2 AND name = ?3
        "#,
        params![product, action_id, name],
        schedule_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub fn save(conn: &Connection, input: SaveSchedule<'_>) -> Result<ProductSchedule> {
    validate_identity(input.product, "product")?;
    validate_identity(input.action_id, "action")?;
    let name = validate_schedule_name(input.name)?;
    let cadence_hours = input.cadence_hours.clamp(1, MAX_CADENCE_HOURS);
    let approval_policy = if input.approval_policy.trim().is_empty() {
        DEFAULT_SCHEDULE_APPROVAL_POLICY
    } else {
        input.approval_policy.trim()
    };
    let existing = get(conn, input.product, input.action_id, name)?;
    let now = Utc::now().to_rfc3339();
    let created_at = existing
        .as_ref()
        .map(|schedule| schedule.created_at.clone())
        .unwrap_or_else(|| now.clone());
    let next_run_at = existing
        .as_ref()
        .filter(|schedule| {
            schedule.enabled == input.enabled && schedule.cadence_hours == cadence_hours
        })
        .map(|schedule| schedule.next_run_at.clone())
        .unwrap_or_else(|| next_run_at(cadence_hours));
    let id = schedule_id(input.product, input.action_id, name);
    let payload_json =
        serde_json::to_string(input.payload).context("failed to serialize schedule payload")?;

    conn.execute(
        r#"
        INSERT INTO patchhive_product_schedules(
            id, name, product, action_id, payload_json, target_selection_mode, cadence_hours, enabled,
            approval_policy, created_at, updated_at, next_run_at, last_run_at,
            last_run_id, last_status, last_error
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        ON CONFLICT(product, action_id, name) DO UPDATE SET
            payload_json = excluded.payload_json,
            target_selection_mode = excluded.target_selection_mode,
            cadence_hours = excluded.cadence_hours,
            enabled = excluded.enabled,
            approval_policy = excluded.approval_policy,
            updated_at = excluded.updated_at,
            next_run_at = excluded.next_run_at
        "#,
        params![
            id,
            name,
            input.product,
            input.action_id,
            payload_json,
            target_selection_mode_label(input.target_selection_mode),
            cadence_hours,
            if input.enabled { 1 } else { 0 },
            approval_policy,
            created_at,
            now,
            next_run_at,
            Option::<String>::None,
            Option::<String>::None,
            "idle",
            Option::<String>::None,
        ],
    )?;

    get(conn, input.product, input.action_id, name)?
        .ok_or_else(|| anyhow!("saved schedule `{name}` could not be reloaded"))
}

pub fn delete(conn: &Connection, product: &str, action_id: &str, name: &str) -> Result<bool> {
    Ok(conn.execute(
        "DELETE FROM patchhive_product_schedules WHERE product = ?1 AND action_id = ?2 AND name = ?3",
        params![product, action_id, name],
    )? > 0)
}

pub fn claim_due(
    conn: &mut Connection,
    product: &str,
    action_id: &str,
    limit: usize,
) -> Result<Vec<ProductSchedule>> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let now = Utc::now().to_rfc3339();
    let mut stmt = tx.prepare(
        r#"
        SELECT id, name, product, action_id, payload_json, target_selection_mode, cadence_hours, enabled,
               approval_policy, created_at, updated_at, next_run_at, last_run_at,
               last_run_id, last_status, last_error
        FROM patchhive_product_schedules
        WHERE product = ?1 AND action_id = ?2 AND enabled = 1 AND next_run_at <= ?3
        ORDER BY next_run_at ASC, name ASC
        LIMIT ?4
        "#,
    )?;
    let rows = stmt.query_map(
        params![product, action_id, now, limit.max(1) as i64],
        schedule_from_row,
    )?;
    let mut schedules = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);

    for schedule in &mut schedules {
        let claimed_at = Utc::now().to_rfc3339();
        let advanced_next_run_at = next_run_at(schedule.cadence_hours);
        tx.execute(
            r#"
            UPDATE patchhive_product_schedules
            SET next_run_at = ?2, updated_at = ?3, last_run_at = ?3,
                last_run_id = NULL, last_status = 'claimed', last_error = NULL
            WHERE id = ?1 AND enabled = 1
            "#,
            params![schedule.id, advanced_next_run_at, claimed_at,],
        )?;
        schedule.next_run_at = advanced_next_run_at;
        schedule.updated_at = claimed_at.clone();
        schedule.last_execution = ScheduleExecutionState::Claimed { claimed_at };
    }

    tx.commit()?;
    Ok(schedules)
}

pub fn record_result(
    conn: &Connection,
    product: &str,
    action_id: &str,
    name: &str,
    result: ScheduleExecutionResult,
) -> Result<bool> {
    let (run_id, status, detail) = match result {
        ScheduleExecutionResult::Completed { run_id, outcome } => {
            if outcome.trim().is_empty() {
                return Err(anyhow!(
                    "a completed schedule execution requires an outcome"
                ));
            }
            (run_id, outcome, None)
        }
        ScheduleExecutionResult::Failed { run_id, error } => {
            if error.trim().is_empty() {
                return Err(anyhow!("a failed schedule execution requires an error"));
            }
            (run_id, "failed".into(), Some(error))
        }
        ScheduleExecutionResult::Cancelled { run_id, reason } => {
            (run_id, "cancelled".into(), reason)
        }
    };
    Ok(conn.execute(
        r#"
        UPDATE patchhive_product_schedules
        SET last_run_at = ?4, last_run_id = ?5, last_status = ?6,
            last_error = ?7, updated_at = ?4
        WHERE product = ?1 AND action_id = ?2 AND name = ?3
        "#,
        params![
            product,
            action_id,
            name,
            Utc::now().to_rfc3339(),
            run_id,
            status,
            detail,
        ],
    )? > 0)
}

pub fn next_run_at(cadence_hours: u32) -> String {
    (Utc::now() + Duration::hours(cadence_hours.clamp(1, MAX_CADENCE_HOURS) as i64)).to_rfc3339()
}

pub fn validate_schedule_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("schedule name is required"));
    }
    if name.chars().count() > MAX_SCHEDULE_NAME_CHARS {
        return Err(anyhow!(
            "schedule name must contain at most {MAX_SCHEDULE_NAME_CHARS} characters"
        ));
    }
    if name.chars().any(char::is_control) {
        return Err(anyhow!("schedule name cannot contain control characters"));
    }
    Ok(name)
}

fn validate_identity(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(anyhow!("schedule {label} identifier is invalid"));
    }
    Ok(())
}

fn schedule_id(product: &str, action_id: &str, name: &str) -> String {
    format!("{product}:{action_id}:{name}")
}

fn schedule_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProductSchedule> {
    let payload_json = row.get::<_, String>(4)?;
    let payload = serde_json::from_str(&payload_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let observed_at = row.get(12)?;
    let run_id = row.get(13)?;
    let raw_status = row.get::<_, String>(14)?;
    let detail = row.get(15)?;
    let last_execution = decode_last_execution(observed_at, run_id, raw_status, detail);
    Ok(ProductSchedule {
        id: row.get(0)?,
        name: row.get(1)?,
        product: row.get(2)?,
        action_id: row.get(3)?,
        payload,
        target_selection_mode: parse_target_selection_mode(&row.get::<_, String>(5)?),
        cadence_hours: row.get(6)?,
        enabled: row.get::<_, i64>(7)? != 0,
        approval_policy: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        next_run_at: row.get(11)?,
        last_execution,
    })
}

fn decode_last_execution(
    observed_at: Option<String>,
    run_id: Option<String>,
    raw_status: String,
    detail: Option<String>,
) -> ScheduleExecutionState {
    let status = raw_status.trim().to_ascii_lowercase();
    match status.as_str() {
        "idle" | "never_run" if observed_at.is_none() && run_id.is_none() && detail.is_none() => {
            ScheduleExecutionState::NeverRun
        }
        "claimed" | "running" if observed_at.is_some() && run_id.is_none() && detail.is_none() => {
            ScheduleExecutionState::Claimed {
                claimed_at: observed_at.expect("claimed timestamp checked above"),
            }
        }
        "error" | "failed"
            if observed_at.is_some()
                && detail
                    .as_deref()
                    .is_some_and(|message| !message.trim().is_empty()) =>
        {
            ScheduleExecutionState::Failed {
                failed_at: observed_at.expect("failure timestamp checked above"),
                run_id,
                error: detail.expect("failure detail checked above"),
            }
        }
        "cancelled" | "canceled" if observed_at.is_some() => ScheduleExecutionState::Cancelled {
            cancelled_at: observed_at.expect("cancellation timestamp checked above"),
            run_id,
            reason: detail,
        },
        _ if observed_at.is_some()
            && detail.is_none()
            && !status.is_empty()
            && !matches!(
                status.as_str(),
                "idle" | "never_run" | "claimed" | "running"
            ) =>
        {
            ScheduleExecutionState::Completed {
                completed_at: observed_at.expect("completion timestamp checked above"),
                run_id,
                outcome: raw_status,
            }
        }
        _ => ScheduleExecutionState::Unknown {
            raw_status,
            observed_at,
            run_id,
            detail,
        },
    }
}

fn target_selection_mode_label(mode: TargetSelectionMode) -> &'static str {
    match mode {
        TargetSelectionMode::Direct => "direct",
        TargetSelectionMode::Discovery => "discovery",
    }
}

fn parse_target_selection_mode(value: &str) -> TargetSelectionMode {
    match value.trim() {
        "discovery" => TargetSelectionMode::Discovery,
        _ => TargetSelectionMode::Direct,
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use serde_json::json;

    use super::{
        claim_due, delete, get, init_schema, list, record_result, save, ProductSchedule,
        SaveSchedule, ScheduleExecutionResult, DEFAULT_SCHEDULE_APPROVAL_POLICY,
    };
    use crate::contract::ScheduleExecutionState;

    fn save_daily(conn: &Connection, name: &str) -> ProductSchedule {
        save(
            conn,
            SaveSchedule {
                name,
                product: "refactor-scout",
                action_id: "scan",
                payload: &json!({"repo_path": "patchhive/example", "max_files": 250}),
                target_selection_mode: crate::contract::TargetSelectionMode::Direct,
                cadence_hours: 24,
                enabled: true,
                approval_policy: DEFAULT_SCHEDULE_APPROVAL_POLICY,
            },
        )
        .expect("schedule should save")
    }

    #[test]
    fn schedules_round_trip_as_suite_contracts() {
        let conn = Connection::open_in_memory().expect("database should open");
        init_schema(&conn).expect("schema should initialize");
        let schedule = save_daily(&conn, "daily-review");

        assert_eq!(
            get(&conn, "refactor-scout", "scan", "daily-review")
                .expect("schedule should load")
                .expect("schedule should exist"),
            schedule
        );
        assert_eq!(
            schedule
                .decode_payload::<serde_json::Value>()
                .expect("payload should decode")["repo_path"],
            "patchhive/example"
        );
        let suite = schedule.to_suite_schedule_record();
        assert_eq!(suite.product, "refactor-scout");
        assert_eq!(suite.action_id, "scan");
        assert_eq!(suite.cadence, "daily");
        assert_eq!(suite.dispatch.payload["max_files"], 250);
        assert_eq!(suite.target_scope["target_selection_mode"], "direct");
        assert_eq!(suite.dispatch.payload["target_selection_mode"], "direct");
    }

    #[test]
    fn legacy_schedule_tables_gain_an_explicit_direct_target_mode() {
        let conn = Connection::open_in_memory().expect("database should open");
        conn.execute_batch(
            r#"
            CREATE TABLE patchhive_product_schedules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                product TEXT NOT NULL,
                action_id TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                cadence_hours INTEGER NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                approval_policy TEXT NOT NULL DEFAULT 'read_only_auto',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                next_run_at TEXT NOT NULL,
                last_run_at TEXT,
                last_run_id TEXT,
                last_status TEXT NOT NULL DEFAULT 'idle',
                last_error TEXT,
                UNIQUE(product, action_id, name)
            );
            "#,
        )
        .expect("legacy table should create");

        init_schema(&conn).expect("schema should migrate");
        let schedule = save_daily(&conn, "legacy-direct");
        assert_eq!(
            schedule.target_selection_mode,
            crate::contract::TargetSelectionMode::Direct
        );
    }

    #[test]
    fn schedule_updates_preserve_run_evidence() {
        let conn = Connection::open_in_memory().expect("database should open");
        init_schema(&conn).expect("schema should initialize");
        save_daily(&conn, "daily-review");
        assert!(record_result(
            &conn,
            "refactor-scout",
            "scan",
            "daily-review",
            ScheduleExecutionResult::completed(Some("run-1"), "ok"),
        )
        .expect("result should record"));

        let updated = save_daily(&conn, "daily-review");
        assert!(matches!(
            updated.last_execution,
            ScheduleExecutionState::Completed {
                run_id: Some(ref run_id),
                ref outcome,
                ..
            } if run_id == "run-1" && outcome == "ok"
        ));
        assert_eq!(
            list(&conn, "refactor-scout", "scan")
                .expect("schedules should list")
                .len(),
            1
        );
    }

    #[test]
    fn due_claims_advance_before_dispatch_and_delete_is_scoped() {
        let mut conn = Connection::open_in_memory().expect("database should open");
        init_schema(&conn).expect("schema should initialize");
        let saved = save_daily(&conn, "daily-review");
        conn.execute(
            "UPDATE patchhive_product_schedules SET next_run_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
            [&saved.id],
        )
        .expect("schedule should become due");

        let claimed =
            claim_due(&mut conn, "refactor-scout", "scan", 4).expect("due schedules should claim");
        assert_eq!(claimed.len(), 1);
        let advanced = get(&conn, "refactor-scout", "scan", "daily-review")
            .expect("schedule should reload")
            .expect("schedule should remain");
        assert!(matches!(
            advanced.last_execution,
            ScheduleExecutionState::Claimed { .. }
        ));
        assert!(advanced.next_run_at.as_str() > "2020-01-01T00:00:00Z");
        assert!(delete(&conn, "refactor-scout", "scan", "daily-review")
            .expect("schedule should delete"));
    }

    #[test]
    fn contradictory_legacy_execution_evidence_is_unknown() {
        let conn = Connection::open_in_memory().expect("database should open");
        init_schema(&conn).expect("schema should initialize");
        let saved = save_daily(&conn, "legacy-contradiction");
        conn.execute(
            "UPDATE patchhive_product_schedules SET last_run_id = 'run-stale', last_status = 'idle' WHERE id = ?1",
            [&saved.id],
        )
        .expect("legacy evidence should update");

        let schedule = get(&conn, "refactor-scout", "scan", "legacy-contradiction")
            .expect("schedule should load")
            .expect("schedule should exist");
        assert_eq!(
            schedule.last_execution,
            ScheduleExecutionState::Unknown {
                raw_status: "idle".into(),
                observed_at: None,
                run_id: Some("run-stale".into()),
                detail: None,
            }
        );
    }

    #[test]
    fn failed_results_require_failure_evidence() {
        let conn = Connection::open_in_memory().expect("database should open");
        init_schema(&conn).expect("schema should initialize");
        save_daily(&conn, "failure-without-detail");

        let error = record_result(
            &conn,
            "refactor-scout",
            "scan",
            "failure-without-detail",
            ScheduleExecutionResult::failed(None::<String>, "  "),
        )
        .expect_err("empty failure evidence should be rejected");
        assert!(error
            .to_string()
            .contains("failed schedule execution requires an error"));

        let error = record_result(
            &conn,
            "refactor-scout",
            "scan",
            "failure-without-detail",
            ScheduleExecutionResult::completed(None::<String>, "  "),
        )
        .expect_err("empty completion outcomes should be rejected");
        assert!(error
            .to_string()
            .contains("completed schedule execution requires an outcome"));
    }
}

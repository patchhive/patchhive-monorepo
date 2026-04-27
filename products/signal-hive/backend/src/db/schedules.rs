use anyhow::Result;
use chrono::{Duration, Utc};
use rusqlite::params;

use crate::models::{ScanParams, ScanSchedule};

use super::schema::connect;

fn next_run_at(cadence_hours: u32) -> String {
    (Utc::now() + Duration::hours(cadence_hours.max(1) as i64)).to_rfc3339()
}

fn scan_schedule_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScanSchedule> {
    Ok(ScanSchedule {
        name: row.get(0)?,
        params: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default(),
        cadence_hours: row.get(2)?,
        enabled: row.get::<_, i64>(3)? != 0,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        next_run_at: row.get(6)?,
        last_run_at: row.get(7)?,
        last_scan_id: row.get(8)?,
        last_status: row.get(9)?,
        last_error: row.get(10)?,
    })
}

pub fn list_scan_schedules() -> Result<Vec<ScanSchedule>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT name, params_json, cadence_hours, enabled, created_at, updated_at,
               next_run_at, last_run_at, last_scan_id, last_status, last_error
        FROM scan_schedules
        ORDER BY enabled DESC, next_run_at ASC, name ASC
        "#,
    )?;
    let rows = stmt.query_map([], scan_schedule_from_row)?;

    let mut schedules = Vec::new();
    for row in rows {
        schedules.push(row?);
    }
    Ok(schedules)
}

pub fn get_scan_schedule(name: &str) -> Result<Option<ScanSchedule>> {
    let conn = connect()?;
    conn.query_row(
        r#"
        SELECT name, params_json, cadence_hours, enabled, created_at, updated_at,
               next_run_at, last_run_at, last_scan_id, last_status, last_error
        FROM scan_schedules
        WHERE name = ?1
        "#,
        [name],
        scan_schedule_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub fn save_scan_schedule(
    name: &str,
    params_in: &ScanParams,
    cadence_hours: u32,
    enabled: bool,
) -> Result<()> {
    let conn = connect()?;
    let now = Utc::now().to_rfc3339();
    let existing = get_scan_schedule(name)?;
    let created_at = existing
        .as_ref()
        .map(|schedule| schedule.created_at.clone())
        .unwrap_or_else(|| now.clone());
    let next_run = if let Some(schedule) = &existing {
        if schedule.enabled == enabled && schedule.cadence_hours == cadence_hours.max(1) {
            schedule.next_run_at.clone()
        } else {
            next_run_at(cadence_hours)
        }
    } else {
        next_run_at(cadence_hours)
    };

    conn.execute(
        r#"
        INSERT OR REPLACE INTO scan_schedules(
            name, params_json, cadence_hours, enabled, created_at, updated_at,
            next_run_at, last_run_at, last_scan_id, last_status, last_error
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            name,
            serde_json::to_string(params_in)?,
            cadence_hours.max(1),
            if enabled { 1 } else { 0 },
            created_at,
            now,
            next_run,
            existing
                .as_ref()
                .and_then(|schedule| schedule.last_run_at.clone()),
            existing
                .as_ref()
                .and_then(|schedule| schedule.last_scan_id.clone()),
            existing
                .as_ref()
                .map(|schedule| schedule.last_status.clone())
                .unwrap_or_else(|| "idle".into()),
            existing
                .as_ref()
                .and_then(|schedule| schedule.last_error.clone()),
        ],
    )?;
    Ok(())
}

pub fn delete_scan_schedule(name: &str) -> Result<()> {
    let conn = connect()?;
    conn.execute("DELETE FROM scan_schedules WHERE name = ?1", [name])?;
    Ok(())
}

pub fn claim_due_scan_schedules(limit: usize) -> Result<Vec<ScanSchedule>> {
    let conn = connect()?;
    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        r#"
        SELECT name, params_json, cadence_hours, enabled, created_at, updated_at,
               next_run_at, last_run_at, last_scan_id, last_status, last_error
        FROM scan_schedules
        WHERE enabled = 1 AND next_run_at <= ?1
        ORDER BY next_run_at ASC, name ASC
        LIMIT ?2
        "#,
    )?;

    let rows = stmt.query_map(params![now, limit as i64], scan_schedule_from_row)?;
    let mut schedules = Vec::new();
    for row in rows {
        let schedule = row?;
        conn.execute(
            r#"
            UPDATE scan_schedules
            SET next_run_at = ?2, updated_at = ?3, last_status = 'running', last_error = NULL
            WHERE name = ?1
            "#,
            params![
                schedule.name,
                next_run_at(schedule.cadence_hours),
                Utc::now().to_rfc3339()
            ],
        )?;
        schedules.push(schedule);
    }

    Ok(schedules)
}

pub fn record_scan_schedule_result(
    name: &str,
    last_scan_id: Option<&str>,
    status: &str,
    error: Option<&str>,
) -> Result<()> {
    let conn = connect()?;
    conn.execute(
        r#"
        UPDATE scan_schedules
        SET last_run_at = ?2, last_scan_id = ?3, last_status = ?4, last_error = ?5, updated_at = ?6
        WHERE name = ?1
        "#,
        params![
            name,
            Utc::now().to_rfc3339(),
            last_scan_id,
            status,
            error,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

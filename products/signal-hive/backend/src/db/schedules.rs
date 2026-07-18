use anyhow::Result;
use patchhive_product_core::scheduling::{
    self, ProductSchedule, SaveSchedule, DEFAULT_SCHEDULE_APPROVAL_POLICY,
};

use crate::models::{ScanParams, ScanSchedule};

use super::schema::connect;

const PRODUCT: &str = "signal-hive";
const ACTION: &str = "scan";

fn scan_schedule(record: ProductSchedule) -> Result<ScanSchedule> {
    let params: ScanParams = record.decode_payload()?;
    Ok(ScanSchedule {
        name: record.name,
        target_selection_mode: params.target_selection_mode(),
        params,
        cadence_hours: record.cadence_hours,
        enabled: record.enabled,
        created_at: record.created_at,
        updated_at: record.updated_at,
        next_run_at: record.next_run_at,
        last_run_at: record.last_run_at,
        last_scan_id: record.last_run_id,
        last_status: record.last_status,
        last_error: record.last_error,
    })
}

pub(crate) fn migrate_legacy_scan_schedules() -> Result<()> {
    let conn = connect()?;
    scheduling::init_schema(&conn)?;
    conn.execute_batch(
        r#"
        INSERT OR IGNORE INTO patchhive_product_schedules(
            id, name, product, action_id, payload_json, cadence_hours, enabled,
            approval_policy, created_at, updated_at, next_run_at, last_run_at,
            last_run_id, last_status, last_error
        )
        SELECT
            'signal-hive:scan:' || name,
            name,
            'signal-hive',
            'scan',
            params_json,
            cadence_hours,
            enabled,
            'read_only_auto',
            created_at,
            updated_at,
            next_run_at,
            last_run_at,
            last_scan_id,
            last_status,
            last_error
        FROM scan_schedules;
        "#,
    )?;
    Ok(())
}

pub fn list_scan_schedules() -> Result<Vec<ScanSchedule>> {
    let conn = connect()?;
    scheduling::list(&conn, PRODUCT, ACTION)?
        .into_iter()
        .map(scan_schedule)
        .collect()
}

pub fn get_scan_schedule(name: &str) -> Result<Option<ScanSchedule>> {
    let conn = connect()?;
    scheduling::get(&conn, PRODUCT, ACTION, name)?
        .map(scan_schedule)
        .transpose()
}

pub fn save_scan_schedule(
    name: &str,
    params: &ScanParams,
    cadence_hours: u32,
    enabled: bool,
) -> Result<()> {
    let conn = connect()?;
    let payload = serde_json::to_value(params)?;
    scheduling::save(
        &conn,
        SaveSchedule {
            name,
            product: PRODUCT,
            action_id: ACTION,
            payload: &payload,
            cadence_hours,
            enabled,
            approval_policy: DEFAULT_SCHEDULE_APPROVAL_POLICY,
        },
    )?;
    Ok(())
}

pub fn delete_scan_schedule(name: &str) -> Result<()> {
    let conn = connect()?;
    scheduling::delete(&conn, PRODUCT, ACTION, name)?;
    Ok(())
}

pub fn claim_due_scan_schedules(limit: usize) -> Result<Vec<ScanSchedule>> {
    let mut conn = connect()?;
    scheduling::claim_due(&mut conn, PRODUCT, ACTION, limit)?
        .into_iter()
        .map(scan_schedule)
        .collect()
}

pub fn record_scan_schedule_result(
    name: &str,
    last_scan_id: Option<&str>,
    status: &str,
    error: Option<&str>,
) -> Result<()> {
    let conn = connect()?;
    scheduling::record_result(&conn, PRODUCT, ACTION, name, last_scan_id, status, error)?;
    Ok(())
}

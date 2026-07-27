use anyhow::Result;
use chrono::Utc;
use patchhive_product_core::repo_policy;
use patchhive_product_core::scope_policy::{
    normalize_repo_name as normalize_scope_repo_name, RepoListType,
};
use rusqlite::params;
use std::collections::HashSet;

use crate::models::{RepoListItem, ScanParams, ScanPreset};

use super::schema::connect;

pub fn scan_count() -> u32 {
    connect()
        .ok()
        .and_then(|conn| {
            conn.query_row("SELECT COUNT(*) FROM scans", [], |row| row.get(0))
                .ok()
        })
        .unwrap_or(0)
}

pub fn normalize_repo_list_type(value: &str) -> Option<&'static str> {
    RepoListType::parse(value).map(RepoListType::as_str)
}

pub fn normalize_repo_name(value: &str) -> Option<String> {
    normalize_scope_repo_name(value)
}

// Repository lists live in the suite-wide `patchhive_repo_policy` table, not in
// SignalHive's own `repo_lists`. An operator who tells one product to stay off a
// repository means all of them: the same owner, the same wishes, and no reason the
// answer should depend on which product asked. SignalHive's table remains on disk as
// migration input and is no longer read.
pub fn list_repo_lists() -> Result<Vec<RepoListItem>> {
    let conn = connect()?;
    let mut repos = repo_policy::list(&conn)?
        .into_iter()
        // Trust is an elevation for operations SignalHive does not perform. Showing
        // it in a scan-scope list would imply this product acts on it.
        .filter(|entry| entry.kind != repo_policy::PolicyKind::Trusted)
        .map(|entry| RepoListItem {
            repo: entry.repository,
            list_type: entry.kind.as_str().to_string(),
            added_at: entry.updated_at,
        })
        .collect::<Vec<_>>();
    repos.sort_by(|left, right| {
        left.list_type
            .cmp(&right.list_type)
            .then_with(|| left.repo.cmp(&right.repo))
    });
    Ok(repos)
}

pub fn save_repo_list(repo: &str, list_type: &str) -> Result<()> {
    let conn = connect()?;
    repo_policy::record_listing(&conn, repo, list_type, "signal-hive")?;
    Ok(())
}

pub fn delete_repo_list(repo: &str) -> Result<()> {
    let conn = connect()?;
    repo_policy::remove_listings(&conn, repo)?;
    Ok(())
}

pub fn repo_list_sets() -> Result<(HashSet<String>, HashSet<String>, HashSet<String>)> {
    let conn = connect()?;
    let policy = repo_policy::scope_policy(&conn)?;
    Ok((policy.allowlist, policy.denylist, policy.opt_out))
}

pub fn list_scan_presets() -> Result<Vec<ScanPreset>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        "SELECT name, params_json, created_at, updated_at FROM scan_presets ORDER BY updated_at DESC, name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let params: ScanParams =
            serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default();
        Ok(ScanPreset {
            name: row.get(0)?,
            target_selection_mode: params.target_selection_mode(),
            params,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    })?;

    let mut presets = Vec::new();
    for row in rows {
        presets.push(row?);
    }
    Ok(presets)
}

pub fn save_scan_preset(name: &str, params_in: &ScanParams) -> Result<()> {
    let conn = connect()?;
    let now = Utc::now().to_rfc3339();
    let existing_created_at: Option<String> = conn
        .query_row(
            "SELECT created_at FROM scan_presets WHERE name = ?1",
            [name],
            |row| row.get(0),
        )
        .ok();

    conn.execute(
        "INSERT OR REPLACE INTO scan_presets(name, params_json, created_at, updated_at) VALUES(?1, ?2, ?3, ?4)",
        params![
            name,
            serde_json::to_string(params_in)?,
            existing_created_at.unwrap_or_else(|| now.clone()),
            now,
        ],
    )?;
    Ok(())
}

pub fn delete_scan_preset(name: &str) -> Result<()> {
    let conn = connect()?;
    conn.execute("DELETE FROM scan_presets WHERE name = ?1", [name])?;
    Ok(())
}

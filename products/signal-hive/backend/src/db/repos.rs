use anyhow::Result;
use chrono::Utc;
use patchhive_product_core::scope_policy::{
    normalize_repo_name as normalize_scope_repo_name, RepoListType, RepoScopePolicy,
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

pub fn list_repo_lists() -> Result<Vec<RepoListItem>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        "SELECT repo, list_type, added_at FROM repo_lists ORDER BY list_type ASC, repo ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let list_type: String = row.get(1)?;
        Ok(RepoListItem {
            repo: row.get(0)?,
            list_type: normalize_repo_list_type(&list_type)
                .unwrap_or("denylist")
                .to_string(),
            added_at: row.get(2)?,
        })
    })?;

    let mut repos = Vec::new();
    for row in rows {
        repos.push(row?);
    }
    Ok(repos)
}

pub fn save_repo_list(repo: &str, list_type: &str) -> Result<()> {
    let conn = connect()?;
    conn.execute(
        "INSERT OR REPLACE INTO repo_lists(repo, list_type, added_at) VALUES(?1, ?2, ?3)",
        params![repo, list_type, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn delete_repo_list(repo: &str) -> Result<()> {
    let conn = connect()?;
    conn.execute("DELETE FROM repo_lists WHERE repo = ?1", [repo])?;
    Ok(())
}

pub fn repo_list_sets() -> Result<(HashSet<String>, HashSet<String>, HashSet<String>)> {
    let rows = list_repo_lists()?;
    let policy = RepoScopePolicy::from_entries(
        rows.iter()
            .map(|row| (row.repo.as_str(), row.list_type.as_str())),
    );
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

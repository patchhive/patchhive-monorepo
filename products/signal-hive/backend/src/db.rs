use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

use crate::models::{
    RepoListItem, RepoSignal, ScanHistoryItem, ScanParams, ScanPreset, ScanRecord, ScanSchedule,
    ScanSummary, ScanTimeline, ScanTimelinePoint,
};

pub fn db_path() -> String {
    std::env::var("SIGNAL_DB_PATH").unwrap_or_else(|_| "signal-hive.db".into())
}

fn connect() -> Result<Connection> {
    Connection::open(db_path()).context("failed to open SignalHive database")
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for row in rows {
        if row? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    if !column_exists(conn, table, column)? {
        conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {definition};"))?;
    }
    Ok(())
}

pub fn init_db() -> Result<()> {
    let conn = connect()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS scans (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            params_signature TEXT NOT NULL DEFAULT '',
            trigger_type TEXT NOT NULL DEFAULT 'manual',
            schedule_name TEXT,
            search_query TEXT NOT NULL,
            topics_json TEXT NOT NULL,
            languages_json TEXT NOT NULL,
            min_stars INTEGER NOT NULL,
            max_repos INTEGER NOT NULL,
            issues_per_repo INTEGER NOT NULL,
            stale_days INTEGER NOT NULL,
            total_repos INTEGER NOT NULL,
            total_signals INTEGER NOT NULL,
            top_repo TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS repo_signals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_id TEXT NOT NULL,
            repo_full_name TEXT NOT NULL,
            repo_url TEXT NOT NULL,
            description TEXT NOT NULL,
            language TEXT NOT NULL,
            stars INTEGER NOT NULL,
            open_issues INTEGER NOT NULL,
            sampled_issues INTEGER NOT NULL DEFAULT 0,
            stale_issues INTEGER NOT NULL,
            unlabeled_issues INTEGER NOT NULL DEFAULT 0,
            stale_bug_issues INTEGER NOT NULL DEFAULT 0,
            stale_high_comment_issues INTEGER NOT NULL DEFAULT 0,
            duplicate_candidates_json TEXT NOT NULL,
            todo_count INTEGER NOT NULL,
            fixme_count INTEGER NOT NULL,
            priority_score REAL NOT NULL,
            score_breakdown_json TEXT NOT NULL DEFAULT '[]',
            summary TEXT NOT NULL,
            signals_json TEXT NOT NULL,
            issue_examples_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS repo_lists (
            repo TEXT PRIMARY KEY,
            list_type TEXT NOT NULL,
            added_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS scan_presets (
            name TEXT PRIMARY KEY,
            params_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS scan_schedules (
            name TEXT PRIMARY KEY,
            params_json TEXT NOT NULL,
            cadence_hours INTEGER NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            next_run_at TEXT NOT NULL,
            last_run_at TEXT,
            last_scan_id TEXT,
            last_status TEXT NOT NULL DEFAULT 'idle',
            last_error TEXT
        );
        "#,
    )?;

    ensure_column(
        &conn,
        "scans",
        "params_signature",
        "params_signature TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "scans",
        "trigger_type",
        "trigger_type TEXT NOT NULL DEFAULT 'manual'",
    )?;
    ensure_column(
        &conn,
        "scans",
        "schedule_name",
        "schedule_name TEXT",
    )?;

    ensure_column(
        &conn,
        "repo_signals",
        "sampled_issues",
        "sampled_issues INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        &conn,
        "repo_signals",
        "unlabeled_issues",
        "unlabeled_issues INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        &conn,
        "repo_signals",
        "stale_bug_issues",
        "stale_bug_issues INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        &conn,
        "repo_signals",
        "stale_high_comment_issues",
        "stale_high_comment_issues INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        &conn,
        "repo_signals",
        "score_breakdown_json",
        "score_breakdown_json TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        &conn,
        "repo_signals",
        "recurring_bug_clusters_json",
        "recurring_bug_clusters_json TEXT NOT NULL DEFAULT '[]'",
    )?;
    Ok(())
}

fn normalized_signature_parts(values: &[String]) -> Vec<String> {
    let mut parts = values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    parts.sort();
    parts.dedup();
    parts
}

pub fn params_signature(params: &ScanParams) -> String {
    serde_json::json!({
        "search_query": params.search_query.trim().to_ascii_lowercase(),
        "topics": normalized_signature_parts(&params.topics),
        "languages": normalized_signature_parts(&params.languages),
        "min_stars": params.min_stars,
        "max_repos": params.max_repos,
        "issues_per_repo": params.issues_per_repo,
        "stale_days": params.stale_days,
    })
    .to_string()
}

fn next_run_at(cadence_hours: u32) -> String {
    (Utc::now() + Duration::hours(cadence_hours.max(1) as i64)).to_rfc3339()
}

pub fn save_scan(
    params_in: &ScanParams,
    repos: &[RepoSignal],
    trigger_type: &str,
    schedule_name: Option<&str>,
) -> Result<ScanRecord> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let params_signature = params_signature(params_in);
    let summary = ScanSummary {
        total_repos: repos.len() as u32,
        total_signals: repos.iter().map(|repo| repo.signals.len() as u32).sum(),
        top_repo: repos
            .first()
            .map(|repo| repo.full_name.clone())
            .unwrap_or_else(|| "none".into()),
    };

    let record = ScanRecord {
        id: id.clone(),
        created_at: created_at.clone(),
        params: params_in.clone(),
        summary: summary.clone(),
        repos: repos.to_vec(),
        trigger_type: trigger_type.to_string(),
        schedule_name: schedule_name.map(|value| value.to_string()),
        trend: None,
    };

    let conn = connect()?;
    let tx = conn.unchecked_transaction()?;

    tx.execute(
        r#"
        INSERT INTO scans (
            id, created_at, params_signature, trigger_type, schedule_name,
            search_query, topics_json, languages_json,
            min_stars, max_repos, issues_per_repo, stale_days,
            total_repos, total_signals, top_repo
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        "#,
        params![
            record.id,
            record.created_at,
            params_signature,
            record.trigger_type,
            record.schedule_name,
            record.params.search_query,
            serde_json::to_string(&record.params.topics)?,
            serde_json::to_string(&record.params.languages)?,
            record.params.min_stars,
            record.params.max_repos,
            record.params.issues_per_repo,
            record.params.stale_days,
            record.summary.total_repos,
            record.summary.total_signals,
            record.summary.top_repo,
        ],
    )?;

    for repo in repos {
        tx.execute(
            r#"
            INSERT INTO repo_signals (
                scan_id, repo_full_name, repo_url, description, language, stars,
                open_issues, sampled_issues, stale_issues, unlabeled_issues,
                stale_bug_issues, stale_high_comment_issues, duplicate_candidates_json,
                recurring_bug_clusters_json, todo_count, fixme_count, priority_score,
                score_breakdown_json, summary, signals_json, issue_examples_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
            "#,
            params![
                record.id,
                repo.full_name,
                repo.repo_url,
                repo.description,
                repo.language,
                repo.stars,
                repo.open_issues,
                repo.sampled_issues,
                repo.stale_issues,
                repo.unlabeled_issues,
                repo.stale_bug_issues,
                repo.stale_high_comment_issues,
                serde_json::to_string(&repo.duplicate_candidates)?,
                serde_json::to_string(&repo.recurring_bug_clusters)?,
                repo.todo_count,
                repo.fixme_count,
                repo.priority_score,
                serde_json::to_string(&repo.score_breakdown)?,
                repo.summary,
                serde_json::to_string(&repo.signals)?,
                serde_json::to_string(&repo.issue_examples)?,
            ],
        )?;
    }

    tx.commit()?;
    Ok(record)
}

pub fn list_scans() -> Result<Vec<ScanHistoryItem>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT id, created_at, search_query, topics_json, languages_json,
               max_repos, total_repos, total_signals, top_repo, trigger_type, schedule_name
        FROM scans
        ORDER BY created_at DESC
        LIMIT 25
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ScanHistoryItem {
            id: row.get(0)?,
            created_at: row.get(1)?,
            search_query: row.get(2)?,
            topics: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(3)?).unwrap_or_default(),
            languages: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(4)?).unwrap_or_default(),
            max_repos: row.get(5)?,
            total_repos: row.get(6)?,
            total_signals: row.get(7)?,
            top_repo: row.get(8)?,
            trigger_type: row.get(9)?,
            schedule_name: row.get(10)?,
        })
    })?;

    let mut scans = Vec::new();
    for row in rows {
        scans.push(row?);
    }
    Ok(scans)
}

pub fn get_scan(id: &str) -> Result<Option<ScanRecord>> {
    let conn = connect()?;

    let scan_row = conn.query_row(
        r#"
        SELECT created_at, search_query, topics_json, languages_json,
               min_stars, max_repos, issues_per_repo, stale_days,
               total_repos, total_signals, top_repo, trigger_type, schedule_name
        FROM scans
        WHERE id = ?1
        "#,
        params![id],
        |row| {
            Ok(ScanRecord {
                id: id.to_string(),
                created_at: row.get(0)?,
                params: ScanParams {
                    search_query: row.get(1)?,
                    topics: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                    languages: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                    min_stars: row.get(4)?,
                    max_repos: row.get(5)?,
                    issues_per_repo: row.get(6)?,
                    stale_days: row.get(7)?,
                },
                summary: ScanSummary {
                    total_repos: row.get(8)?,
                    total_signals: row.get(9)?,
                    top_repo: row.get(10)?,
                },
                repos: Vec::new(),
                trigger_type: row.get(11)?,
                schedule_name: row.get(12)?,
                trend: None,
            })
        },
    );

    let mut record = match scan_row {
        Ok(record) => record,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    let mut stmt = conn.prepare(
        r#"
        SELECT repo_full_name, repo_url, description, language, stars,
               open_issues, sampled_issues, stale_issues, unlabeled_issues,
               stale_bug_issues, stale_high_comment_issues, duplicate_candidates_json,
               recurring_bug_clusters_json, todo_count, fixme_count, priority_score,
               score_breakdown_json, summary, signals_json, issue_examples_json
        FROM repo_signals
        WHERE scan_id = ?1
        ORDER BY priority_score DESC, stars DESC
        "#,
    )?;

    let rows = stmt.query_map(params![id], |row| {
        Ok(RepoSignal {
            full_name: row.get(0)?,
            repo_url: row.get(1)?,
            description: row.get(2)?,
            language: row.get(3)?,
            stars: row.get(4)?,
            open_issues: row.get(5)?,
            sampled_issues: row.get(6)?,
            stale_issues: row.get(7)?,
            unlabeled_issues: row.get(8)?,
            stale_bug_issues: row.get(9)?,
            stale_high_comment_issues: row.get(10)?,
            duplicate_candidates: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
            recurring_bug_clusters: serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or_default(),
            todo_count: row.get(13)?,
            fixme_count: row.get(14)?,
            priority_score: row.get(15)?,
            score_breakdown: serde_json::from_str(&row.get::<_, String>(16)?).unwrap_or_default(),
            summary: row.get(17)?,
            signals: serde_json::from_str(&row.get::<_, String>(18)?).unwrap_or_default(),
            issue_examples: serde_json::from_str(&row.get::<_, String>(19)?).unwrap_or_default(),
            trend: None,
        })
    })?;

    for row in rows {
        record.repos.push(row?);
    }

    Ok(Some(record))
}

pub fn scan_timeline(id: &str, limit: usize) -> Result<Option<ScanTimeline>> {
    let conn = connect()?;
    let (signature, created_at): (String, String) = match conn.query_row(
        "SELECT params_signature, created_at FROM scans WHERE id = ?1",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ) {
        Ok(values) => values,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    let mut stmt = conn.prepare(
        r#"
        SELECT s.id,
               s.created_at,
               s.total_repos,
               s.total_signals,
               s.top_repo,
               s.trigger_type,
               s.schedule_name,
               COALESCE((SELECT SUM(stale_issues) FROM repo_signals WHERE scan_id = s.id), 0) AS total_stale_issues,
               COALESCE((SELECT AVG(priority_score) FROM repo_signals WHERE scan_id = s.id), 0) AS avg_priority_score,
               COALESCE((SELECT MAX(priority_score) FROM repo_signals WHERE scan_id = s.id), 0) AS top_priority_score
        FROM scans s
        WHERE s.params_signature = ?1 AND s.created_at <= ?2
        ORDER BY s.created_at DESC
        LIMIT ?3
        "#,
    )?;

    let rows = stmt.query_map(params![signature, created_at, limit as i64], |row| {
        Ok(ScanTimelinePoint {
            id: row.get(0)?,
            created_at: row.get(1)?,
            total_repos: row.get(2)?,
            total_signals: row.get(3)?,
            top_repo: row.get(4)?,
            trigger_type: row.get(5)?,
            schedule_name: row.get(6)?,
            total_stale_issues: row.get(7)?,
            avg_priority_score: row.get(8)?,
            top_priority_score: row.get(9)?,
        })
    })?;

    let mut points = Vec::new();
    for row in rows {
        points.push(row?);
    }
    points.reverse();

    Ok(Some(ScanTimeline {
        current_scan_id: id.to_string(),
        points,
    }))
}

pub fn previous_scan_for_params(
    current_id: &str,
    current_created_at: &str,
    params: &ScanParams,
) -> Result<Option<ScanRecord>> {
    let conn = connect()?;
    let signature = params_signature(params);
    let previous_id: Option<String> = conn
        .query_row(
            r#"
            SELECT id
            FROM scans
            WHERE params_signature = ?1 AND created_at < ?2 AND id != ?3
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            params![signature, current_created_at, current_id],
            |row| row.get(0),
        )
        .optional()?;

    match previous_id {
        Some(id) => get_scan(&id),
        None => Ok(None),
    }
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
            existing.as_ref().and_then(|schedule| schedule.last_run_at.clone()),
            existing.as_ref().and_then(|schedule| schedule.last_scan_id.clone()),
            existing
                .as_ref()
                .map(|schedule| schedule.last_status.clone())
                .unwrap_or_else(|| "idle".into()),
            existing.as_ref().and_then(|schedule| schedule.last_error.clone()),
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
            params![schedule.name, next_run_at(schedule.cadence_hours), Utc::now().to_rfc3339()],
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

pub fn scan_count() -> u32 {
    connect()
        .ok()
        .and_then(|conn| conn.query_row("SELECT COUNT(*) FROM scans", [], |row| row.get(0)).ok())
        .unwrap_or(0)
}

pub fn normalize_repo_list_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "allowlist" => Some("allowlist"),
        "denylist" | "blocklist" => Some("denylist"),
        "opt_out" | "opt-out" | "optout" => Some("opt_out"),
        _ => None,
    }
}

pub fn normalize_repo_name(value: &str) -> Option<String> {
    let mut parts = value
        .trim()
        .split('/')
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| !part.is_empty());
    let owner = parts.next()?;
    let repo = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
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
    let allow = rows
        .iter()
        .filter(|row| row.list_type == "allowlist")
        .map(|row| row.repo.clone())
        .collect();
    let deny = rows
        .iter()
        .filter(|row| row.list_type == "denylist")
        .map(|row| row.repo.clone())
        .collect();
    let opt_out = rows
        .iter()
        .filter(|row| row.list_type == "opt_out")
        .map(|row| row.repo.clone())
        .collect();
    Ok((allow, deny, opt_out))
}

pub fn list_scan_presets() -> Result<Vec<ScanPreset>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        "SELECT name, params_json, created_at, updated_at FROM scan_presets ORDER BY updated_at DESC, name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ScanPreset {
            name: row.get(0)?,
            params: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default(),
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

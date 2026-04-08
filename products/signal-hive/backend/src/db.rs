use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::collections::HashSet;

use crate::models::{
    RepoListItem, RepoSignal, ScanHistoryItem, ScanParams, ScanPreset, ScanRecord, ScanSummary,
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
        "#,
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

pub fn save_scan(params_in: &ScanParams, repos: &[RepoSignal]) -> Result<ScanRecord> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
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
    };

    let conn = connect()?;
    let tx = conn.unchecked_transaction()?;

    tx.execute(
        r#"
        INSERT INTO scans (
            id, created_at, search_query, topics_json, languages_json,
            min_stars, max_repos, issues_per_repo, stale_days,
            total_repos, total_signals, top_repo
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
        params![
            record.id,
            record.created_at,
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
               max_repos, total_repos, total_signals, top_repo
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
               total_repos, total_signals, top_repo
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
        })
    })?;

    for row in rows {
        record.repos.push(row?);
    }

    Ok(Some(record))
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

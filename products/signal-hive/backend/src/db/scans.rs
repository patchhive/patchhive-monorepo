use anyhow::Result;
use chrono::{Duration, Utc};
use rusqlite::params;

use crate::models::{
    RepoSignal, ScanHistoryItem, ScanParams, ScanRecord, ScanSummary, ScanTimeline,
    ScanTimelinePoint,
};

use super::schema::{connect, init_schema};

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

pub fn save_scan(
    params_in: &ScanParams,
    repos: &[RepoSignal],
    warnings: &[String],
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
        warnings: warnings.to_vec(),
        trigger_type: trigger_type.to_string(),
        schedule_name: schedule_name.map(|value| value.to_string()),
        trend: None,
    };

    let conn = connect()?;
    let tx = conn.unchecked_transaction()?;

    tx.execute(
        r#"
        INSERT INTO scans (
            id, created_at, params_signature, warnings_json, trigger_type, schedule_name,
            search_query, topics_json, languages_json,
            min_stars, max_repos, issues_per_repo, stale_days,
            total_repos, total_signals, top_repo
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        "#,
        params![
            record.id,
            record.created_at,
            params_signature,
            serde_json::to_string(&record.warnings)?,
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
        insert_repo_signal(&tx, &record.id, repo)?;
    }

    tx.commit()?;
    Ok(record)
}

pub(crate) fn insert_repo_signal(
    tx: &rusqlite::Transaction<'_>,
    scan_id: &str,
    repo: &RepoSignal,
) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO repo_signals (
            scan_id, repo_full_name, repo_url, description, language, stars,
            open_issues, sampled_issues, stale_issues, unlabeled_issues,
            stale_bug_issues, stale_high_comment_issues, duplicate_candidates_json,
            recurring_bug_clusters_json, todo_count, fixme_count, todo_available,
            fixme_available, priority_score, score_breakdown_json, summary,
            signals_json, issue_examples_json, warnings_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
        "#,
        params![
            scan_id,
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
            repo.todo_available,
            repo.fixme_available,
            repo.priority_score,
            serde_json::to_string(&repo.score_breakdown)?,
            repo.summary,
            serde_json::to_string(&repo.signals)?,
            serde_json::to_string(&repo.issue_examples)?,
            serde_json::to_string(&repo.warnings)?,
        ],
    )?;
    Ok(())
}

pub fn list_scans() -> Result<Vec<ScanHistoryItem>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT id, created_at, search_query, topics_json, languages_json,
               max_repos, total_repos, total_signals, top_repo, warnings_json, trigger_type, schedule_name
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
            topics: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(3)?)
                .unwrap_or_default(),
            languages: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(4)?)
                .unwrap_or_default(),
            max_repos: row.get(5)?,
            total_repos: row.get(6)?,
            total_signals: row.get(7)?,
            top_repo: row.get(8)?,
            warning_count: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(9)?)
                .unwrap_or_default()
                .len() as u32,
            trigger_type: row.get(10)?,
            schedule_name: row.get(11)?,
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
               total_repos, total_signals, top_repo, warnings_json, trigger_type, schedule_name
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
                warnings: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                repos: Vec::new(),
                trigger_type: row.get(12)?,
                schedule_name: row.get(13)?,
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
               recurring_bug_clusters_json, todo_count, fixme_count, todo_available,
               fixme_available, priority_score, score_breakdown_json, summary,
               signals_json, issue_examples_json, warnings_json
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
            duplicate_candidates: serde_json::from_str(&row.get::<_, String>(11)?)
                .unwrap_or_default(),
            recurring_bug_clusters: serde_json::from_str(&row.get::<_, String>(12)?)
                .unwrap_or_default(),
            todo_count: row.get(13)?,
            fixme_count: row.get(14)?,
            todo_available: row.get(15)?,
            fixme_available: row.get(16)?,
            priority_score: row.get(17)?,
            score_breakdown: serde_json::from_str(&row.get::<_, String>(18)?).unwrap_or_default(),
            summary: row.get(19)?,
            signals: serde_json::from_str(&row.get::<_, String>(20)?).unwrap_or_default(),
            issue_examples: serde_json::from_str(&row.get::<_, String>(21)?).unwrap_or_default(),
            warnings: serde_json::from_str(&row.get::<_, String>(22)?).unwrap_or_default(),
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

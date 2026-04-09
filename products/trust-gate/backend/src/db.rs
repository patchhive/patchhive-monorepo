use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{RepoRuleSet, ReviewHistoryItem, ReviewResult, SavedRuleSet};

pub fn db_path() -> String {
    std::env::var("TRUST_DB_PATH").unwrap_or_else(|_| "trust-gate.db".into())
}

fn connect() -> Result<Connection> {
    Connection::open(db_path()).context("failed to open TrustGate database")
}

pub fn normalize_repo_name(repo: &str) -> Option<String> {
    let trimmed = repo.trim().trim_matches('/');
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.split('/');
    let owner = parts.next()?.trim();
    let name = parts.next()?.trim();
    if owner.is_empty() || name.is_empty() || parts.next().is_some() {
        return None;
    }

    Some(format!("{owner}/{name}"))
}

pub fn init_db() -> Result<()> {
    let conn = connect()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS rule_sets (
          repo TEXT PRIMARY KEY,
          rules_json TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS reviews (
          id TEXT PRIMARY KEY,
          repo TEXT NOT NULL,
          created_at TEXT NOT NULL,
          ai_source TEXT NOT NULL,
          recommendation TEXT NOT NULL,
          risk_score INTEGER NOT NULL,
          files_changed INTEGER NOT NULL,
          summary TEXT NOT NULL,
          review_json TEXT NOT NULL
        );
        "#,
    )
    .context("failed to initialize TrustGate schema")?;
    Ok(())
}

pub fn save_rules(repo: &str, rules: &RepoRuleSet) -> Result<()> {
    let conn = connect()?;
    let now = Utc::now().to_rfc3339();
    let rules_json = serde_json::to_string(rules).context("failed to serialize rule set")?;

    conn.execute(
        r#"
        INSERT INTO rule_sets (repo, rules_json, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(repo) DO UPDATE SET
          rules_json = excluded.rules_json,
          updated_at = excluded.updated_at
        "#,
        params![repo, rules_json, now, now],
    )
    .context("failed to save TrustGate rule set")?;

    Ok(())
}

pub fn list_rules() -> Result<Vec<SavedRuleSet>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        "SELECT repo, rules_json, created_at, updated_at FROM rule_sets ORDER BY updated_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        let repo: String = row.get(0)?;
        let rules_json: String = row.get(1)?;
        let created_at: String = row.get(2)?;
        let updated_at: String = row.get(3)?;
        Ok((repo, rules_json, created_at, updated_at))
    })?;

    rows.into_iter()
        .map(|row| {
            let (repo, rules_json, created_at, updated_at) = row?;
            let rules =
                serde_json::from_str::<RepoRuleSet>(&rules_json).context("failed to parse rule set")?;
            Ok(SavedRuleSet {
                repo,
                rules,
                created_at,
                updated_at,
            })
        })
        .collect()
}

pub fn get_rules(repo: &str) -> Result<Option<RepoRuleSet>> {
    let conn = connect()?;
    let rules_json: Option<String> = conn
        .query_row(
            "SELECT rules_json FROM rule_sets WHERE repo = ?1",
            params![repo],
            |row| row.get(0),
        )
        .optional()?;

    rules_json
        .map(|value| serde_json::from_str::<RepoRuleSet>(&value).context("failed to parse rule set"))
        .transpose()
}

pub fn delete_rules(repo: &str) -> Result<()> {
    let conn = connect()?;
    conn.execute("DELETE FROM rule_sets WHERE repo = ?1", params![repo])
        .context("failed to delete TrustGate rule set")?;
    Ok(())
}

pub fn save_review(review: &ReviewResult) -> Result<()> {
    let conn = connect()?;
    let review_json = serde_json::to_string(review).context("failed to serialize review")?;

    conn.execute(
        r#"
        INSERT INTO reviews (
          id, repo, created_at, ai_source, recommendation, risk_score, files_changed, summary, review_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            review.id,
            review.repo,
            review.created_at,
            review.ai_source,
            review.recommendation,
            i64::from(review.risk_score),
            i64::from(review.metrics.files_changed),
            review.summary,
            review_json
        ],
    )
    .context("failed to save TrustGate review")?;

    Ok(())
}

pub fn list_reviews() -> Result<Vec<ReviewHistoryItem>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT id, created_at, repo, ai_source, recommendation, risk_score, files_changed, summary, review_json
        FROM reviews
        ORDER BY created_at DESC
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)? as u32,
            row.get::<_, i64>(6)? as u32,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
        ))
    })?;

    rows.into_iter()
        .map(|row| {
            let (
                id,
                created_at,
                repo,
                ai_source,
                recommendation,
                risk_score,
                files_changed,
                summary,
                review_json,
            ) = row?;

            let parsed = serde_json::from_str::<ReviewResult>(&review_json).ok();
            let source_kind = parsed
                .as_ref()
                .map(|review| review.source_kind.clone())
                .unwrap_or_else(|| "manual".into());
            let pr_number = parsed
                .as_ref()
                .and_then(|review| review.github.as_ref().map(|github| github.pr_number));

            Ok::<ReviewHistoryItem, rusqlite::Error>(ReviewHistoryItem {
                id,
                created_at,
                repo,
                ai_source,
                recommendation,
                risk_score,
                files_changed,
                summary,
                source_kind,
                pr_number,
            })
        })
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to list TrustGate reviews")
}

pub fn get_review(id: &str) -> Result<Option<ReviewResult>> {
    let conn = connect()?;
    let review_json: Option<String> = conn
        .query_row(
            "SELECT review_json FROM reviews WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;

    review_json
        .map(|value| serde_json::from_str::<ReviewResult>(&value).context("failed to parse review"))
        .transpose()
}

pub fn review_count() -> usize {
    connect()
        .ok()
        .and_then(|conn| {
            conn.query_row("SELECT COUNT(*) FROM reviews", [], |row| row.get::<_, i64>(0))
                .ok()
        })
        .unwrap_or(0) as usize
}

pub fn rule_count() -> usize {
    connect()
        .ok()
        .and_then(|conn| {
            conn.query_row("SELECT COUNT(*) FROM rule_sets", [], |row| row.get::<_, i64>(0))
                .ok()
        })
        .unwrap_or(0) as usize
}

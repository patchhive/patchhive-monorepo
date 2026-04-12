use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{HistoryItem, OverviewCounts, OverviewPayload, ReviewResult};

pub fn db_path() -> String {
    std::env::var("REVIEW_BEE_DB_PATH").unwrap_or_else(|_| "review-bee.db".into())
}

fn connect() -> Result<Connection> {
    Connection::open(db_path()).context("Could not open ReviewBee database")
}

pub fn init_db() -> Result<()> {
    let conn = connect()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS review_runs (
          id TEXT PRIMARY KEY,
          repo TEXT NOT NULL,
          pr_number INTEGER NOT NULL,
          pr_title TEXT NOT NULL,
          pr_url TEXT NOT NULL,
          status TEXT NOT NULL,
          summary TEXT NOT NULL,
          action_items INTEGER NOT NULL,
          open_items INTEGER NOT NULL,
          resolved_items INTEGER NOT NULL,
          reviewer_count INTEGER NOT NULL,
          created_at TEXT NOT NULL,
          payload TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_review_runs_created_at
        ON review_runs(created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_review_runs_repo_created_at
        ON review_runs(repo, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub fn save_review(review: &ReviewResult) -> Result<()> {
    let conn = connect()?;
    let payload = serde_json::to_string(review).context("Could not encode review payload")?;
    conn.execute(
        r#"
        INSERT INTO review_runs (
          id, repo, pr_number, pr_title, pr_url, status, summary,
          action_items, open_items, resolved_items, reviewer_count, created_at, payload
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        params![
            review.id,
            review.repo,
            review.pr_number,
            review.pr_title,
            review.pr_url,
            review.status,
            review.summary,
            review.checklist.len() as i64,
            review.metrics.open_items,
            review.metrics.resolved_items,
            review.metrics.reviewer_count,
            review.created_at,
            payload,
        ],
    )
    .context("Could not persist ReviewBee run")?;
    Ok(())
}

pub fn history(limit: usize) -> Vec<HistoryItem> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };

    let mut stmt = match conn.prepare(
        r#"
        SELECT id, repo, pr_number, pr_title, status, summary,
               action_items, open_items, resolved_items, reviewer_count, created_at
        FROM review_runs
        ORDER BY created_at DESC
        LIMIT ?1
        "#,
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };

    stmt.query_map([limit as i64], |row| {
        Ok(HistoryItem {
            id: row.get(0)?,
            repo: row.get(1)?,
            pr_number: row.get(2)?,
            pr_title: row.get(3)?,
            status: row.get(4)?,
            summary: row.get(5)?,
            action_items: row.get::<_, i64>(6)? as u32,
            open_items: row.get::<_, i64>(7)? as u32,
            resolved_items: row.get::<_, i64>(8)? as u32,
            reviewer_count: row.get::<_, i64>(9)? as u32,
            created_at: row.get(10)?,
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

pub fn get_review(id: &str) -> Option<ReviewResult> {
    let conn = connect().ok()?;
    let payload = conn
        .query_row(
            "SELECT payload FROM review_runs WHERE id = ?1 LIMIT 1",
            [id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()?;
    serde_json::from_str(&payload).ok()
}

pub fn overview_counts() -> OverviewCounts {
    let Ok(conn) = connect() else {
        return OverviewCounts::default();
    };

    conn.query_row(
        r#"
        SELECT
          COUNT(*) AS reviews,
          COUNT(DISTINCT repo) AS repos,
          COALESCE(SUM(open_items), 0) AS open_items
        FROM review_runs
        "#,
        [],
        |row| {
            Ok(OverviewCounts {
                reviews: row.get::<_, i64>(0)? as u32,
                repos: row.get::<_, i64>(1)? as u32,
                open_items: row.get::<_, i64>(2)? as u32,
            })
        },
    )
    .unwrap_or_default()
}

pub fn overview() -> OverviewPayload {
    OverviewPayload {
        product: "ReviewBee by PatchHive".into(),
        tagline: "Close PR review threads faster by turning reviewer comments into concrete follow-up tasks.".into(),
        counts: overview_counts(),
        recent_reviews: history(6),
    }
}

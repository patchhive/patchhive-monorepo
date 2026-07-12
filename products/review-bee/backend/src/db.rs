use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use patchhive_product_core::sqlite::{PooledSqliteConnection, SqlitePool};
use rusqlite::{params, OptionalExtension};

use crate::models::{HistoryItem, OverviewCounts, OverviewPayload, ReviewResult};

static DB_POOL: Lazy<SqlitePool> = Lazy::new(|| {
    SqlitePool::new(db_path(), "ReviewBee").with_pool_size_env("REVIEW_BEE_DB_POOL_SIZE")
});

pub fn db_path() -> String {
    std::env::var("REVIEW_BEE_DB_PATH").unwrap_or_else(|_| "review-bee.db".into())
}

fn connect() -> Result<PooledSqliteConnection<'static>> {
    DB_POOL.get().context("Could not open ReviewBee database")
}

pub fn health_check() -> bool {
    connect()
        .and_then(|conn| {
            conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                .context("Could not query ReviewBee database")
        })
        .is_ok()
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
               action_items, open_items, resolved_items, reviewer_count, created_at, payload
        FROM review_runs
        ORDER BY created_at DESC
        LIMIT ?1
        "#,
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };

    stmt.query_map([limit as i64], |row| {
        let stored_status = row.get(4)?;
        let stored_summary = row.get(5)?;
        let stored_reviewer_count = row.get::<_, i64>(9)? as u32;
        let payload = row.get::<_, String>(11)?;
        let normalized = serde_json::from_str::<ReviewResult>(&payload)
            .ok()
            .map(|mut review| {
                crate::pipeline::normalize_review_result_reviewers(&mut review);
                review
            });
        Ok(HistoryItem {
            id: row.get(0)?,
            repo: row.get(1)?,
            pr_number: row.get(2)?,
            pr_title: row.get(3)?,
            status: normalized
                .as_ref()
                .map(|review| review.status.clone())
                .unwrap_or(stored_status),
            summary: normalized
                .as_ref()
                .map(|review| review.summary.clone())
                .unwrap_or(stored_summary),
            action_items: row.get::<_, i64>(6)? as u32,
            open_items: row.get::<_, i64>(7)? as u32,
            resolved_items: row.get::<_, i64>(8)? as u32,
            reviewer_count: normalized
                .as_ref()
                .map(|review| review.metrics.reviewer_count)
                .unwrap_or(stored_reviewer_count),
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
    let mut review = serde_json::from_str(&payload).ok()?;
    crate::pipeline::normalize_review_result_reviewers(&mut review);
    Some(review)
}

pub fn comment_publish_verified() -> bool {
    let Ok(conn) = connect() else {
        return false;
    };
    let mut stmt =
        match conn.prepare("SELECT payload FROM review_runs ORDER BY created_at DESC LIMIT 100") {
            Ok(stmt) => stmt,
            Err(_) => return false,
        };
    let rows = match stmt.query_map([], |row| row.get::<_, String>(0)) {
        Ok(rows) => rows,
        Err(_) => return false,
    };

    let verified = rows.flatten().any(|payload| {
        serde_json::from_str::<ReviewResult>(&payload)
            .ok()
            .and_then(|review| review.github_report)
            .is_some_and(|report| report.delivered && !report.comment_url.trim().is_empty())
    });
    verified
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

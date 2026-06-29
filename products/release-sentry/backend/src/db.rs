use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use patchhive_product_core::sqlite::{PooledSqliteConnection, SqlitePool};
use rusqlite::{params, OptionalExtension};

use crate::models::{HistoryItem, OverviewCounts, OverviewPayload, ReleaseReadinessResult};

static DB_POOL: Lazy<SqlitePool> = Lazy::new(|| {
    SqlitePool::new(db_path(), "release-sentry").with_pool_size_env("RELEASE_SENTRY_DB_POOL_SIZE")
});

pub fn db_path() -> String {
    std::env::var("RELEASE_SENTRY_DB_PATH").unwrap_or_else(|_| "release-sentry.db".into())
}

fn connect() -> Result<PooledSqliteConnection<'static>> {
    DB_POOL
        .get()
        .context("Could not open ReleaseSentry database")
}

pub fn health_check() -> bool {
    connect()
        .and_then(|conn| {
            conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                .context("Could not query ReleaseSentry database")
        })
        .is_ok()
}

pub fn init_db() -> Result<()> {
    let conn = connect()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS release_sentry_runs (
          id TEXT PRIMARY KEY,
          repo TEXT NOT NULL,
          branch TEXT NOT NULL,
          target_version TEXT NOT NULL,
          target_tag TEXT NOT NULL,
          decision TEXT NOT NULL,
          score INTEGER NOT NULL,
          summary TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          payload TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_release_sentry_runs_created_at
        ON release_sentry_runs(created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_release_sentry_runs_repo_created_at
        ON release_sentry_runs(repo, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub fn save_run(run: &ReleaseReadinessResult) -> Result<()> {
    let conn = connect()?;
    let payload = serde_json::to_string(run).context("Could not encode ReleaseSentry payload")?;
    conn.execute(
        r#"
        INSERT INTO release_sentry_runs (
          id, repo, branch, target_version, target_tag, decision,
          score, summary, created_at, updated_at, payload
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            run.id,
            run.repo,
            run.branch,
            run.target_version,
            run.target_tag,
            run.decision,
            run.score,
            run.summary,
            run.created_at,
            run.updated_at,
            payload,
        ],
    )
    .context("Could not persist ReleaseSentry run")?;
    Ok(())
}

pub fn history(limit: usize) -> Vec<HistoryItem> {
    let Ok(conn) = connect() else {
        return Vec::new();
    };

    let mut stmt = match conn.prepare(
        r#"
        SELECT id, repo, branch, target_version, target_tag, decision,
               score, summary, created_at, updated_at, payload
        FROM release_sentry_runs
        ORDER BY created_at DESC
        LIMIT ?1
        "#,
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };

    stmt.query_map([limit as i64], |row| {
        let decision: String = row.get(5)?;
        let summary: String = row.get(7)?;
        let payload: String = row.get(10)?;
        let saved_run = serde_json::from_str::<ReleaseReadinessResult>(&payload)
            .ok()
            .map(normalize_saved_run);
        let summary = saved_run
            .as_ref()
            .map(|run| run.summary.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| normalize_saved_text(&summary));
        let metrics = saved_run
            .as_ref()
            .map(|run| run.metrics.clone())
            .unwrap_or_default();
        Ok(HistoryItem {
            id: row.get(0)?,
            repo: row.get(1)?,
            branch: row.get(2)?,
            target_version: row.get(3)?,
            target_tag: row.get(4)?,
            status: decision.clone(),
            decision,
            score: row.get::<_, i64>(6)? as u32,
            summary,
            metrics,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

pub fn get_run(id: &str) -> Option<ReleaseReadinessResult> {
    let conn = connect().ok()?;
    let payload = conn
        .query_row(
            "SELECT payload FROM release_sentry_runs WHERE id = ?1 LIMIT 1",
            [id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()?;

    serde_json::from_str(&payload).ok().map(normalize_saved_run)
}

pub fn overview_counts() -> OverviewCounts {
    let Ok(conn) = connect() else {
        return OverviewCounts::default();
    };

    conn.query_row(
        r#"
        SELECT
          COUNT(*) AS runs,
          COUNT(DISTINCT repo) AS repos,
          COALESCE(SUM(CASE WHEN decision = 'ready' THEN 1 ELSE 0 END), 0) AS ready,
          COALESCE(SUM(CASE WHEN decision = 'watch' THEN 1 ELSE 0 END), 0) AS watch,
          COALESCE(SUM(CASE WHEN decision = 'hold' THEN 1 ELSE 0 END), 0) AS hold
        FROM release_sentry_runs
        "#,
        [],
        |row| {
            Ok(OverviewCounts {
                runs: row.get::<_, i64>(0)? as u32,
                repos: row.get::<_, i64>(1)? as u32,
                ready: row.get::<_, i64>(2)? as u32,
                watch: row.get::<_, i64>(3)? as u32,
                hold: row.get::<_, i64>(4)? as u32,
            })
        },
    )
    .unwrap_or_default()
}

pub fn overview() -> OverviewPayload {
    OverviewPayload {
        product: "ReleaseSentry by PatchHive".into(),
        tagline: "Check release readiness with CI, tags, changelog, blocker, and release evidence."
            .into(),
        counts: overview_counts(),
        recent_runs: history(6),
    }
}

fn normalize_saved_run(mut run: ReleaseReadinessResult) -> ReleaseReadinessResult {
    run.summary = normalize_saved_text(&run.summary);
    for check in &mut run.checks {
        check.detail = normalize_saved_text(&check.detail);
        for evidence in &mut check.evidence {
            *evidence = normalize_saved_text(evidence);
        }
    }
    for warning in &mut run.warnings {
        *warning = normalize_saved_text(warning);
    }
    run
}

fn normalize_saved_text(value: &str) -> String {
    value
        .replace("successs", "successful runs")
        .replace(
            "blocking checks need attention",
            "blocking gate checks need attention",
        )
        .replace("failing, ", "failed runs, ")
        .replace(" pending across ", " pending runs across ")
        .replace(" recent runs on ", " recent workflow runs on ")
}

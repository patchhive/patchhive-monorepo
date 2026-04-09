use rusqlite::{params, params_from_iter, Connection, OptionalExtension};

use crate::models::{HistoryItem, IngestRecord, KnownRepo, MemoryEntry, OverviewCounts};

pub fn db_path() -> String {
    std::env::var("REPO_MEMORY_DB_PATH").unwrap_or_else(|_| "repo-memory.db".into())
}

fn connect() -> rusqlite::Result<Connection> {
    Connection::open(db_path())
}

pub fn init_db() -> rusqlite::Result<()> {
    let conn = connect()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS product_meta (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS memory_runs (
          id TEXT PRIMARY KEY,
          repo TEXT NOT NULL,
          created_at TEXT NOT NULL,
          params_json TEXT NOT NULL,
          summary_json TEXT NOT NULL,
          prompt_pack TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS memory_entries (
          id TEXT PRIMARY KEY,
          run_id TEXT NOT NULL,
          repo TEXT NOT NULL,
          kind TEXT NOT NULL,
          title TEXT NOT NULL,
          detail TEXT NOT NULL,
          prompt_line TEXT NOT NULL,
          confidence REAL NOT NULL,
          frequency INTEGER NOT NULL,
          tags_json TEXT NOT NULL,
          evidence_json TEXT NOT NULL,
          created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_memory_runs_repo_created
          ON memory_runs (repo, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_memory_entries_repo_kind_confidence
          ON memory_entries (repo, kind, confidence DESC, created_at DESC);
        "#,
    )?;
    conn.execute(
        r#"
        INSERT INTO product_meta (key, value)
        VALUES ('product', 'RepoMemory')
        ON CONFLICT(key) DO NOTHING
        "#,
        [],
    )?;
    Ok(())
}

pub fn save_run(run: &IngestRecord) -> rusqlite::Result<()> {
    let conn = connect()?;
    let tx = conn.unchecked_transaction()?;

    tx.execute(
        r#"
        INSERT INTO memory_runs (id, repo, created_at, params_json, summary_json, prompt_pack)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            run.id,
            run.repo,
            run.created_at,
            serde_json::to_string(&run.params).unwrap_or_default(),
            serde_json::to_string(&run.summary).unwrap_or_default(),
            run.prompt_pack,
        ],
    )?;

    for entry in &run.entries {
        tx.execute(
            r#"
            INSERT INTO memory_entries (
              id, run_id, repo, kind, title, detail, prompt_line, confidence,
              frequency, tags_json, evidence_json, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                entry.id,
                entry.run_id,
                entry.repo,
                entry.kind,
                entry.title,
                entry.detail,
                entry.prompt_line,
                entry.confidence,
                entry.frequency,
                serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".into()),
                serde_json::to_string(&entry.evidence).unwrap_or_else(|_| "[]".into()),
                entry.created_at,
            ],
        )?;
    }

    tx.commit()
}

pub fn run_count() -> u32 {
    scalar_count("SELECT COUNT(*) FROM memory_runs")
}

pub fn memory_count() -> u32 {
    scalar_count("SELECT COUNT(*) FROM memory_entries")
}

pub fn repo_count() -> u32 {
    connect()
        .ok()
        .and_then(|conn| {
            conn.query_row(
                "SELECT COUNT(DISTINCT repo) FROM memory_runs",
                [],
                |row| row.get::<_, i64>(0),
            )
            .ok()
        })
        .unwrap_or(0) as u32
}

fn scalar_count(sql: &str) -> u32 {
    connect()
        .ok()
        .and_then(|conn| conn.query_row(sql, [], |row| row.get::<_, i64>(0)).ok())
        .unwrap_or(0) as u32
}

pub fn overview_counts() -> OverviewCounts {
    OverviewCounts {
        repos: repo_count(),
        runs: run_count(),
        memories: memory_count(),
    }
}

pub fn list_known_repos() -> rusqlite::Result<Vec<KnownRepo>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT
          mr.repo,
          MAX(mr.created_at) AS last_ingested_at,
          COUNT(DISTINCT mr.id) AS run_count,
          COUNT(me.id) AS memory_count,
          COALESCE((
            SELECT title
            FROM memory_entries latest_me
            JOIN memory_runs latest_mr ON latest_mr.id = latest_me.run_id
            WHERE latest_mr.repo = mr.repo
            ORDER BY latest_me.confidence DESC, latest_me.created_at DESC
            LIMIT 1
          ), '')
        FROM memory_runs mr
        LEFT JOIN memory_entries me ON me.run_id = mr.id
        GROUP BY mr.repo
        ORDER BY last_ingested_at DESC
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(KnownRepo {
            repo: row.get(0)?,
            last_ingested_at: row.get(1)?,
            run_count: row.get::<_, i64>(2)? as u32,
            memory_count: row.get::<_, i64>(3)? as u32,
            top_memory: row.get(4)?,
        })
    })?;

    rows.collect()
}

pub fn featured_memories(limit: usize) -> rusqlite::Result<Vec<MemoryEntry>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT
          id, run_id, repo, kind, title, detail, prompt_line, confidence,
          frequency, tags_json, evidence_json, created_at
        FROM memory_entries
        ORDER BY confidence DESC, created_at DESC
        LIMIT ?1
        "#,
    )?;

    let rows = stmt.query_map(params![limit as i64], decode_memory_entry)?;
    rows.collect()
}

pub fn list_history(repo: Option<&str>) -> rusqlite::Result<Vec<HistoryItem>> {
    let conn = connect()?;
    let mut sql = String::from(
        "SELECT id, repo, created_at, summary_json FROM memory_runs",
    );
    let mut params = Vec::new();

    if let Some(repo) = repo.filter(|value| !value.trim().is_empty()) {
        sql.push_str(" WHERE repo = ?1");
        params.push(repo.to_string());
    }

    sql.push_str(" ORDER BY created_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), |row| {
        let summary_json: String = row.get(3)?;
        let summary = serde_json::from_str::<crate::models::IngestSummary>(&summary_json)
            .unwrap_or_else(|_| crate::models::IngestSummary::empty());
        Ok(HistoryItem {
            id: row.get(0)?,
            repo: row.get(1)?,
            created_at: row.get(2)?,
            memories_created: summary.memories_created,
            conventions: summary.conventions,
            failures: summary.failures,
            hotspots: summary.hotspots,
            top_memory: summary.top_memory,
        })
    })?;

    rows.collect()
}

pub fn get_history(id: &str) -> rusqlite::Result<Option<IngestRecord>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT repo, created_at, params_json, summary_json, prompt_pack
        FROM memory_runs
        WHERE id = ?1
        "#,
    )?;

    let run = stmt
        .query_row(params![id], |row| {
            let params_json: String = row.get(2)?;
            let summary_json: String = row.get(3)?;
            Ok(IngestRecord {
                id: id.to_string(),
                repo: row.get(0)?,
                created_at: row.get(1)?,
                params: serde_json::from_str(&params_json).unwrap_or_default(),
                summary: serde_json::from_str(&summary_json)
                    .unwrap_or_else(|_| crate::models::IngestSummary::empty()),
                prompt_pack: row.get(4)?,
                entries: Vec::new(),
            })
        })
        .optional()?;

    let Some(mut run) = run else {
        return Ok(None);
    };

    run.entries = list_memories(Some(&run.repo), None, None, Some(id))?;
    Ok(Some(run))
}

pub fn list_memories(
    repo: Option<&str>,
    kind: Option<&str>,
    search: Option<&str>,
    run_id: Option<&str>,
) -> rusqlite::Result<Vec<MemoryEntry>> {
    let conn = connect()?;
    let mut sql = String::from(
        r#"
        SELECT
          id, run_id, repo, kind, title, detail, prompt_line, confidence,
          frequency, tags_json, evidence_json, created_at
        FROM memory_entries
        WHERE 1=1
        "#,
    );
    let mut params = Vec::new();

    if let Some(run_id) = run_id.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(" AND run_id = ?{}", params.len() + 1));
        params.push(run_id.to_string());
    } else if let Some(repo) = repo.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(
            " AND run_id = (SELECT id FROM memory_runs WHERE repo = ?{} ORDER BY created_at DESC LIMIT 1)",
            params.len() + 1
        ));
        params.push(repo.to_string());
    }

    if let Some(kind) = kind.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(" AND kind = ?{}", params.len() + 1));
        params.push(kind.to_string());
    }

    if let Some(search) = search.filter(|value| !value.trim().is_empty()) {
        let slot = params.len() + 1;
        sql.push_str(&format!(
            " AND (title LIKE ?{slot} OR detail LIKE ?{slot} OR prompt_line LIKE ?{slot} OR tags_json LIKE ?{slot})"
        ));
        params.push(format!("%{}%", search.trim()));
    }

    sql.push_str(" ORDER BY confidence DESC, frequency DESC, created_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), decode_memory_entry)?;
    rows.collect()
}

fn decode_memory_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryEntry> {
    let tags_json: String = row.get(9)?;
    let evidence_json: String = row.get(10)?;
    Ok(MemoryEntry {
        id: row.get(0)?,
        run_id: row.get(1)?,
        repo: row.get(2)?,
        kind: row.get(3)?,
        title: row.get(4)?,
        detail: row.get(5)?,
        prompt_line: row.get(6)?,
        confidence: row.get(7)?,
        frequency: row.get::<_, i64>(8)? as u32,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        evidence: serde_json::from_str(&evidence_json).unwrap_or_default(),
        created_at: row.get(11)?,
    })
}

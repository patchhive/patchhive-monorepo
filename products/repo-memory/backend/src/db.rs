use rusqlite::{params, params_from_iter, Connection, OptionalExtension};

use crate::models::{
    stable_memory_ref, HistoryItem, IngestRecord, KnownRepo, MemoryEntry, OverviewCounts,
};

pub fn db_path() -> String {
    std::env::var("REPO_MEMORY_DB_PATH").unwrap_or_else(|_| "repo-memory.db".into())
}

fn connect() -> rusqlite::Result<Connection> {
    Connection::open(db_path())
}

pub fn health_check() -> bool {
    connect()
        .and_then(|conn| conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0)))
        .is_ok()
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
          memory_ref TEXT NOT NULL DEFAULT '',
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

        CREATE TABLE IF NOT EXISTS memory_curations (
          repo TEXT NOT NULL,
          memory_ref TEXT NOT NULL,
          disposition TEXT NOT NULL DEFAULT 'signal',
          pinned INTEGER NOT NULL DEFAULT 0,
          updated_at TEXT NOT NULL,
          PRIMARY KEY (repo, memory_ref)
        );

        CREATE INDEX IF NOT EXISTS idx_memory_runs_repo_created
          ON memory_runs (repo, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_memory_entries_repo_kind_confidence
          ON memory_entries (repo, kind, confidence DESC, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_memory_entries_repo_ref
          ON memory_entries (repo, memory_ref);
        "#,
    )?;
    ensure_column(&conn, "memory_entries", "memory_ref", "TEXT NOT NULL DEFAULT ''")?;
    backfill_memory_refs(&conn)?;
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
              id, memory_ref, run_id, repo, kind, title, detail, prompt_line,
              confidence, frequency, tags_json, evidence_json, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                entry.id,
                entry.memory_ref,
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
          me.id, me.memory_ref, me.run_id, me.repo, me.kind, me.title, me.detail,
          me.prompt_line, me.confidence, me.frequency,
          COALESCE(mc.disposition, 'signal'),
          COALESCE(mc.pinned, 0),
          me.tags_json, me.evidence_json, me.created_at
        FROM memory_entries me
        LEFT JOIN memory_curations mc
          ON mc.repo = me.repo AND mc.memory_ref = me.memory_ref
        WHERE COALESCE(mc.disposition, 'signal') != 'suppressed'
        ORDER BY COALESCE(mc.pinned, 0) DESC,
                 CASE COALESCE(mc.disposition, 'signal') WHEN 'policy' THEN 0 ELSE 1 END,
                 me.confidence DESC,
                 me.created_at DESC
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
          me.id, me.memory_ref, me.run_id, me.repo, me.kind, me.title, me.detail,
          me.prompt_line, me.confidence, me.frequency,
          COALESCE(mc.disposition, 'signal'),
          COALESCE(mc.pinned, 0),
          me.tags_json, me.evidence_json, me.created_at
        FROM memory_entries me
        LEFT JOIN memory_curations mc
          ON mc.repo = me.repo AND mc.memory_ref = me.memory_ref
        WHERE 1=1
        "#,
    );
    let mut params = Vec::new();

    if let Some(run_id) = run_id.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(" AND me.run_id = ?{}", params.len() + 1));
        params.push(run_id.to_string());
    } else if let Some(repo) = repo.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(
            " AND me.run_id = (SELECT id FROM memory_runs WHERE repo = ?{} ORDER BY created_at DESC LIMIT 1)",
            params.len() + 1
        ));
        params.push(repo.to_string());
    }

    if let Some(kind) = kind.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(" AND me.kind = ?{}", params.len() + 1));
        params.push(kind.to_string());
    }

    if let Some(search) = search.filter(|value| !value.trim().is_empty()) {
        let slot = params.len() + 1;
        sql.push_str(&format!(
            " AND (title LIKE ?{slot} OR detail LIKE ?{slot} OR prompt_line LIKE ?{slot} OR tags_json LIKE ?{slot})"
        ));
        params.push(format!("%{}%", search.trim()));
    }

    sql.push_str(
        " ORDER BY CASE COALESCE(mc.disposition, 'signal')
                    WHEN 'policy' THEN 0
                    WHEN 'signal' THEN 1
                    ELSE 2
                  END,
                 COALESCE(mc.pinned, 0) DESC,
                 me.confidence DESC,
                 me.frequency DESC,
                 me.created_at DESC",
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), decode_memory_entry)?;
    rows.collect()
}

pub fn save_memory_curation(
    repo: &str,
    memory_ref: &str,
    disposition: &str,
    pinned: bool,
) -> rusqlite::Result<()> {
    let conn = connect()?;

    if disposition == "signal" && !pinned {
        conn.execute(
            "DELETE FROM memory_curations WHERE repo = ?1 AND memory_ref = ?2",
            params![repo, memory_ref],
        )?;
        return Ok(());
    }

    conn.execute(
        r#"
        INSERT INTO memory_curations (repo, memory_ref, disposition, pinned, updated_at)
        VALUES (?1, ?2, ?3, ?4, datetime('now'))
        ON CONFLICT(repo, memory_ref) DO UPDATE SET
          disposition = excluded.disposition,
          pinned = excluded.pinned,
          updated_at = excluded.updated_at
        "#,
        params![repo, memory_ref, disposition, if pinned { 1 } else { 0 }],
    )?;
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> rusqlite::Result<()> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut exists = false;
    for row in rows {
        if row?.eq_ignore_ascii_case(column) {
            exists = true;
            break;
        }
    }

    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }

    Ok(())
}

fn backfill_memory_refs(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, repo, kind, title
        FROM memory_entries
        WHERE memory_ref = '' OR memory_ref IS NULL
        "#,
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    for row in rows {
        let (id, repo, kind, title) = row?;
        let memory_ref = stable_memory_ref(&repo, &kind, &title);
        conn.execute(
            "UPDATE memory_entries SET memory_ref = ?1 WHERE id = ?2",
            params![memory_ref, id],
        )?;
    }

    Ok(())
}

fn decode_memory_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryEntry> {
    let tags_json: String = row.get(12)?;
    let evidence_json: String = row.get(13)?;
    let repo: String = row.get(3)?;
    let kind: String = row.get(4)?;
    let title: String = row.get(5)?;
    let memory_ref = {
        let value: String = row.get(1)?;
        if value.trim().is_empty() {
            stable_memory_ref(&repo, &kind, &title)
        } else {
            value
        }
    };
    Ok(MemoryEntry {
        id: row.get(0)?,
        memory_ref,
        run_id: row.get(2)?,
        repo,
        kind,
        title,
        detail: row.get(6)?,
        prompt_line: row.get(7)?,
        confidence: row.get(8)?,
        frequency: row.get::<_, i64>(9)? as u32,
        disposition: row.get(10)?,
        pinned: row.get::<_, i64>(11)? != 0,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        evidence: serde_json::from_str(&evidence_json).unwrap_or_default(),
        created_at: row.get(14)?,
    })
}

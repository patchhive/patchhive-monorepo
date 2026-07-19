use once_cell::sync::Lazy;
use patchhive_product_core::sqlite::{product_db_path, PooledSqliteConnection, SqlitePool};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};

use crate::models::{
    stable_memory_ref, FailGuardCandidate, FailGuardGuardrail, FailGuardMatchRecord, HistoryItem,
    IngestRecord, KnownRepo, MemoryEntry, OverviewCounts,
};

static DB_POOL: Lazy<SqlitePool> = Lazy::new(|| {
    SqlitePool::new(db_path(), "RepoMemory").with_pool_size_env("REPO_MEMORY_DB_POOL_SIZE")
});

pub fn db_path() -> String {
    product_db_path("REPO_MEMORY_DB_PATH", "repo-memory.db")
}

fn connect() -> rusqlite::Result<PooledSqliteConnection<'static>> {
    DB_POOL.get()
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

        CREATE TABLE IF NOT EXISTS failguard_candidates (
          id TEXT PRIMARY KEY,
          repo TEXT NOT NULL,
          source_type TEXT NOT NULL,
          source_ref TEXT NOT NULL DEFAULT '',
          title TEXT NOT NULL,
          outcome TEXT NOT NULL,
          lesson TEXT NOT NULL,
          prevention TEXT NOT NULL,
          affected_paths_json TEXT NOT NULL,
          evidence_json TEXT NOT NULL,
          confidence REAL NOT NULL,
          correlation_key TEXT NOT NULL DEFAULT '',
          occurrence_count INTEGER NOT NULL DEFAULT 1,
          status TEXT NOT NULL DEFAULT 'open',
          memory_ref TEXT NOT NULL DEFAULT '',
          resolution_note TEXT NOT NULL DEFAULT '',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          last_seen_at TEXT NOT NULL DEFAULT '',
          recurrence_of TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS failguard_guardrails (
          id TEXT PRIMARY KEY,
          repo TEXT NOT NULL,
          candidate_id TEXT NOT NULL DEFAULT '',
          memory_ref TEXT NOT NULL,
          title TEXT NOT NULL,
          prevention TEXT NOT NULL,
          affected_paths_json TEXT NOT NULL,
          suggestions_json TEXT NOT NULL,
          status TEXT NOT NULL DEFAULT 'active',
          match_count INTEGER NOT NULL DEFAULT 0,
          last_matched_at TEXT NOT NULL DEFAULT '',
          recurrence_count INTEGER NOT NULL DEFAULT 0,
          last_recurred_at TEXT NOT NULL DEFAULT '',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS failguard_matches (
          id TEXT PRIMARY KEY,
          guardrail_id TEXT NOT NULL,
          repo TEXT NOT NULL,
          consumer TEXT NOT NULL,
          context_ref TEXT NOT NULL DEFAULT '',
          matched_paths_json TEXT NOT NULL,
          matched_terms_json TEXT NOT NULL,
          created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_memory_runs_repo_created
          ON memory_runs (repo, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_memory_entries_repo_kind_confidence
          ON memory_entries (repo, kind, confidence DESC, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_memory_entries_repo_ref
          ON memory_entries (repo, memory_ref);
        CREATE INDEX IF NOT EXISTS idx_failguard_candidates_repo_status
          ON failguard_candidates (repo, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_failguard_guardrails_repo_status
          ON failguard_guardrails (repo, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_failguard_matches_guardrail_created
          ON failguard_matches (guardrail_id, created_at DESC);
        "#,
    )?;
    ensure_column(
        &conn,
        "memory_entries",
        "memory_ref",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "failguard_candidates",
        "recurrence_of",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "failguard_guardrails",
        "recurrence_count",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        &conn,
        "failguard_guardrails",
        "last_recurred_at",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "failguard_candidates",
        "correlation_key",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "failguard_candidates",
        "occurrence_count",
        "INTEGER NOT NULL DEFAULT 1",
    )?;
    ensure_column(
        &conn,
        "failguard_candidates",
        "last_seen_at",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    conn.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_failguard_candidates_correlation
        ON failguard_candidates (repo, correlation_key, status)
        "#,
        [],
    )?;
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
    scalar_count(
        r#"
        WITH latest_runs AS (
          SELECT mr.id, mr.repo
          FROM memory_runs mr
          WHERE mr.id = (
            SELECT latest.id
            FROM memory_runs latest
            WHERE latest.repo = mr.repo
            ORDER BY latest.created_at DESC, latest.rowid DESC
            LIMIT 1
          )
        )
        SELECT COUNT(*)
        FROM memory_entries me
        JOIN latest_runs lr ON lr.id = me.run_id
        LEFT JOIN memory_curations mc
          ON mc.repo = me.repo AND mc.memory_ref = me.memory_ref
        WHERE COALESCE(mc.disposition, 'signal') != 'suppressed'
        "#,
    )
}

pub fn repo_count() -> u32 {
    connect()
        .ok()
        .and_then(|conn| {
            conn.query_row("SELECT COUNT(DISTINCT repo) FROM memory_runs", [], |row| {
                row.get::<_, i64>(0)
            })
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
          (
            SELECT COUNT(*)
            FROM memory_entries latest_me
            LEFT JOIN memory_curations latest_mc
              ON latest_mc.repo = latest_me.repo
             AND latest_mc.memory_ref = latest_me.memory_ref
            WHERE latest_me.run_id = (
              SELECT latest.id
              FROM memory_runs latest
              WHERE latest.repo = mr.repo
              ORDER BY latest.created_at DESC, latest.rowid DESC
              LIMIT 1
            )
              AND COALESCE(latest_mc.disposition, 'signal') != 'suppressed'
          ) AS memory_count,
          COALESCE((
            SELECT title
            FROM memory_entries latest_me
            LEFT JOIN memory_curations latest_mc
              ON latest_mc.repo = latest_me.repo
             AND latest_mc.memory_ref = latest_me.memory_ref
            WHERE latest_me.run_id = (
              SELECT latest.id
              FROM memory_runs latest
              WHERE latest.repo = mr.repo
              ORDER BY latest.created_at DESC, latest.rowid DESC
              LIMIT 1
            )
              AND COALESCE(latest_mc.disposition, 'signal') != 'suppressed'
            ORDER BY latest_me.confidence DESC, latest_me.created_at DESC
            LIMIT 1
          ), '')
        FROM memory_runs mr
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
        WITH latest_runs AS (
          SELECT mr.id, mr.repo
          FROM memory_runs mr
          WHERE mr.id = (
            SELECT latest.id
            FROM memory_runs latest
            WHERE latest.repo = mr.repo
            ORDER BY latest.created_at DESC, latest.rowid DESC
            LIMIT 1
          )
        )
        SELECT
          me.id, me.memory_ref, me.run_id, me.repo, me.kind, me.title, me.detail,
          me.prompt_line, me.confidence, me.frequency,
          COALESCE(mc.disposition, 'signal'),
          COALESCE(mc.pinned, 0),
          me.tags_json, me.evidence_json, me.created_at
        FROM memory_entries me
        JOIN latest_runs lr ON lr.id = me.run_id
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
    let mut sql = String::from("SELECT id, repo, created_at, summary_json FROM memory_runs");
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
            partial_read_warnings: summary.partial_read_warnings,
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
    list_memories_with_connection(&conn, repo, kind, search, run_id)
}

pub fn apply_memory_curations(entries: &mut [MemoryEntry]) -> rusqlite::Result<()> {
    let conn = connect()?;
    apply_memory_curations_with_connection(&conn, entries)
}

fn apply_memory_curations_with_connection(
    conn: &Connection,
    entries: &mut [MemoryEntry],
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        r#"
        SELECT disposition, pinned
        FROM memory_curations
        WHERE repo = ?1 AND memory_ref = ?2
        "#,
    )?;

    for entry in entries {
        if let Some((disposition, pinned)) = stmt
            .query_row(params![&entry.repo, &entry.memory_ref], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
            })
            .optional()?
        {
            entry.disposition = disposition;
            entry.pinned = pinned;
        }
    }

    Ok(())
}

fn list_memories_with_connection(
    conn: &Connection,
    repo: Option<&str>,
    kind: Option<&str>,
    search: Option<&str>,
    run_id: Option<&str>,
) -> rusqlite::Result<Vec<MemoryEntry>> {
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
    } else {
        sql.push_str(
            r#"
            AND me.id = (
              SELECT latest_me.id
              FROM memory_entries latest_me
              JOIN memory_runs latest_run ON latest_run.id = latest_me.run_id
              WHERE latest_me.repo = me.repo
                AND latest_me.memory_ref = me.memory_ref
              ORDER BY
                latest_run.created_at DESC,
                latest_run.rowid DESC,
                latest_me.created_at DESC,
                latest_me.rowid DESC
              LIMIT 1
            )
            AND (
              me.run_id = (
                SELECT latest_repo_run.id
                FROM memory_runs latest_repo_run
                WHERE latest_repo_run.repo = me.repo
                ORDER BY latest_repo_run.created_at DESC, latest_repo_run.rowid DESC
                LIMIT 1
              )
              OR (
                COALESCE(mc.disposition, 'signal') = 'policy'
                AND COALESCE(mc.pinned, 0) = 1
              )
            )
            "#,
        );

        if let Some(repo) = repo.filter(|value| !value.trim().is_empty()) {
            sql.push_str(&format!(" AND me.repo = ?{}", params.len() + 1));
            params.push(repo.to_string());
        }
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

pub fn save_failguard_candidate(candidate: &FailGuardCandidate) -> rusqlite::Result<()> {
    let conn = connect()?;
    conn.execute(
        r#"
        INSERT INTO failguard_candidates (
          id, repo, source_type, source_ref, title, outcome, lesson, prevention,
          affected_paths_json, evidence_json, confidence, correlation_key, occurrence_count,
          status, memory_ref, resolution_note, created_at, updated_at, last_seen_at,
          recurrence_of
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
        "#,
        params![
            candidate.id,
            candidate.repo,
            candidate.source_type,
            candidate.source_ref,
            candidate.title,
            candidate.outcome,
            candidate.lesson,
            candidate.prevention,
            serde_json::to_string(&candidate.affected_paths).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&candidate.evidence).unwrap_or_else(|_| "[]".into()),
            candidate.confidence,
            candidate.correlation_key,
            candidate.occurrence_count,
            candidate.status,
            candidate.memory_ref,
            candidate.resolution_note,
            candidate.created_at,
            candidate.updated_at,
            candidate.last_seen_at,
            candidate.recurrence_of,
        ],
    )?;
    Ok(())
}

pub fn find_open_failguard_candidate(
    repo: &str,
    correlation_key: &str,
) -> rusqlite::Result<Option<FailGuardCandidate>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT
          id, repo, source_type, source_ref, title, outcome, lesson, prevention,
          affected_paths_json, evidence_json, confidence, correlation_key, occurrence_count,
          status, memory_ref, resolution_note, created_at, updated_at, last_seen_at,
          recurrence_of
        FROM failguard_candidates
        WHERE repo = ?1 AND correlation_key = ?2 AND status = 'open'
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )?;
    stmt.query_row(params![repo, correlation_key], decode_failguard_candidate)
        .optional()
}

pub fn update_failguard_candidate(candidate: &FailGuardCandidate) -> rusqlite::Result<()> {
    let conn = connect()?;
    conn.execute(
        r#"
        UPDATE failguard_candidates
        SET source_ref = ?2,
            outcome = ?3,
            lesson = ?4,
            prevention = ?5,
            affected_paths_json = ?6,
            evidence_json = ?7,
            confidence = ?8,
            occurrence_count = ?9,
            updated_at = ?10,
            last_seen_at = ?11,
            correlation_key = ?12,
            recurrence_of = ?13
        WHERE id = ?1
        "#,
        params![
            candidate.id,
            candidate.source_ref,
            candidate.outcome,
            candidate.lesson,
            candidate.prevention,
            serde_json::to_string(&candidate.affected_paths).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&candidate.evidence).unwrap_or_else(|_| "[]".into()),
            candidate.confidence,
            candidate.occurrence_count,
            candidate.updated_at,
            candidate.last_seen_at,
            candidate.correlation_key,
            candidate.recurrence_of,
        ],
    )?;
    Ok(())
}

pub fn save_failguard_guardrail(guardrail: &FailGuardGuardrail) -> rusqlite::Result<()> {
    let conn = connect()?;
    conn.execute(
        r#"
        INSERT INTO failguard_guardrails (
          id, repo, candidate_id, memory_ref, title, prevention, affected_paths_json,
          suggestions_json, status, match_count, last_matched_at, recurrence_count,
          last_recurred_at, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        ON CONFLICT(id) DO UPDATE SET
          candidate_id = CASE
            WHEN excluded.candidate_id = '' THEN failguard_guardrails.candidate_id
            ELSE excluded.candidate_id
          END,
          title = excluded.title,
          prevention = excluded.prevention,
          affected_paths_json = excluded.affected_paths_json,
          suggestions_json = excluded.suggestions_json,
          status = excluded.status,
          updated_at = excluded.updated_at
        "#,
        params![
            guardrail.id,
            guardrail.repo,
            guardrail.candidate_id,
            guardrail.memory_ref,
            guardrail.title,
            guardrail.prevention,
            serde_json::to_string(&guardrail.affected_paths).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&guardrail.suggestions).unwrap_or_else(|_| "[]".into()),
            guardrail.status,
            guardrail.match_count,
            guardrail.last_matched_at,
            guardrail.recurrence_count,
            guardrail.last_recurred_at,
            guardrail.created_at,
            guardrail.updated_at,
        ],
    )?;
    Ok(())
}

pub fn list_failguard_guardrails(
    repo: Option<&str>,
    status: Option<&str>,
) -> rusqlite::Result<Vec<FailGuardGuardrail>> {
    let conn = connect()?;
    let mut sql = String::from(
        r#"
        SELECT id, repo, candidate_id, memory_ref, title, prevention,
               affected_paths_json, suggestions_json, status, match_count,
               last_matched_at, recurrence_count, last_recurred_at, created_at, updated_at
        FROM failguard_guardrails
        WHERE 1=1
        "#,
    );
    let mut values = Vec::new();
    if let Some(repo) = repo.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(" AND repo = ?{}", values.len() + 1));
        values.push(repo.to_string());
    }
    if let Some(status) = status.filter(|value| !value.trim().is_empty() && *value != "all") {
        sql.push_str(&format!(" AND status = ?{}", values.len() + 1));
        values.push(status.to_string());
    }
    sql.push_str(" ORDER BY updated_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(values.iter()), decode_failguard_guardrail)?;
    rows.collect()
}

pub fn find_promoted_failguard_guardrail(
    repo: &str,
    correlation_key: &str,
) -> rusqlite::Result<Option<String>> {
    let conn = connect()?;
    conn.query_row(
        r#"
        SELECT g.id
        FROM failguard_candidates c
        JOIN failguard_guardrails g ON g.candidate_id = c.id
        WHERE c.repo = ?1
          AND c.correlation_key = ?2
          AND c.status = 'promoted'
          AND g.status = 'active'
        ORDER BY c.updated_at DESC
        LIMIT 1
        "#,
        params![repo, correlation_key],
        |row| row.get(0),
    )
    .optional()
}

pub fn record_failguard_recurrence(guardrail_id: &str, occurred_at: &str) -> rusqlite::Result<()> {
    let conn = connect()?;
    conn.execute(
        r#"
        UPDATE failguard_guardrails
        SET recurrence_count = recurrence_count + 1,
            last_recurred_at = ?2,
            updated_at = ?2
        WHERE id = ?1
        "#,
        params![guardrail_id, occurred_at],
    )?;
    Ok(())
}

pub fn record_failguard_match(record: &FailGuardMatchRecord) -> rusqlite::Result<()> {
    let mut conn = connect()?;
    let tx = conn.transaction()?;
    tx.execute(
        r#"
        INSERT INTO failguard_matches (
          id, guardrail_id, repo, consumer, context_ref, matched_paths_json,
          matched_terms_json, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            record.id,
            record.guardrail_id,
            record.repo,
            record.consumer,
            record.context_ref,
            serde_json::to_string(&record.matched_paths).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&record.matched_terms).unwrap_or_else(|_| "[]".into()),
            record.created_at,
        ],
    )?;
    tx.execute(
        r#"
        UPDATE failguard_guardrails
        SET match_count = match_count + 1,
            last_matched_at = ?2,
            updated_at = ?2
        WHERE id = ?1
        "#,
        params![record.guardrail_id, record.created_at],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn list_failguard_matches(
    repo: Option<&str>,
    consumer: Option<&str>,
    limit: u32,
) -> rusqlite::Result<Vec<FailGuardMatchRecord>> {
    let conn = connect()?;
    let mut sql = String::from(
        r#"
        SELECT id, guardrail_id, repo, consumer, context_ref, matched_paths_json,
               matched_terms_json, created_at
        FROM failguard_matches
        WHERE 1=1
        "#,
    );
    let mut values = Vec::new();
    if let Some(repo) = repo.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(" AND repo = ?{}", values.len() + 1));
        values.push(repo.to_string());
    }
    if let Some(consumer) = consumer.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(" AND consumer = ?{}", values.len() + 1));
        values.push(consumer.to_string());
    }
    sql.push_str(&format!(
        " ORDER BY created_at DESC LIMIT {}",
        limit.clamp(1, 200)
    ));
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(values.iter()), |row| {
        let matched_paths_json: String = row.get(5)?;
        let matched_terms_json: String = row.get(6)?;
        Ok(FailGuardMatchRecord {
            id: row.get(0)?,
            guardrail_id: row.get(1)?,
            repo: row.get(2)?,
            consumer: row.get(3)?,
            context_ref: row.get(4)?,
            matched_paths: serde_json::from_str(&matched_paths_json).unwrap_or_default(),
            matched_terms: serde_json::from_str(&matched_terms_json).unwrap_or_default(),
            created_at: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn list_failguard_candidates(
    repo: Option<&str>,
    status: Option<&str>,
) -> rusqlite::Result<Vec<FailGuardCandidate>> {
    let conn = connect()?;
    let mut sql = String::from(
        r#"
        SELECT
          id, repo, source_type, source_ref, title, outcome, lesson, prevention,
          affected_paths_json, evidence_json, confidence, correlation_key, occurrence_count,
          status, memory_ref, resolution_note, created_at, updated_at, last_seen_at,
          recurrence_of
        FROM failguard_candidates
        WHERE 1=1
        "#,
    );
    let mut params = Vec::new();

    if let Some(repo) = repo.filter(|value| !value.trim().is_empty()) {
        sql.push_str(&format!(" AND repo = ?{}", params.len() + 1));
        params.push(repo.to_string());
    }

    if let Some(status) = status.filter(|value| !value.trim().is_empty() && *value != "all") {
        sql.push_str(&format!(" AND status = ?{}", params.len() + 1));
        params.push(status.to_string());
    }

    sql.push_str(
        r#"
        ORDER BY
          CASE status WHEN 'open' THEN 0 WHEN 'promoted' THEN 1 ELSE 2 END,
          confidence DESC,
          updated_at DESC
        "#,
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter()), decode_failguard_candidate)?;
    rows.collect()
}

pub fn get_failguard_candidate(id: &str) -> rusqlite::Result<Option<FailGuardCandidate>> {
    let conn = connect()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT
          id, repo, source_type, source_ref, title, outcome, lesson, prevention,
          affected_paths_json, evidence_json, confidence, correlation_key, occurrence_count,
          status, memory_ref, resolution_note, created_at, updated_at, last_seen_at,
          recurrence_of
        FROM failguard_candidates
        WHERE id = ?1
        "#,
    )?;
    stmt.query_row(params![id], decode_failguard_candidate)
        .optional()
}

pub fn update_failguard_candidate_status(
    id: &str,
    status: &str,
    memory_ref: Option<&str>,
    resolution_note: &str,
) -> rusqlite::Result<bool> {
    let conn = connect()?;
    let changed = conn.execute(
        r#"
        UPDATE failguard_candidates
        SET status = ?2,
            memory_ref = COALESCE(?3, memory_ref),
            resolution_note = ?4,
            updated_at = datetime('now')
        WHERE id = ?1
        "#,
        params![id, status, memory_ref, resolution_note],
    )?;
    Ok(changed > 0)
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<()> {
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

fn decode_failguard_candidate(row: &rusqlite::Row<'_>) -> rusqlite::Result<FailGuardCandidate> {
    let affected_paths_json: String = row.get(8)?;
    let evidence_json: String = row.get(9)?;
    Ok(FailGuardCandidate {
        id: row.get(0)?,
        repo: row.get(1)?,
        source_type: row.get(2)?,
        source_ref: row.get(3)?,
        title: row.get(4)?,
        outcome: row.get(5)?,
        lesson: row.get(6)?,
        prevention: row.get(7)?,
        affected_paths: serde_json::from_str(&affected_paths_json).unwrap_or_default(),
        evidence: serde_json::from_str(&evidence_json).unwrap_or_default(),
        confidence: row.get(10)?,
        correlation_key: row.get(11)?,
        occurrence_count: row.get::<_, i64>(12)? as u32,
        status: row.get(13)?,
        memory_ref: row.get(14)?,
        resolution_note: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
        last_seen_at: row.get(18)?,
        recurrence_of: row.get(19)?,
    })
}

fn decode_failguard_guardrail(row: &rusqlite::Row<'_>) -> rusqlite::Result<FailGuardGuardrail> {
    let affected_paths_json: String = row.get(6)?;
    let suggestions_json: String = row.get(7)?;
    Ok(FailGuardGuardrail {
        id: row.get(0)?,
        repo: row.get(1)?,
        candidate_id: row.get(2)?,
        memory_ref: row.get(3)?,
        title: row.get(4)?,
        prevention: row.get(5)?,
        affected_paths: serde_json::from_str(&affected_paths_json).unwrap_or_default(),
        suggestions: serde_json::from_str(&suggestions_json).unwrap_or_default(),
        status: row.get(8)?,
        match_count: row.get::<_, i64>(9)? as u32,
        last_matched_at: row.get(10)?,
        recurrence_count: row.get::<_, i64>(11)? as u32,
        last_recurred_at: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        conn.execute_batch(
            r#"
            CREATE TABLE memory_runs (
              id TEXT PRIMARY KEY,
              repo TEXT NOT NULL,
              created_at TEXT NOT NULL
            );
            CREATE TABLE memory_entries (
              id TEXT PRIMARY KEY,
              memory_ref TEXT NOT NULL,
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
            CREATE TABLE memory_curations (
              repo TEXT NOT NULL,
              memory_ref TEXT NOT NULL,
              disposition TEXT NOT NULL,
              pinned INTEGER NOT NULL,
              PRIMARY KEY (repo, memory_ref)
            );

            INSERT INTO memory_runs (id, repo, created_at) VALUES
              ('old-run', 'owner/repo', '2026-07-12T10:00:00Z'),
              ('new-run', 'owner/repo', '2026-07-12T11:00:00Z'),
              ('other-run', 'other/repo', '2026-07-12T12:00:00Z');

            INSERT INTO memory_entries (
              id, memory_ref, run_id, repo, kind, title, detail, prompt_line,
              confidence, frequency, tags_json, evidence_json, created_at
            ) VALUES
              (
                'old-shared', 'owner-repo__hotspot__shared', 'old-run', 'owner/repo',
                'hotspot', 'Shared memory', 'old detail', 'old prompt', 0.80, 2,
                '[]', '[]', '2026-07-12T10:00:00Z'
              ),
              (
                'old-only', 'owner-repo__convention__durable', 'old-run', 'owner/repo',
                'convention', 'Durable memory', 'durable detail', 'durable prompt', 0.70, 1,
                '[]', '[]', '2026-07-12T10:00:01Z'
              ),
              (
                'old-retired', 'owner-repo__failure__retired', 'old-run', 'owner/repo',
                'failure', 'Retired memory', 'retired detail', 'retired prompt', 0.65, 2,
                '[]', '[]', '2026-07-12T10:00:02Z'
              ),
              (
                'new-shared', 'owner-repo__hotspot__shared', 'new-run', 'owner/repo',
                'hotspot', 'Shared memory', 'new detail', 'new prompt', 0.95, 4,
                '[]', '[{"kind":"merged_pr"}]', '2026-07-12T11:00:00Z'
              ),
              (
                'other-shared', 'owner-repo__hotspot__shared', 'other-run', 'other/repo',
                'hotspot', 'Shared memory', 'other detail', 'other prompt', 0.90, 3,
                '[]', '[]', '2026-07-12T12:00:00Z'
              );

            INSERT INTO memory_curations (repo, memory_ref, disposition, pinned) VALUES
              ('owner/repo', 'owner-repo__hotspot__shared', 'policy', 1),
              ('owner/repo', 'owner-repo__convention__durable', 'policy', 1);
            "#,
        )
        .expect("create memory fixtures");
        conn
    }

    #[test]
    fn memory_library_returns_latest_entry_per_repo_and_memory_ref() {
        let conn = memory_test_connection();

        let memories = list_memories_with_connection(&conn, None, None, None, None)
            .expect("list current durable memories");
        let ids = memories
            .iter()
            .map(|memory| memory.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(memories.len(), 3);
        assert!(ids.contains(&"new-shared"));
        assert!(ids.contains(&"old-only"));
        assert!(ids.contains(&"other-shared"));
        assert!(!ids.contains(&"old-shared"));
        assert!(!ids.contains(&"old-retired"));

        let current = memories
            .iter()
            .find(|memory| memory.id == "new-shared")
            .expect("newest shared memory");
        assert_eq!(current.detail, "new detail");
        assert_eq!(current.disposition, "policy");
        assert!(current.pinned);
    }

    #[test]
    fn ingest_snapshot_applies_existing_memory_curations() {
        let conn = memory_test_connection();
        let mut entries = vec![
            MemoryEntry {
                repo: "owner/repo".into(),
                memory_ref: "owner-repo__hotspot__shared".into(),
                disposition: "signal".into(),
                ..MemoryEntry::default()
            },
            MemoryEntry {
                repo: "owner/repo".into(),
                memory_ref: "owner-repo__testing__uncurated".into(),
                disposition: "signal".into(),
                ..MemoryEntry::default()
            },
        ];

        apply_memory_curations_with_connection(&conn, &mut entries)
            .expect("apply memory curations");

        assert_eq!(entries[0].disposition, "policy");
        assert!(entries[0].pinned);
        assert_eq!(entries[1].disposition, "signal");
        assert!(!entries[1].pinned);
    }

    #[test]
    fn run_detail_keeps_historical_memory_entries() {
        let conn = memory_test_connection();

        let memories =
            list_memories_with_connection(&conn, Some("owner/repo"), None, None, Some("old-run"))
                .expect("list historical run memories");
        let ids = memories
            .iter()
            .map(|memory| memory.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(memories.len(), 3);
        assert!(ids.contains(&"old-shared"));
        assert!(ids.contains(&"old-only"));
        assert!(ids.contains(&"old-retired"));
        assert!(!ids.contains(&"new-shared"));
    }
}

use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use rusqlite::{params, types::Value, Connection, OpenFlags};

const MANIFEST_TABLE: &str = "database_consolidation_manifest";

struct SourceSpec {
    label: &'static str,
    relative_path: &'static str,
}

const SOURCES: &[SourceSpec] = &[
    SourceSpec {
        label: "suite",
        relative_path: "products/refactor-scout/patchhive-backend.db",
    },
    SourceSpec {
        label: "merge-keeper",
        relative_path: "products/merge-keeper/backend/merge-keeper.db",
    },
    SourceSpec {
        label: "release-sentry",
        relative_path: "products/release-sentry/release-sentry.db",
    },
    SourceSpec {
        label: "dep-triage",
        relative_path: "products/dep-triage/dep-triage.db",
    },
    SourceSpec {
        label: "vuln-triage",
        relative_path: "products/vuln-triage/vuln-triage.db",
    },
    SourceSpec {
        label: "flake-sting",
        relative_path: "products/flake-sting/flake-sting.db",
    },
    SourceSpec {
        label: "review-bee",
        relative_path: "products/review-bee/review-bee.db",
    },
    SourceSpec {
        label: "trust-gate",
        relative_path: "products/trust-gate/trust-gate.db",
    },
    SourceSpec {
        label: "repo-memory",
        relative_path: "products/repo-memory/backend/repo-memory.db",
    },
    SourceSpec {
        label: "signal-hive",
        relative_path: "products/signal-hive/signal-hive.db",
    },
    SourceSpec {
        label: "refactor-scout",
        relative_path: "products/refactor-scout/refactor-scout.db",
    },
];

fn main() -> Result<()> {
    let options = Options::parse()?;
    if let Some(parent) = options.target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create target directory {}", parent.display()))?;
    }

    let mut target = Connection::open(&options.target)
        .with_context(|| format!("failed to open target {}", options.target.display()))?;
    target.execute_batch(
        r#"
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=OFF;
        CREATE TABLE IF NOT EXISTS database_consolidation_manifest (
            source_label TEXT NOT NULL,
            source_path TEXT NOT NULL,
            source_table TEXT NOT NULL,
            destination_table TEXT NOT NULL,
            source_rows INTEGER NOT NULL,
            copied_at TEXT NOT NULL,
            PRIMARY KEY (source_label, source_path, source_table)
        );
        "#,
    )?;

    let mut total_tables = 0usize;
    let mut total_rows = 0usize;
    for spec in SOURCES {
        let source_path = options.root.join(spec.relative_path);
        if !source_path.is_file() {
            bail!(
                "required {} database is missing at {}",
                spec.label,
                source_path.display()
            );
        }
        let (tables, rows) = import_source(&mut target, spec.label, &source_path)?;
        total_tables += tables;
        total_rows += rows;
        println!("{}: {tables} tables, {rows} rows", spec.label);
    }

    target.execute_batch("PRAGMA foreign_keys=ON; PRAGMA optimize;")?;
    let integrity: String = target.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        bail!("consolidated database integrity check failed: {integrity}");
    }
    println!(
        "consolidated {total_rows} rows from {total_tables} tables into {}",
        options.target.display()
    );
    Ok(())
}

fn import_source(
    target: &mut Connection,
    label: &str,
    source_path: &Path,
) -> Result<(usize, usize)> {
    let source = Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open source {}", source_path.display()))?;
    let tables = source_tables(&source)?;
    let source_path_text = source_path.display().to_string();
    let copied_at = Utc::now().to_rfc3339();
    let transaction = target.transaction()?;
    let mut copied_rows = 0usize;

    for (source_table, create_sql) in &tables {
        let destination_table = destination_table(label, source_table);
        ensure_destination_table(&transaction, destination_table, create_sql)?;
        let rows = copy_table(&source, &transaction, source_table, destination_table)?;
        copied_rows += rows;
        transaction.execute(
            &format!(
                "INSERT OR REPLACE INTO {} (source_label, source_path, source_table, destination_table, source_rows, copied_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                quote_identifier(MANIFEST_TABLE)
            ),
            params![
                label,
                source_path_text,
                source_table,
                destination_table,
                rows as i64,
                copied_at,
            ],
        )?;
    }
    transaction.commit()?;
    Ok((tables.len(), copied_rows))
}

fn source_tables(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut statement = conn.prepare(
        "SELECT name, sql FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn destination_table<'a>(label: &str, source_table: &'a str) -> &'a str {
    if label != "refactor-scout" {
        return source_table;
    }
    match source_table {
        "scans" => "refactor_scout_scans",
        "scan_presets" => "refactor_scout_scan_presets",
        "repo_lists" => "refactor_scout_repo_lists",
        _ => source_table,
    }
}

fn ensure_destination_table(
    target: &Connection,
    destination_table: &str,
    source_create_sql: &str,
) -> Result<()> {
    let exists: bool = target.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [destination_table],
        |row| row.get(0),
    )?;
    if exists {
        return Ok(());
    }

    let columns_start = source_create_sql.find('(').with_context(|| {
        format!("could not parse CREATE TABLE statement for {destination_table}")
    })?;
    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS {} {}",
        quote_identifier(destination_table),
        &source_create_sql[columns_start..]
    );
    target.execute_batch(&create_sql)?;
    Ok(())
}

fn copy_table(
    source: &Connection,
    target: &Connection,
    source_table: &str,
    destination_table: &str,
) -> Result<usize> {
    let source_columns = table_columns(source, source_table)?;
    let source_column_set = source_columns.iter().cloned().collect::<HashSet<_>>();
    let columns = table_columns(target, destination_table)?
        .into_iter()
        .filter(|column| source_column_set.contains(column))
        .collect::<Vec<_>>();
    if columns.is_empty() {
        bail!("no compatible columns for {source_table} -> {destination_table}");
    }

    let column_sql = columns
        .iter()
        .map(|column| quote_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let select_sql = format!(
        "SELECT {column_sql} FROM {}",
        quote_identifier(source_table)
    );
    let mut select = source.prepare(&select_sql)?;
    let rows = select.query_map([], |row| {
        (0..columns.len())
            .map(|index| row.get::<_, Value>(index))
            .collect::<rusqlite::Result<Vec<_>>>()
    })?;
    let values = rows.collect::<rusqlite::Result<Vec<_>>>()?;

    let placeholders = (1..=columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let insert_sql = format!(
        "INSERT OR IGNORE INTO {} ({column_sql}) VALUES ({placeholders})",
        quote_identifier(destination_table)
    );
    let mut insert = target.prepare(&insert_sql)?;
    for row in &values {
        insert.execute(rusqlite::params_from_iter(row.iter()))?;
    }
    Ok(values.len())
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({})", quote_identifier(table)))?;
    let rows = statement.query_map([], |row| row.get(1))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

struct Options {
    root: PathBuf,
    target: PathBuf,
}

impl Options {
    fn parse() -> Result<Self> {
        let default_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .context("failed to resolve PatchHive repository root")?;
        let mut root = default_root;
        let mut target = None;
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => {
                    root = PathBuf::from(args.next().context("--root requires a path")?);
                }
                "--target" => {
                    target = Some(PathBuf::from(
                        args.next().context("--target requires a path")?,
                    ));
                }
                "--help" | "-h" => {
                    println!(
                        "Usage: consolidate-databases [--root REPO] [--target DB]\n\nDefaults to <repo>/data/patchhive.db. Stop the unified backend and back up source DBs before running against live data."
                    );
                    std::process::exit(0);
                }
                _ => bail!("unknown argument: {arg}"),
            }
        }
        let target = target.unwrap_or_else(|| root.join("data/patchhive.db"));
        Ok(Self { root, target })
    }
}

#[cfg(test)]
mod tests {
    use super::{copy_table, destination_table, ensure_destination_table, quote_identifier};
    use rusqlite::Connection;

    #[test]
    fn refactor_scout_generic_tables_are_namespaced() {
        assert_eq!(
            destination_table("refactor-scout", "scans"),
            "refactor_scout_scans"
        );
        assert_eq!(
            destination_table("refactor-scout", "scan_presets"),
            "refactor_scout_scan_presets"
        );
        assert_eq!(destination_table("signal-hive", "scans"), "scans");
    }

    #[test]
    fn sqlite_identifiers_are_safely_quoted() {
        assert_eq!(quote_identifier("normal"), "\"normal\"");
        assert_eq!(quote_identifier("odd\"name"), "\"odd\"\"name\"");
    }

    #[test]
    fn signal_and_refactor_scan_histories_coexist_idempotently() {
        let signal = Connection::open_in_memory().expect("signal source should open");
        signal
            .execute_batch(
                "CREATE TABLE scans (id TEXT PRIMARY KEY, summary TEXT NOT NULL);\
                 INSERT INTO scans VALUES ('signal-1', 'signal');",
            )
            .expect("signal fixture should initialize");
        let refactor = Connection::open_in_memory().expect("refactor source should open");
        refactor
            .execute_batch(
                "CREATE TABLE scans (id TEXT PRIMARY KEY, summary TEXT NOT NULL);\
                 INSERT INTO scans VALUES ('refactor-1', 'refactor');",
            )
            .expect("refactor fixture should initialize");
        let target = Connection::open_in_memory().expect("target should open");

        ensure_destination_table(
            &target,
            "scans",
            "CREATE TABLE scans (id TEXT PRIMARY KEY, summary TEXT NOT NULL)",
        )
        .expect("signal table should initialize");
        ensure_destination_table(
            &target,
            "refactor_scout_scans",
            "CREATE TABLE scans (id TEXT PRIMARY KEY, summary TEXT NOT NULL)",
        )
        .expect("refactor table should initialize");
        copy_table(&signal, &target, "scans", "scans").expect("signal rows should copy");
        copy_table(&refactor, &target, "scans", "refactor_scout_scans")
            .expect("refactor rows should copy");
        copy_table(&refactor, &target, "scans", "refactor_scout_scans")
            .expect("repeat import should succeed");

        let signal_count: i64 = target
            .query_row("SELECT COUNT(*) FROM scans", [], |row| row.get(0))
            .expect("signal count should load");
        let refactor_count: i64 = target
            .query_row("SELECT COUNT(*) FROM refactor_scout_scans", [], |row| {
                row.get(0)
            })
            .expect("refactor count should load");
        assert_eq!(signal_count, 1);
        assert_eq!(refactor_count, 1);
    }
}

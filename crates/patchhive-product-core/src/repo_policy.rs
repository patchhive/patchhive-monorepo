//! Suite-wide repository policy: one allow/deny/opt-out list for every product.
//!
//! Why this is shared rather than per-product: a repository owner who does not want
//! RepoReaper opening pull requests does not want SignalHive scanning them either.
//! Exclusion is a property of the repository, not of one product's settings.
//!
//! Before this module, five stores could each answer the question differently —
//! `repo_lists` (SignalHive), `refactor_scout_repo_lists`, `repo_reaper_repo_lists`,
//! HiveCore's `repository_policies`, and free text in `suite_settings`. The
//! `RepoScopePolicy` evaluator in `scope_policy` was already shared, so every product
//! applied the same *rules* to a different *list*, which is worse than obviously
//! separate: it looks consistent and is not.
//!
//! One table in the suite database, one query, and because integrated products all
//! resolve `PATCHHIVE_DB_PATH`, they are physically reading the same rows.
//!
//! See docs/hivecore-repository-safety-and-pr-budgets.md and
//! docs/suite-backend-direction.md § Suite Scope Policy.

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::scope_policy::normalize_repo_name;

pub const TABLE: &str = "patchhive_repo_policy";

/// How a repository is listed. Ordered by strength — see [`Decision`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyKind {
    /// Strongest. Cannot be overridden by trust, allowlist, schedule, or budget.
    OptOut,
    Denylist,
    Allowlist,
    /// An elevation, not a permission: it unlocks operations that require trust,
    /// and never bypasses an exclusion.
    Trusted,
}

impl PolicyKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "opt_out" | "opt-out" | "optout" => Some(Self::OptOut),
            "denylist" | "blocklist" | "deny" => Some(Self::Denylist),
            "allowlist" | "allow" => Some(Self::Allowlist),
            "trusted" | "trust" => Some(Self::Trusted),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OptOut => "opt_out",
            Self::Denylist => "denylist",
            Self::Allowlist => "allowlist",
            Self::Trusted => "trusted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoPolicyEntry {
    /// Normalized `owner/repo`.
    pub repository: String,
    pub kind: PolicyKind,
    /// Who or what created this entry — an operator, a migration, the public
    /// opt-out service. Recorded so an exclusion can be traced to its origin.
    pub source: String,
    pub notes: String,
    /// True only for opt-outs verified through the public patchhive.dev flow.
    pub verified: bool,
    pub updated_at: String,
}

/// The outcome of a policy question, with the reason chain that produced it.
///
/// A denial is product evidence, not a server error: products record this with the
/// run so "why was this repository skipped" is answerable later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub repository: String,
    pub product: String,
    pub operation: String,
    pub allowed: bool,
    pub trusted: bool,
    /// Each precedence step that was evaluated, in order.
    pub chain: Vec<String>,
    pub reason: String,
    pub policy_version: &'static str,
    pub evaluated_at: String,
}

pub const POLICY_VERSION: &str = "patchhive.repo-policy.v1";

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        r#"
        CREATE TABLE IF NOT EXISTS {TABLE} (
            repository TEXT NOT NULL,
            kind TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'operator',
            notes TEXT NOT NULL DEFAULT '',
            verified INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (repository, kind)
        );

        CREATE INDEX IF NOT EXISTS idx_patchhive_repo_policy_kind
            ON {TABLE} (kind, repository);
        "#
    ))
    .context("could not initialise the shared repository policy schema")?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<RepoPolicyEntry>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT repository, kind, source, notes, verified, updated_at FROM {TABLE} \
         ORDER BY repository, kind"
    ))?;
    let rows = stmt.query_map([], decode)?;
    Ok(rows.flatten().collect())
}

pub fn entries_for(conn: &Connection, repository: &str) -> Result<Vec<RepoPolicyEntry>> {
    let Some(normalized) = normalize_repo_name(repository) else {
        return Ok(Vec::new());
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT repository, kind, source, notes, verified, updated_at FROM {TABLE} \
         WHERE repository = ?1"
    ))?;
    let rows = stmt.query_map([&normalized], decode)?;
    Ok(rows.flatten().collect())
}

/// Insert or update one entry. Repository names are normalized, so `Owner/Repo` and
/// `owner/repo` cannot become two rows that disagree.
pub fn upsert(conn: &Connection, entry: &RepoPolicyEntry) -> Result<()> {
    let repository = normalize_repo_name(&entry.repository)
        .with_context(|| format!("repository `{}` must be owner/repo", entry.repository))?;
    conn.execute(
        &format!(
            "INSERT INTO {TABLE} (repository, kind, source, notes, verified, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(repository, kind) DO UPDATE SET \
               source = excluded.source, notes = excluded.notes, \
               verified = excluded.verified, updated_at = excluded.updated_at"
        ),
        params![
            repository,
            entry.kind.as_str(),
            entry.source,
            entry.notes,
            i64::from(entry.verified),
            entry.updated_at,
        ],
    )?;
    Ok(())
}

/// Insert only when the (repository, kind) pair is absent.
///
/// Used by the migration: re-running it must not rewrite `updated_at`, which would
/// make "when was this rule set" meaningless, and must not clobber notes or source
/// an operator edited after the first import.
pub fn insert_if_absent(conn: &Connection, entry: &RepoPolicyEntry) -> Result<bool> {
    let repository = normalize_repo_name(&entry.repository)
        .with_context(|| format!("repository `{}` must be owner/repo", entry.repository))?;
    let inserted = conn.execute(
        &format!(
            "INSERT OR IGNORE INTO {TABLE} \
             (repository, kind, source, notes, verified, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        ),
        params![
            repository,
            entry.kind.as_str(),
            entry.source,
            entry.notes,
            i64::from(entry.verified),
            entry.updated_at,
        ],
    )?;
    Ok(inserted > 0)
}

pub fn remove(conn: &Connection, repository: &str, kind: PolicyKind) -> Result<bool> {
    let Some(normalized) = normalize_repo_name(repository) else {
        return Ok(false);
    };
    let changed = conn.execute(
        &format!("DELETE FROM {TABLE} WHERE repository = ?1 AND kind = ?2"),
        params![normalized, kind.as_str()],
    )?;
    Ok(changed > 0)
}

fn decode(row: &rusqlite::Row<'_>) -> rusqlite::Result<RepoPolicyEntry> {
    let kind: String = row.get(1)?;
    Ok(RepoPolicyEntry {
        repository: row.get(0)?,
        kind: PolicyKind::parse(&kind).unwrap_or(PolicyKind::Denylist),
        source: row.get(2)?,
        notes: row.get(3)?,
        verified: row.get::<_, i64>(4)? != 0,
        updated_at: row.get(5)?,
    })
}

fn allowlist_is_configured(conn: &Connection) -> Result<bool> {
    let count: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {TABLE} WHERE kind = 'allowlist'"),
            [],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(count > 0)
}

/// Operations that require the repository to be explicitly trusted.
///
/// Trust permits *attempting* an operation under normal sandbox and validation
/// rules. It is never evidence that the repository's code is safe.
pub fn operation_requires_trust(operation: &str) -> bool {
    matches!(
        operation.trim().to_ascii_lowercase().as_str(),
        "execute_repository_tests" | "execute_host_tests" | "broader_sandbox"
    )
}

/// Evaluate one repository for one product operation.
///
/// Precedence, from docs/hivecore-repository-safety-and-pr-budgets.md:
/// opt-out → denylist → allowlist → trust requirement. Earlier denials cannot be
/// overridden by later grants; trust never overrides an exclusion.
pub fn evaluate(
    conn: &Connection,
    repository: &str,
    product: &str,
    operation: &str,
) -> Result<Decision> {
    let evaluated_at = now_rfc3339();
    let product = product.trim().to_string();
    let operation = operation.trim().to_string();

    let Some(normalized) = normalize_repo_name(repository) else {
        return Ok(Decision {
            repository: repository.to_string(),
            product,
            operation,
            allowed: false,
            trusted: false,
            chain: vec!["repository name is not owner/repo".into()],
            reason: "Repository must use owner/repo format.".into(),
            policy_version: POLICY_VERSION,
            evaluated_at,
        });
    };

    let entries = entries_for(conn, &normalized)?;
    let has = |kind: PolicyKind| entries.iter().any(|entry| entry.kind == kind);
    let trusted = has(PolicyKind::Trusted);
    let mut chain = Vec::new();

    if has(PolicyKind::OptOut) {
        chain.push("public repository opt-out: excluded".into());
        return Ok(Decision {
            repository: normalized.clone(),
            product,
            operation,
            allowed: false,
            trusted: false,
            chain,
            reason: format!("{normalized} has opted out of PatchHive automation."),
            policy_version: POLICY_VERSION,
            evaluated_at,
        });
    }
    chain.push("public repository opt-out: none".into());

    if has(PolicyKind::Denylist) {
        chain.push("operator denylist: excluded".into());
        return Ok(Decision {
            repository: normalized.clone(),
            product,
            operation,
            allowed: false,
            trusted,
            chain,
            reason: format!("{normalized} is on the operator denylist."),
            policy_version: POLICY_VERSION,
            evaluated_at,
        });
    }
    chain.push("operator denylist: not listed".into());

    // An allowlist only constrains when one exists. An empty allowlist means "no
    // allowlist configured", not "nothing is allowed" — the opposite reading would
    // silently stop the whole suite the first time the table is created.
    if allowlist_is_configured(conn)? {
        if !has(PolicyKind::Allowlist) {
            chain.push("allowlist: configured, repository not listed".into());
            return Ok(Decision {
                repository: normalized.clone(),
                product,
                operation,
                allowed: false,
                trusted,
                chain,
                reason: format!("{normalized} is not in the operator allowlist."),
                policy_version: POLICY_VERSION,
                evaluated_at,
            });
        }
        chain.push("allowlist: listed".into());
    } else {
        chain.push("allowlist: not configured".into());
    }

    if operation_requires_trust(&operation) && !trusted {
        chain.push(format!(
            "operation `{operation}` requires trust: not trusted"
        ));
        return Ok(Decision {
            repository: normalized.clone(),
            product,
            operation: operation.clone(),
            allowed: false,
            trusted,
            chain,
            reason: format!("Operation `{operation}` requires {normalized} to be trusted."),
            policy_version: POLICY_VERSION,
            evaluated_at,
        });
    }
    chain.push(if trusted {
        "trust requirement: satisfied".into()
    } else {
        "trust requirement: none".into()
    });

    Ok(Decision {
        repository: normalized.clone(),
        product,
        operation,
        allowed: true,
        trusted,
        chain,
        reason: format!("{normalized} is eligible."),
        policy_version: POLICY_VERSION,
        evaluated_at,
    })
}

/// Filter a discovered repository set through policy in one pass.
///
/// Discovery is where a single wrong list does the most damage, because nobody typed
/// the repository names. Returns the allowed set plus every exclusion with its
/// reason, so a run can record what it skipped and why.
pub fn filter_discovered(
    conn: &Connection,
    repositories: &[String],
    product: &str,
    operation: &str,
) -> Result<(Vec<String>, Vec<Decision>)> {
    let mut allowed = Vec::new();
    let mut excluded = Vec::new();
    for repository in repositories {
        let decision = evaluate(conn, repository, product, operation)?;
        if decision.allowed {
            allowed.push(decision.repository);
        } else {
            excluded.push(decision);
        }
    }
    Ok((allowed, excluded))
}

/// Initialize the shared table and fold any legacy per-product lists into it.
///
/// Every product calls this from its own schema init. In suite mode they share one
/// database and the first caller does the work; standalone, each product migrates its
/// own copy. Either way the product stops reading its private table afterwards.
///
/// Conflicts are logged, not swallowed: a repository allowed in one product and denied
/// in another resolves toward exclusion, and an operator needs to know it happened.
pub fn init_and_migrate(conn: &Connection, product: &str) -> Result<MigrationReport> {
    init_schema(conn)?;
    let report = migrate_legacy_tables(conn)?;
    if report.imported > 0 {
        tracing::info!(
            "{product}: imported {} repository policy entries from {}",
            report.imported,
            report.sources.join(", ")
        );
    }
    for conflict in &report.conflicts {
        tracing::warn!(
            "{product}: repository policy conflict for {}: {} — resolved to {}",
            conflict.repository,
            conflict.claims.join(" / "),
            conflict.resolved_to.as_str()
        );
    }
    Ok(report)
}

/// The shared store as a [`RepoScopePolicy`], for products that already filter with one.
///
/// Products reached the same three sets from their own `*_repo_lists` table. The sets
/// were never the problem — the *source* was, because five copies of a list is five
/// chances for a repository to be excluded in one product and reachable from another.
/// This hands back the familiar shape, read from the one table, so a product repoints
/// without rewriting the loop it already filters in.
///
/// Trust is deliberately not represented: it gates elevated operations, which these
/// products do not perform, and flattening an elevation into a permission set is how
/// a safety distinction gets lost.
pub fn scope_policy(conn: &Connection) -> Result<crate::scope_policy::RepoScopePolicy> {
    let entries = list(conn)?;
    let collect = |kind| {
        entries
            .iter()
            .filter(|entry| entry.kind == kind)
            .map(|entry| entry.repository.clone())
            .collect::<std::collections::HashSet<_>>()
    };
    Ok(crate::scope_policy::RepoScopePolicy::new(
        collect(PolicyKind::Allowlist),
        collect(PolicyKind::Denylist),
        collect(PolicyKind::OptOut),
    ))
}

/// Record a repository listing from a product's own list editor.
///
/// `list_type` is the legacy `allowlist` / `denylist` / `opt_out` spelling the product
/// UIs already send. An unrecognised value is rejected rather than defaulted: guessing
/// a listing wrong in either direction is a safety decision, not a formatting one.
pub fn record_listing(
    conn: &Connection,
    repository: &str,
    list_type: &str,
    source: &str,
) -> Result<RepoPolicyEntry> {
    let Some(repository) = normalize_repo_name(repository) else {
        anyhow::bail!("Repository must use owner/repo format.");
    };
    let Some(kind) = PolicyKind::parse(list_type) else {
        anyhow::bail!("Unknown list type '{list_type}'.");
    };
    let entry = RepoPolicyEntry {
        repository,
        kind,
        source: source.to_string(),
        notes: String::new(),
        verified: false,
        updated_at: now_rfc3339(),
    };
    upsert(conn, &entry)?;
    Ok(entry)
}

/// Remove every operator-owned listing for a repository.
///
/// Verified public opt-outs are left in place. A product's list editor removing a row
/// means "I no longer list this"; it cannot mean "the owner withdrew their opt-out".
pub fn remove_listings(conn: &Connection, repository: &str) -> Result<usize> {
    let Some(repository) = normalize_repo_name(repository) else {
        return Ok(0);
    };
    let removed = conn.execute(
        &format!("DELETE FROM {TABLE} WHERE repository = ?1 AND kind <> ?2"),
        params![repository, PolicyKind::OptOut.as_str()],
    )?;
    Ok(removed)
}

/// What a legacy-table import did, and what it could not decide cleanly.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationReport {
    pub imported: usize,
    /// Repositories listed differently by different products.
    pub conflicts: Vec<PolicyConflict>,
    /// Legacy tables that were present and read.
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyConflict {
    pub repository: String,
    /// Every (source table, kind) pair that mentioned this repository.
    pub claims: Vec<String>,
    /// What was written — always the strongest claim.
    pub resolved_to: PolicyKind,
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |_| Ok(()),
    )
    .optional()
    .ok()
    .flatten()
    .is_some()
}

/// Strength order for conflict resolution. Exclusion always wins over permission.
fn strength(kind: PolicyKind) -> u8 {
    match kind {
        PolicyKind::OptOut => 3,
        PolicyKind::Denylist => 2,
        PolicyKind::Allowlist => 1,
        PolicyKind::Trusted => 0,
    }
}

/// Import the per-product repo lists into the shared table.
///
/// Conflicts are resolved toward the strongest claim and reported, never unioned
/// toward permission: if one product denied a repository and another allowed it,
/// treating that as "allowed" would quietly widen what the suite may touch. Trust is
/// carried separately from exclusion, so a trusted-and-denied repository stays
/// denied.
///
/// Idempotent — upserts by (repository, kind), so re-running changes nothing.
pub fn migrate_legacy_tables(conn: &Connection) -> Result<MigrationReport> {
    use std::collections::BTreeMap;

    let mut claims: BTreeMap<String, Vec<(String, PolicyKind)>> = BTreeMap::new();
    let mut report = MigrationReport::default();

    for table in [
        "repo_lists",
        "refactor_scout_repo_lists",
        "repo_reaper_repo_lists",
    ] {
        if !table_exists(conn, table) {
            continue;
        }
        report.sources.push(table.to_string());
        let mut stmt = conn.prepare(&format!("SELECT repo, list_type FROM {table}"))?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows.flatten() {
            let (repo, list_type) = row;
            let (Some(repository), Some(kind)) =
                (normalize_repo_name(&repo), PolicyKind::parse(&list_type))
            else {
                continue;
            };
            claims
                .entry(repository)
                .or_default()
                .push((table.to_string(), kind));
        }
    }

    // HiveCore's structured table used booleans rather than a list type.
    if table_exists(conn, "repository_policies") {
        report.sources.push("repository_policies".to_string());
        let mut stmt =
            conn.prepare("SELECT repository, trusted, operator_excluded FROM repository_policies")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? != 0,
                row.get::<_, i64>(2)? != 0,
            ))
        })?;
        for (repo, trusted, excluded) in rows.flatten() {
            let Some(repository) = normalize_repo_name(&repo) else {
                continue;
            };
            if excluded {
                claims
                    .entry(repository.clone())
                    .or_default()
                    .push(("repository_policies".into(), PolicyKind::Denylist));
            }
            if trusted {
                claims
                    .entry(repository)
                    .or_default()
                    .push(("repository_policies".into(), PolicyKind::Trusted));
            }
        }
    }

    // HiveCore's free-text suite settings: two comma/newline-separated fields on a
    // single-row table. Easy to miss precisely because they do not look like a store
    // — but an operator who typed a denial there expects it to hold, and dropping it
    // during migration would silently widen the suite's reach.
    if table_exists(conn, "suite_settings") {
        report.sources.push("suite_settings".to_string());
        let lists = conn
            .query_row(
                "SELECT repo_allowlist, repo_denylist FROM suite_settings WHERE id = 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((allowlist, denylist)) = lists {
            for (raw, kind) in [
                (allowlist, PolicyKind::Allowlist),
                (denylist, PolicyKind::Denylist),
            ] {
                for candidate in raw.split([',', ';', '\n', '\r']) {
                    let Some(repository) = normalize_repo_name(candidate) else {
                        continue;
                    };
                    claims
                        .entry(repository)
                        .or_default()
                        .push(("suite_settings".into(), kind));
                }
            }
        }
    }

    let now = now_rfc3339();
    for (repository, mut entries) in claims {
        entries.sort_by_key(|(_, kind)| std::cmp::Reverse(strength(*kind)));

        // Trust is an elevation, not a listing: carry it alongside whatever
        // exclusion or permission was strongest.
        let trusted = entries.iter().any(|(_, kind)| *kind == PolicyKind::Trusted);
        let listing = entries
            .iter()
            .find(|(_, kind)| *kind != PolicyKind::Trusted)
            .map(|(_, kind)| *kind);

        let distinct: std::collections::BTreeSet<&'static str> = entries
            .iter()
            .filter(|(_, kind)| *kind != PolicyKind::Trusted)
            .map(|(_, kind)| kind.as_str())
            .collect();

        if let Some(kind) = listing {
            let wrote = insert_if_absent(
                conn,
                &RepoPolicyEntry {
                    repository: repository.clone(),
                    kind,
                    source: "migration".into(),
                    notes: "Imported from a per-product repo list.".into(),
                    verified: false,
                    updated_at: now.clone(),
                },
            )?;
            if wrote {
                report.imported += 1;
            }
            // Report the conflict only when this call actually resolved it. Every
            // product runs this migration and every one of them re-reads the same
            // legacy tables, so reporting on inspection made one disagreement look
            // like four — each later run claiming to have resolved something that was
            // already settled. A repeat migration that changes nothing says nothing.
            if wrote && distinct.len() > 1 {
                report.conflicts.push(PolicyConflict {
                    repository: repository.clone(),
                    claims: entries
                        .iter()
                        .map(|(source, kind)| format!("{source}: {}", kind.as_str()))
                        .collect(),
                    resolved_to: kind,
                });
            }
        }

        if trusted
            && insert_if_absent(
                conn,
                &RepoPolicyEntry {
                    repository,
                    kind: PolicyKind::Trusted,
                    source: "migration".into(),
                    notes: "Imported from HiveCore repository policies.".into(),
                    verified: false,
                    updated_at: now.clone(),
                },
            )?
        {
            report.imported += 1;
        }
    }

    Ok(report)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        init_schema(&conn).expect("schema");
        conn
    }

    fn entry(repository: &str, kind: PolicyKind) -> RepoPolicyEntry {
        RepoPolicyEntry {
            repository: repository.into(),
            kind,
            source: "test".into(),
            notes: String::new(),
            verified: false,
            updated_at: now_rfc3339(),
        }
    }

    #[test]
    fn allows_when_no_policy_exists() {
        let conn = conn();
        let decision = evaluate(&conn, "owner/repo", "signal-hive", "scan").unwrap();
        assert!(decision.allowed);
    }

    #[test]
    fn opt_out_beats_everything_including_trust_and_allowlist() {
        let conn = conn();
        upsert(&conn, &entry("owner/repo", PolicyKind::OptOut)).unwrap();
        upsert(&conn, &entry("owner/repo", PolicyKind::Allowlist)).unwrap();
        upsert(&conn, &entry("owner/repo", PolicyKind::Trusted)).unwrap();
        let decision = evaluate(&conn, "owner/repo", "repo-reaper", "open_pull_request").unwrap();
        assert!(!decision.allowed);
        assert!(decision.reason.contains("opted out"));
    }

    #[test]
    fn denylist_blocks_but_opt_out_takes_precedence_in_the_chain() {
        let conn = conn();
        upsert(&conn, &entry("owner/repo", PolicyKind::Denylist)).unwrap();
        let decision = evaluate(&conn, "owner/repo", "signal-hive", "scan").unwrap();
        assert!(!decision.allowed);
        assert_eq!(decision.chain[0], "public repository opt-out: none");
    }

    #[test]
    fn empty_allowlist_does_not_block_everything() {
        let conn = conn();
        upsert(&conn, &entry("other/repo", PolicyKind::Denylist)).unwrap();
        let decision = evaluate(&conn, "owner/repo", "signal-hive", "scan").unwrap();
        assert!(decision.allowed, "an unconfigured allowlist must not deny");
    }

    #[test]
    fn configured_allowlist_excludes_unlisted_repositories() {
        let conn = conn();
        upsert(&conn, &entry("allowed/repo", PolicyKind::Allowlist)).unwrap();
        assert!(
            evaluate(&conn, "allowed/repo", "p", "scan")
                .unwrap()
                .allowed
        );
        assert!(!evaluate(&conn, "other/repo", "p", "scan").unwrap().allowed);
    }

    #[test]
    fn trust_gates_only_operations_that_require_it() {
        let conn = conn();
        assert!(
            evaluate(&conn, "owner/repo", "repo-reaper", "open_pull_request")
                .unwrap()
                .allowed
        );
        assert!(
            !evaluate(
                &conn,
                "owner/repo",
                "repo-reaper",
                "execute_repository_tests"
            )
            .unwrap()
            .allowed
        );
        upsert(&conn, &entry("owner/repo", PolicyKind::Trusted)).unwrap();
        assert!(
            evaluate(
                &conn,
                "owner/repo",
                "repo-reaper",
                "execute_repository_tests"
            )
            .unwrap()
            .allowed
        );
    }

    #[test]
    fn repository_names_normalize_so_casing_cannot_split_a_rule() {
        let conn = conn();
        upsert(&conn, &entry("Owner/Repo", PolicyKind::Denylist)).unwrap();
        let decision = evaluate(&conn, "owner/repo", "signal-hive", "scan").unwrap();
        assert!(!decision.allowed);
    }

    fn legacy_tables(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE repo_lists (repo TEXT, list_type TEXT, added_at TEXT);
             CREATE TABLE repo_reaper_repo_lists (repo TEXT, list_type TEXT, added_at TEXT);
             CREATE TABLE repository_policies (repository TEXT, trusted INTEGER, \
               operator_excluded INTEGER, notes TEXT, updated_at TEXT);",
        )
        .unwrap();
    }

    #[test]
    fn migration_resolves_disagreement_toward_exclusion_and_reports_it() {
        let conn = conn();
        legacy_tables(&conn);
        // SignalHive allowed it; RepoReaper denied it. Unioning toward "allowed"
        // would widen what the suite may touch, which is the unsafe direction.
        conn.execute(
            "INSERT INTO repo_lists VALUES ('owner/repo', 'allowlist', '')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO repo_reaper_repo_lists VALUES ('Owner/Repo', 'denylist', '')",
            [],
        )
        .unwrap();

        let report = migrate_legacy_tables(&conn).unwrap();
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].resolved_to, PolicyKind::Denylist);
        assert!(
            !evaluate(&conn, "owner/repo", "signal-hive", "scan")
                .unwrap()
                .allowed
        );
    }

    #[test]
    fn migration_keeps_trust_separate_from_exclusion() {
        let conn = conn();
        legacy_tables(&conn);
        conn.execute(
            "INSERT INTO repository_policies VALUES ('owner/repo', 1, 1, '', '')",
            [],
        )
        .unwrap();
        migrate_legacy_tables(&conn).unwrap();
        // Trusted and excluded: exclusion must still win.
        let decision = evaluate(&conn, "owner/repo", "repo-reaper", "scan").unwrap();
        assert!(!decision.allowed);
        assert!(
            decision.trusted,
            "trust is recorded, it just does not permit"
        );
    }

    #[test]
    fn migration_is_idempotent() {
        let conn = conn();
        legacy_tables(&conn);
        conn.execute(
            "INSERT INTO repo_lists VALUES ('owner/repo', 'denylist', '')",
            [],
        )
        .unwrap();
        migrate_legacy_tables(&conn).unwrap();
        let before = list(&conn).unwrap();
        migrate_legacy_tables(&conn).unwrap();
        assert_eq!(before, list(&conn).unwrap());
    }

    #[test]
    fn discovery_filter_reports_every_exclusion_with_a_reason() {
        let conn = conn();
        upsert(&conn, &entry("bad/one", PolicyKind::OptOut)).unwrap();
        upsert(&conn, &entry("bad/two", PolicyKind::Denylist)).unwrap();
        let found = vec!["good/one".to_string(), "bad/one".into(), "bad/two".into()];
        let (allowed, excluded) = filter_discovered(&conn, &found, "signal-hive", "scan").unwrap();
        assert_eq!(allowed, vec!["good/one".to_string()]);
        assert_eq!(excluded.len(), 2);
        assert!(excluded.iter().all(|decision| !decision.reason.is_empty()));
    }

    #[test]
    fn migration_absorbs_hivecore_free_text_suite_lists() {
        // These two fields are a store even though they read like a preference: an
        // operator typing a repository into repo_denylist expects a denial. Losing
        // them during migration would quietly widen what the suite may touch.
        let conn = conn();
        conn.execute_batch(
            r#"
            CREATE TABLE suite_settings (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              repo_allowlist TEXT NOT NULL,
              repo_denylist TEXT NOT NULL
            );
            INSERT INTO suite_settings (id, repo_allowlist, repo_denylist)
            -- char(10) is a real newline; a raw-string \n would be a literal
            -- backslash and would not exercise the newline separator at all.
            VALUES (1, 'Owner/Allowed, owner/second',
                       'owner/denied' || char(10) || 'owner/other;junk');
            "#,
        )
        .unwrap();

        let report = migrate_legacy_tables(&conn).unwrap();
        assert!(report.sources.contains(&"suite_settings".to_string()));

        let kinds = |repo: &str| {
            entries_for(&conn, repo)
                .unwrap()
                .into_iter()
                .map(|entry| entry.kind)
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds("owner/allowed"), vec![PolicyKind::Allowlist]);
        assert_eq!(kinds("owner/second"), vec![PolicyKind::Allowlist]);
        assert_eq!(kinds("owner/denied"), vec![PolicyKind::Denylist]);
        assert_eq!(kinds("owner/other"), vec![PolicyKind::Denylist]);
        // "junk" is not owner/repo and must not become a policy row.
        assert!(entries_for(&conn, "junk").unwrap().is_empty());
    }

    #[test]
    fn free_text_denial_beats_free_text_allowance_for_the_same_repo() {
        // Same precedence rule as cross-product conflicts: resolve toward exclusion,
        // and report it rather than silently unioning toward permission.
        let conn = conn();
        conn.execute_batch(
            r#"
            CREATE TABLE suite_settings (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              repo_allowlist TEXT NOT NULL,
              repo_denylist TEXT NOT NULL
            );
            INSERT INTO suite_settings (id, repo_allowlist, repo_denylist)
            VALUES (1, 'owner/both', 'owner/both');
            "#,
        )
        .unwrap();

        let report = migrate_legacy_tables(&conn).unwrap();
        assert_eq!(
            entries_for(&conn, "owner/both")
                .unwrap()
                .into_iter()
                .map(|entry| entry.kind)
                .collect::<Vec<_>>(),
            vec![PolicyKind::Denylist]
        );
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].resolved_to, PolicyKind::Denylist);
    }

    #[test]
    fn one_products_denial_binds_every_other_product() {
        // The whole point of the shared store. SignalHive lists a repository as
        // denied; RepoReaper — a different product, and the write-capable one — must
        // reach the same answer. When each product owned its own table this was the
        // failure that could not be seen from any single product's UI.
        let conn = conn();
        record_listing(&conn, "Owner/Quiet", "denylist", "signal-hive").unwrap();

        for product in [
            "signal-hive",
            "repo-reaper",
            "refactor-scout",
            "vuln-triage",
        ] {
            let decision = evaluate(&conn, "owner/quiet", product, "scan").unwrap();
            assert!(!decision.allowed, "{product} did not honour the denial");
        }
    }

    #[test]
    fn scope_policy_view_carries_exclusions_but_not_trust() {
        // Products that still filter with RepoScopePolicy read the shared store
        // through this. Trust must not leak into it: it is an elevation for
        // operations these products do not perform, and flattening it into a
        // permission set is how a safety distinction quietly disappears.
        let conn = conn();
        upsert(&conn, &entry("owner/allowed", PolicyKind::Allowlist)).unwrap();
        upsert(&conn, &entry("owner/denied", PolicyKind::Denylist)).unwrap();
        upsert(&conn, &entry("owner/gone", PolicyKind::OptOut)).unwrap();
        upsert(&conn, &entry("owner/trusted", PolicyKind::Trusted)).unwrap();

        let policy = scope_policy(&conn).unwrap();
        assert!(policy.allowlist.contains("owner/allowed"));
        assert!(policy.denylist.contains("owner/denied"));
        assert!(policy.opt_out.contains("owner/gone"));
        assert!(!policy.allowlist.contains("owner/trusted"));
    }

    #[test]
    fn a_product_list_editor_cannot_remove_a_verified_opt_out() {
        // "Remove from my list" is not "the owner withdrew their request". Product
        // list editors are the most likely place for that conflation, because from
        // inside one product the opt-out just looks like another row.
        let conn = conn();
        upsert(
            &conn,
            &RepoPolicyEntry {
                repository: "owner/quiet".into(),
                kind: PolicyKind::OptOut,
                source: "patchhive.dev".into(),
                notes: String::new(),
                verified: true,
                updated_at: now_rfc3339(),
            },
        )
        .unwrap();
        record_listing(&conn, "owner/quiet", "denylist", "repo-reaper").unwrap();

        remove_listings(&conn, "owner/quiet").unwrap();

        assert!(
            !evaluate(&conn, "owner/quiet", "repo-reaper", "scan")
                .unwrap()
                .allowed
        );
    }

    #[test]
    fn an_unknown_list_type_is_rejected_rather_than_defaulted() {
        // Defaulting a listing is a safety decision wearing a formatting decision's
        // clothes: guess "denylist" and the operator silently blocks a repository,
        // guess "allowlist" and they silently open one.
        let conn = conn();
        assert!(record_listing(&conn, "owner/repo", "blocklisted-ish", "signal-hive").is_err());
        assert!(entries_for(&conn, "owner/repo").unwrap().is_empty());
    }

    #[test]
    fn a_repeat_migration_reports_no_conflict_it_did_not_resolve() {
        // Every product runs this migration against the same legacy tables. If
        // conflicts were reported on inspection rather than on resolution, one
        // disagreement would be logged once per product and read as several.
        let conn = conn();
        conn.execute_batch(
            r#"
            CREATE TABLE repo_lists (repo TEXT, list_type TEXT);
            CREATE TABLE repo_reaper_repo_lists (repo TEXT, list_type TEXT);
            INSERT INTO repo_lists VALUES ('owner/split', 'allowlist');
            INSERT INTO repo_reaper_repo_lists VALUES ('owner/split', 'denylist');
            "#,
        )
        .unwrap();

        let first = migrate_legacy_tables(&conn).unwrap();
        assert_eq!(first.conflicts.len(), 1);
        assert_eq!(first.conflicts[0].resolved_to, PolicyKind::Denylist);

        let second = migrate_legacy_tables(&conn).unwrap();
        assert_eq!(second.imported, 0);
        assert!(second.conflicts.is_empty(), "resolved conflict re-reported");
    }
}

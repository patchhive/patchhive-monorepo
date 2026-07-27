//! Autonomous repository discovery, filtered by the suite-wide policy store.
//!
//! Discovery is where a wrong repository list does the most damage, because nobody
//! typed the names. An operator reviewing a direct target sees what they are about to
//! act on; a discovery run finds repositories on its own, and the only thing standing
//! between "found" and "acted on" is the policy check. So the check is not something a
//! caller performs after calling this — it happens inside, and there is no entry point
//! that returns unfiltered search results.
//!
//! That is the whole reason this lives in a shared crate rather than in each product.
//! Eleven products each writing "search, then filter" is eleven chances to forget the
//! second half, and the failure is silent: a run that touches an excluded repository
//! looks exactly like a run that did not.
//!
//! What this does **not** do is backfill. Ask for twenty and find that five are
//! excluded, and you get fifteen — not twenty topped up with a second search. Quietly
//! issuing more API calls to hit a number would spend a rate limit the caller did not
//! budget for, and would make the exclusions invisible in the result. The count that
//! was considered and every exclusion with its reason come back alongside the
//! survivors, so a run can record what it skipped and why.

use anyhow::Result;
use patchhive_product_core::repo_policy::{self, Decision};
use reqwest::Client;
use rusqlite::Connection;

use crate::client::search_repositories;
use crate::models::GitHubRepository;

/// What to look for, and on whose behalf.
#[derive(Debug, Clone)]
pub struct DiscoveryRequest<'a> {
    /// A GitHub repository search query.
    pub query: &'a str,
    /// How many results to request from GitHub, before policy filtering.
    pub limit: u32,
    pub sort: &'a str,
    pub order: &'a str,
    /// The product slug, recorded on every decision so an exclusion is traceable.
    pub product: &'a str,
    /// The operation the product intends, e.g. `scan` or `open_pull_request`.
    /// Operations that require trust are refused for untrusted repositories.
    pub operation: &'a str,
}

/// The survivors, plus everything that did not survive and why.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryOutcome {
    /// Repositories the policy store allows for this product and operation.
    pub repositories: Vec<GitHubRepository>,
    /// One decision per excluded repository, each carrying its full reason chain.
    pub excluded: Vec<Decision>,
    /// How many GitHub returned before filtering. `considered - excluded.len()`
    /// is the number kept, which makes a heavily-filtered run obvious rather than
    /// looking like a query that simply found little.
    pub considered: usize,
}

impl DiscoveryOutcome {
    /// True when discovery found repositories but policy excluded every one.
    ///
    /// Worth distinguishing from an empty search: "your query matched nothing" and
    /// "everything your query matched is excluded" call for different operator
    /// responses, and both otherwise surface as zero results.
    pub fn fully_excluded(&self) -> bool {
        self.considered > 0 && self.repositories.is_empty()
    }
}

/// Search GitHub, then keep only what the suite-wide policy store allows.
pub async fn discover_repositories(
    http: &Client,
    conn: &Connection,
    request: DiscoveryRequest<'_>,
) -> Result<DiscoveryOutcome> {
    let found = search_repositories(
        http,
        request.query,
        request.limit,
        request.sort,
        request.order,
    )
    .await?;

    apply_policy(conn, found.items, request.product, request.operation)
}

/// The filtering half, separated from the network half so it is testable.
///
/// A repository GitHub returned without a usable `full_name` is dropped rather than
/// passed through: the policy store keys on `owner/repo`, so a nameless result cannot
/// be evaluated, and "could not be evaluated" must never resolve to "allowed".
pub fn apply_policy(
    conn: &Connection,
    found: Vec<GitHubRepository>,
    product: &str,
    operation: &str,
) -> Result<DiscoveryOutcome> {
    let considered = found.len();
    let mut repositories = Vec::new();
    let mut excluded = Vec::new();

    for repository in found {
        let decision = repo_policy::evaluate(conn, &repository.full_name, product, operation)?;
        if decision.allowed {
            repositories.push(repository);
        } else {
            excluded.push(decision);
        }
    }

    Ok(DiscoveryOutcome {
        repositories,
        excluded,
        considered,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchhive_product_core::repo_policy::{PolicyKind, RepoPolicyEntry};

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        repo_policy::init_schema(&conn).expect("policy schema should initialize");
        conn
    }

    fn repo(full_name: &str) -> GitHubRepository {
        GitHubRepository {
            full_name: full_name.to_string(),
            name: full_name.split('/').next_back().unwrap_or_default().into(),
            ..GitHubRepository::default()
        }
    }

    fn listed(conn: &Connection, repository: &str, kind: PolicyKind) {
        repo_policy::upsert(
            conn,
            &RepoPolicyEntry {
                repository: repository.into(),
                kind,
                source: "test".into(),
                notes: String::new(),
                verified: false,
                updated_at: "2026-07-27T00:00:00Z".into(),
            },
        )
        .expect("policy entry should save");
    }

    #[test]
    fn excluded_repositories_never_reach_the_caller() {
        let conn = conn();
        listed(&conn, "owner/denied", PolicyKind::Denylist);
        listed(&conn, "owner/gone", PolicyKind::OptOut);

        let outcome = apply_policy(
            &conn,
            vec![repo("owner/ok"), repo("owner/denied"), repo("owner/gone")],
            "signal-hive",
            "scan",
        )
        .unwrap();

        let kept: Vec<&str> = outcome
            .repositories
            .iter()
            .map(|item| item.full_name.as_str())
            .collect();
        assert_eq!(kept, vec!["owner/ok"]);
        assert_eq!(outcome.considered, 3);
        assert_eq!(outcome.excluded.len(), 2);
    }

    #[test]
    fn every_exclusion_carries_a_reason_a_run_can_record() {
        // A skipped repository is product evidence, not an absence. Without the
        // reason, "why didn't it touch this one" is unanswerable after the fact.
        let conn = conn();
        listed(&conn, "owner/denied", PolicyKind::Denylist);

        let outcome =
            apply_policy(&conn, vec![repo("owner/denied")], "repo-reaper", "scan").unwrap();

        let decision = &outcome.excluded[0];
        assert!(!decision.reason.is_empty());
        assert!(!decision.chain.is_empty());
        assert_eq!(decision.product, "repo-reaper");
    }

    #[test]
    fn a_repository_with_no_usable_name_is_excluded_not_passed_through() {
        // The store keys on owner/repo. A result that cannot be evaluated must not
        // default to allowed — "unknown" is the one answer that has to fail closed.
        let conn = conn();
        let outcome = apply_policy(&conn, vec![repo("")], "signal-hive", "scan").unwrap();
        assert!(outcome.repositories.is_empty());
        assert_eq!(outcome.excluded.len(), 1);
    }

    #[test]
    fn results_are_not_backfilled_to_replace_excluded_ones() {
        // Discovery returns what survived, not what was asked for. Topping the count
        // back up would hide the exclusions and spend an unbudgeted rate limit.
        let conn = conn();
        listed(&conn, "owner/two", PolicyKind::Denylist);

        let outcome = apply_policy(
            &conn,
            vec![repo("owner/one"), repo("owner/two"), repo("owner/three")],
            "signal-hive",
            "scan",
        )
        .unwrap();

        assert_eq!(outcome.repositories.len(), 2);
        assert_eq!(outcome.considered, 3);
        assert!(!outcome.fully_excluded());
    }

    #[test]
    fn total_exclusion_is_distinguishable_from_an_empty_search() {
        // Both look like zero results; they need different operator responses.
        let conn = conn();
        listed(&conn, "owner/one", PolicyKind::Denylist);

        let all_excluded =
            apply_policy(&conn, vec![repo("owner/one")], "signal-hive", "scan").unwrap();
        let nothing_found = apply_policy(&conn, Vec::new(), "signal-hive", "scan").unwrap();

        assert!(all_excluded.fully_excluded());
        assert!(!nothing_found.fully_excluded());
    }

    #[test]
    fn an_allowlist_confines_discovery_to_listed_repositories() {
        // The reason allowlists exist: a discovery query is a wide net, and an
        // operator who configured an allowlist means "only these", including for
        // repositories the query surfaced that they never named.
        let conn = conn();
        listed(&conn, "owner/one", PolicyKind::Allowlist);

        let outcome = apply_policy(
            &conn,
            vec![repo("owner/one"), repo("owner/unlisted")],
            "signal-hive",
            "scan",
        )
        .unwrap();

        assert_eq!(outcome.repositories.len(), 1);
        assert_eq!(outcome.repositories[0].full_name, "owner/one");
    }

    #[test]
    fn trust_gated_operations_are_refused_for_untrusted_discoveries() {
        // Discovery finding a repository is not a reason to run tests inside it.
        // Trust is granted per repository by an operator, never inferred from the
        // fact that a search returned it.
        let conn = conn();
        let found = vec![repo("owner/one")];

        let scanning = apply_policy(&conn, found.clone(), "refactor-scout", "scan").unwrap();
        assert_eq!(scanning.repositories.len(), 1);

        let testing =
            apply_policy(&conn, found, "refactor-scout", "execute_repository_tests").unwrap();
        assert!(testing.repositories.is_empty());
    }
}

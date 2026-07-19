use std::collections::HashSet;

use anyhow::{anyhow, Result};
use patchhive_github_data::search_repositories;
use patchhive_product_core::hivecore_policy::{
    check_repository_policy, RepositoryPolicyDecisionRequest,
};

use crate::{models::DiscoveryScope, state::AppState};

const DISCOVERY_CANDIDATE_LIMIT: u32 = 50;

pub(crate) fn normalize_discovery_scope(scope: &DiscoveryScope) -> DiscoveryScope {
    DiscoveryScope {
        query: scope.query.trim().to_string(),
        topics: normalize_parts(&scope.topics),
        languages: normalize_parts(&scope.languages),
        min_stars: scope.min_stars.clamp(1, 1_000_000),
        cooldown_days: scope.cooldown_days.clamp(1, 365),
    }
}

pub(crate) fn validate_discovery_scope(scope: &DiscoveryScope) -> Result<()> {
    let scope = normalize_discovery_scope(scope);
    if scope.query.is_empty() && scope.topics.is_empty() && scope.languages.is_empty() {
        return Err(anyhow!(
            "Autonomous discovery needs a search query, topic, or language scope."
        ));
    }
    Ok(())
}

pub(crate) async fn select_repository(
    state: &AppState,
    scope: &DiscoveryScope,
    recently_scanned: &HashSet<String>,
) -> Result<String> {
    let scope = normalize_discovery_scope(scope);
    validate_discovery_scope(&scope)?;
    let languages = if scope.languages.is_empty() {
        vec![String::new()]
    } else {
        scope.languages.clone()
    };
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    let mut errors = Vec::new();

    for language in languages {
        let query = discovery_query(&scope, &language);
        match search_repositories(
            &state.http,
            &query,
            DISCOVERY_CANDIDATE_LIMIT,
            "updated",
            "desc",
        )
        .await
        {
            Ok(response) => {
                for repository in response.items {
                    let name = repository.full_name.trim();
                    if !name.is_empty() && seen.insert(name.to_ascii_lowercase()) {
                        candidates.push(name.to_string());
                    }
                }
            }
            Err(error) => errors.push(error.to_string()),
        }
    }

    let mut policy_blocked = 0usize;
    for repository in candidates
        .iter()
        .filter(|repository| !contains_repo(recently_scanned, repository))
    {
        if repository_policy_allows(state, repository).await? {
            return Ok(repository.clone());
        }
        policy_blocked += 1;
    }

    if candidates.is_empty() {
        let reason = errors
            .first()
            .map(String::as_str)
            .unwrap_or("GitHub returned no matching public repositories");
        return Err(anyhow!(
            "Autonomous discovery could not find a repository in this scope: {reason}."
        ));
    }

    let recent_count = candidates
        .iter()
        .filter(|repository| contains_repo(recently_scanned, repository))
        .count();
    Err(anyhow!(
        "Autonomous discovery found {} matching repositories, but none are currently eligible: {} were scanned by this schedule within the last {} days and {} were blocked by repository policy. Broaden the scope, shorten the cooldown, or review repository controls.",
        candidates.len(),
        recent_count,
        scope.cooldown_days,
        policy_blocked
    ))
}

pub(crate) async fn repository_policy_allows(state: &AppState, repository: &str) -> Result<bool> {
    if !crate::db::repo_scope_policy()?.allows(repository) {
        return Ok(false);
    }
    let decision = check_repository_policy(
        &state.http,
        &RepositoryPolicyDecisionRequest {
            repository: repository.into(),
            product: "refactor-scout".into(),
            operation: "read_only_scan".into(),
        },
    )
    .await?;
    Ok(decision.as_ref().is_none_or(|decision| decision.allowed()))
}

fn discovery_query(scope: &DiscoveryScope, language: &str) -> String {
    let mut parts = vec![
        "archived:false".to_string(),
        "fork:false".to_string(),
        "is:public".to_string(),
        format!("stars:>={}", scope.min_stars),
    ];
    if !scope.query.is_empty() {
        parts.push(scope.query.clone());
    }
    parts.extend(scope.topics.iter().map(|topic| format!("topic:{topic}")));
    if !language.is_empty() {
        parts.push(format!("language:{language}"));
    }
    parts.join(" ")
}

fn normalize_parts(parts: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .filter(|part| seen.insert(part.to_ascii_lowercase()))
        .map(str::to_string)
        .collect()
}

fn contains_repo(repositories: &HashSet<String>, candidate: &str) -> bool {
    repositories
        .iter()
        .any(|repository| repository.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{contains_repo, discovery_query, normalize_discovery_scope};
    use crate::models::DiscoveryScope;

    #[test]
    fn discovery_query_uses_bounded_public_repository_qualifiers() {
        let scope = DiscoveryScope {
            query: "maintenance".into(),
            topics: vec!["developer-tools".into()],
            languages: vec!["rust".into()],
            min_stars: 50,
            cooldown_days: 30,
        };
        assert_eq!(
            discovery_query(&scope, "rust"),
            "archived:false fork:false is:public stars:>=50 maintenance topic:developer-tools language:rust"
        );
    }

    #[test]
    fn discovery_scope_is_trimmed_deduplicated_and_bounded() {
        let normalized = normalize_discovery_scope(&DiscoveryScope {
            query: "  cleanup  ".into(),
            topics: vec!["agents".into(), " AGENTS ".into()],
            languages: vec!["Rust".into(), "rust".into()],
            min_stars: 0,
            cooldown_days: 900,
        });
        assert_eq!(normalized.query, "cleanup");
        assert_eq!(normalized.topics, vec!["agents"]);
        assert_eq!(normalized.languages, vec!["Rust"]);
        assert_eq!(normalized.min_stars, 1);
        assert_eq!(normalized.cooldown_days, 365);
    }

    #[test]
    fn recent_repository_matching_is_case_insensitive() {
        let recent = HashSet::from(["PatchHive/Example".to_string()]);
        assert!(contains_repo(&recent, "patchhive/example"));
    }
}

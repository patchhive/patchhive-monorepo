use anyhow::Result;
use patchhive_github_data::{
    code_search_count, fetch_issues, fetch_repository, search_repositories,
};
use patchhive_product_core::scope_policy::repo_scope_decision;
use reqwest::Client;
use std::collections::HashSet;
use tracing::warn;

use crate::models::{GitHubIssue, ScanParams, SearchRepo};

#[derive(Debug, Clone)]
pub struct MarkerSearchResult {
    pub count: u32,
    pub available: bool,
    pub rate_limited: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RepositoryDiscovery {
    pub repos: Vec<SearchRepo>,
    pub warnings: Vec<String>,
}

fn discovery_slice_label(language: &str) -> String {
    let language = language.trim();
    if language.is_empty() {
        "the requested scope".to_string()
    } else {
        format!("the `{language}` language scope")
    }
}

fn discovery_failure_warning(language: &str, error: &anyhow::Error) -> String {
    let message = error.to_string().to_ascii_lowercase();
    let reason = if message.contains("temporarily unavailable")
        || message.contains("502 bad gateway")
        || message.contains("503 service unavailable")
        || message.contains("504 gateway timeout")
    {
        "GitHub was temporarily unavailable after automatic retries"
    } else if message.contains("rate limit") || message.contains("[rate_limited]") {
        "GitHub rate-limited repository discovery"
    } else {
        "GitHub repository search was unavailable"
    };

    format!(
        "SignalHive could not load {}: {reason}. This scan may be incomplete; missing repositories were not treated as clean evidence.",
        discovery_slice_label(language)
    )
}

fn repo_allowed(
    full_name: &str,
    allowlist: &HashSet<String>,
    denylist: &HashSet<String>,
    opt_out: &HashSet<String>,
) -> bool {
    repo_scope_decision(full_name, allowlist, denylist, opt_out).is_allowed()
}

fn direct_repo_target(params: &ScanParams) -> Option<String> {
    params.direct_repository()
}

pub async fn fetch_repo(client: &Client, full_name: &str) -> Result<SearchRepo> {
    fetch_repository(client, full_name).await
}

pub async fn discover_repositories(
    client: &Client,
    params: &ScanParams,
    allowlist: &HashSet<String>,
    denylist: &HashSet<String>,
    opt_out: &HashSet<String>,
) -> Result<RepositoryDiscovery> {
    if params.search_query.trim().starts_with("repo:") && direct_repo_target(params).is_none() {
        anyhow::bail!("SignalHive direct targets must use `repo:owner/repository` format.");
    }
    if let Some(repo) = direct_repo_target(params) {
        if !repo_allowed(&repo, allowlist, denylist, opt_out) {
            anyhow::bail!(
                "SignalHive repository policy blocks the direct target `{repo}`. Review allowlist, denylist, and opt-out controls before scanning it."
            );
        }
        return Ok(RepositoryDiscovery {
            repos: vec![fetch_repo(client, &repo).await?],
            warnings: Vec::new(),
        });
    }

    if !allowlist.is_empty() {
        let mut repos = Vec::new();
        let mut warnings = Vec::new();
        for repo in allowlist {
            if !repo_allowed(repo, allowlist, denylist, opt_out) {
                continue;
            }
            match fetch_repo(client, repo).await {
                Ok(found) => repos.push(found),
                Err(err) => {
                    warn!("failed to load allowlisted repo {repo}: {err}");
                    warnings.push(format!(
                        "SignalHive could not load allowlisted repository `{repo}` from GitHub. This scan may be incomplete; the missing repository was not treated as clean evidence."
                    ));
                }
            }
            if repos.len() >= params.max_repos as usize {
                break;
            }
        }
        return Ok(RepositoryDiscovery { repos, warnings });
    }

    let languages = if params.languages.is_empty() {
        vec![String::new()]
    } else {
        params.languages.clone()
    };

    let mut seen = std::collections::HashSet::new();
    let mut repos = Vec::new();
    let mut warnings = Vec::new();

    for language in languages {
        if repos.len() >= params.max_repos as usize {
            break;
        }

        let mut query_parts = vec![
            "archived:false".to_string(),
            "is:public".to_string(),
            format!("stars:>={}", params.min_stars.max(1)),
        ];

        if !params.search_query.trim().is_empty() {
            query_parts.push(params.search_query.trim().to_string());
        }

        for topic in &params.topics {
            let topic = topic.trim();
            if !topic.is_empty() {
                query_parts.push(topic.to_string());
            }
        }

        if !language.trim().is_empty() {
            query_parts.push(format!("language:{language}"));
        }

        let response = match search_repositories(
            client,
            &query_parts.join(" "),
            params.max_repos.min(25),
            "updated",
            "desc",
        )
        .await
        {
            Ok(response) => response,
            Err(error) => {
                warn!("repository discovery failed for language scope `{language}`: {error}");
                warnings.push(discovery_failure_warning(&language, &error));
                continue;
            }
        };

        for repo in response.items {
            if !repo_allowed(&repo.full_name, allowlist, denylist, opt_out) {
                continue;
            }
            if seen.insert(repo.full_name.clone()) {
                repos.push(repo);
            }
            if repos.len() >= params.max_repos as usize {
                break;
            }
        }
    }

    Ok(RepositoryDiscovery { repos, warnings })
}

pub async fn fetch_open_issues(
    client: &Client,
    owner: &str,
    repo: &str,
    per_page: u32,
) -> Result<Vec<GitHubIssue>> {
    let mut issues = fetch_issues(
        client,
        &format!("{owner}/{repo}"),
        "open",
        "updated",
        "desc",
        per_page.min(100),
    )
    .await?;

    issues.retain(|issue| issue.pull_request.is_none());
    Ok(issues)
}

fn is_rate_limit_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("rate limit exceeded") || lower.contains("secondary rate limit")
}

pub async fn search_code_marker(
    client: &Client,
    full_name: &str,
    marker: &str,
) -> MarkerSearchResult {
    match code_search_count(client, &format!("{marker} repo:{full_name}")).await {
        Ok(total_count) => MarkerSearchResult {
            count: total_count,
            available: true,
            rate_limited: false,
            warning: None,
        },
        Err(err) => {
            warn!("code search failed for {full_name} marker {marker}: {err}");
            let message = err.to_string();
            let rate_limited = is_rate_limit_error(&message);
            let warning = if rate_limited {
                format!(
                    "GitHub code search rate-limited TODO/FIXME scanning for `{full_name}`. Marker counts may be partial or unavailable in this scan."
                )
            } else {
                format!(
                    "GitHub code search failed for `{full_name}` while checking `{marker}` markers. Marker counts may be partial for this repo."
                )
            };

            MarkerSearchResult {
                count: 0,
                available: false,
                rate_limited,
                warning: Some(warning),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{direct_repo_target, discovery_failure_warning};
    use crate::models::ScanParams;
    use anyhow::anyhow;

    #[test]
    fn direct_repo_target_requires_an_explicit_repo_prefix() {
        let mut params = ScanParams {
            search_query: "repo:NousResearch/hermes-agent".into(),
            ..ScanParams::default()
        };
        assert_eq!(
            direct_repo_target(&params).as_deref(),
            Some("nousresearch/hermes-agent")
        );

        params.search_query = "maintenance tooling".into();
        assert!(direct_repo_target(&params).is_none());

        params.search_query = "repo:not-a-repository".into();
        assert!(direct_repo_target(&params).is_none());
    }

    #[test]
    fn transient_discovery_failures_become_concise_coverage_warnings() {
        let warning = discovery_failure_warning(
            "rust",
            &anyhow!(
                "GitHub GET /search/repositories -> 503 Service Unavailable [http_status]: GitHub is temporarily unavailable after retrying the request."
            ),
        );

        assert!(warning.contains("`rust` language scope"));
        assert!(warning.contains("temporarily unavailable"));
        assert!(warning.contains("not treated as clean evidence"));
        assert!(!warning.contains("DOCTYPE"));
    }
}

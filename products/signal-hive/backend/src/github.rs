use anyhow::Result;
use patchhive_github_data::{
    code_search_count, fetch_issues, fetch_repository, search_repositories,
    validate_token as validate_shared_token,
};
use reqwest::Client;
use std::collections::HashSet;
use tracing::warn;

use crate::models::{GitHubIssue, ScanParams, SearchRepo};

pub async fn validate_token(client: &Client) -> Result<()> {
    validate_shared_token(client).await
}

fn repo_allowed(
    full_name: &str,
    allowlist: &HashSet<String>,
    denylist: &HashSet<String>,
    opt_out: &HashSet<String>,
) -> bool {
    let name = full_name.to_ascii_lowercase();
    if opt_out.contains(&name) || denylist.contains(&name) {
        return false;
    }
    allowlist.is_empty() || allowlist.contains(&name)
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
) -> Result<Vec<SearchRepo>> {
    if !allowlist.is_empty() {
        let mut repos = Vec::new();
        for repo in allowlist {
            if !repo_allowed(repo, allowlist, denylist, opt_out) {
                continue;
            }
            match fetch_repo(client, repo).await {
                Ok(found) => repos.push(found),
                Err(err) => warn!("failed to load allowlisted repo {repo}: {err}"),
            }
            if repos.len() >= params.max_repos as usize {
                break;
            }
        }
        return Ok(repos);
    }

    let languages = if params.languages.is_empty() {
        vec![String::new()]
    } else {
        params.languages.clone()
    };

    let mut seen = std::collections::HashSet::new();
    let mut repos = Vec::new();

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

        let response = search_repositories(
            client,
            &query_parts.join(" "),
            params.max_repos.min(25),
            "updated",
            "desc",
        )
        .await?;

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

    Ok(repos)
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

pub async fn search_code_marker(client: &Client, full_name: &str, marker: &str) -> u32 {
    match code_search_count(client, &format!("{marker} repo:{full_name}")).await {
        Ok(total_count) => total_count,
        Err(err) => {
            warn!("code search failed for {full_name} marker {marker}: {err}");
            0
        }
    }
}

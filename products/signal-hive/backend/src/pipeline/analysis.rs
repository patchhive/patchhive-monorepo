// analysis.rs — Per-repo analysis: issue draft, marker collection, signal finalization

use anyhow::Result;
use patchhive_github_data::code_search_rate_limit;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::warn;

use crate::models::{RepoSignal, ScanParams};

use super::scoring::{
    issue_signals, priority_score, summary_from_signals, MarkerCounts, RepoAnalysisDraft,
    SummarySignalInput,
};

const MAX_CODE_SEARCH_WAIT_SECONDS: u64 = 70;

fn code_search_wait_seconds(remaining: u32, reset: i64, now: i64) -> u64 {
    if remaining > 0 {
        return 0;
    }

    reset.saturating_sub(now).max(0) as u64 + 1
}

async fn wait_for_code_search_capacity(client: &reqwest::Client) -> Result<()> {
    let budget = code_search_rate_limit(client).await?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    let wait_seconds = code_search_wait_seconds(budget.remaining, budget.reset, now);

    if wait_seconds == 0 {
        return Ok(());
    }
    if wait_seconds > MAX_CODE_SEARCH_WAIT_SECONDS {
        anyhow::bail!(
            "GitHub code-search capacity will not reset within the {MAX_CODE_SEARCH_WAIT_SECONDS}-second scan wait budget"
        );
    }

    tokio::time::sleep(Duration::from_secs(wait_seconds)).await;
    let refreshed = code_search_rate_limit(client).await?;
    if refreshed.remaining == 0 {
        anyhow::bail!("GitHub code-search capacity did not reset when expected");
    }
    Ok(())
}

async fn search_marker_with_capacity(
    client: &reqwest::Client,
    full_name: &str,
    marker: &str,
) -> crate::github::MarkerSearchResult {
    if let Err(error) = wait_for_code_search_capacity(client).await {
        warn!("could not confirm GitHub code-search capacity before scanning {full_name}: {error}");
    }

    let first = crate::github::search_code_marker(client, full_name, marker).await;
    if !first.rate_limited {
        return first;
    }

    match wait_for_code_search_capacity(client).await {
        Ok(()) => crate::github::search_code_marker(client, full_name, marker).await,
        Err(error) => {
            warn!("could not recover GitHub code-search capacity for {full_name}: {error}");
            first
        }
    }
}

pub async fn analyze_repo_issue_draft(
    client: &reqwest::Client,
    repo: &crate::models::SearchRepo,
    params: &ScanParams,
) -> Result<RepoAnalysisDraft> {
    let issues = crate::github::fetch_open_issues(
        client,
        &repo.owner.login,
        &repo.name,
        params.issues_per_repo,
    )
    .await?;
    let issue_analysis = issue_signals(&issues, params.stale_days);
    let issue_only_priority_score = priority_score(
        repo.stargazers_count,
        repo.open_issues_count,
        &issue_analysis,
        0,
        0,
    )
    .0;

    Ok(RepoAnalysisDraft {
        repo: repo.clone(),
        issue_analysis,
        issue_only_priority_score,
    })
}

pub async fn collect_marker_counts(client: &reqwest::Client, full_name: &str) -> MarkerCounts {
    let mut counts = MarkerCounts::default();
    let todo_result = search_marker_with_capacity(client, full_name, "TODO").await;
    counts.todo_count = todo_result.count;
    counts.todo_available = todo_result.available;
    if let Some(warning) = todo_result.warning {
        counts.warnings.push(warning);
    }
    if !todo_result.available {
        return counts;
    }

    let fixme_result = search_marker_with_capacity(client, full_name, "FIXME").await;
    counts.fixme_count = fixme_result.count;
    counts.fixme_available = fixme_result.available;
    if let Some(warning) = fixme_result.warning {
        counts.warnings.push(warning);
    }

    counts
}

pub fn finalize_repo_signal(draft: RepoAnalysisDraft, marker_counts: MarkerCounts) -> RepoSignal {
    let (priority_score, score_breakdown) = priority_score(
        draft.repo.stargazers_count,
        draft.repo.open_issues_count,
        &draft.issue_analysis,
        marker_counts.todo_count,
        marker_counts.fixme_count,
    );
    let (summary, signals) = summary_from_signals(SummarySignalInput {
        stars: draft.repo.stargazers_count,
        open_issues: draft.repo.open_issues_count,
        issue_analysis: &draft.issue_analysis,
        todo_count: marker_counts.todo_count,
        fixme_count: marker_counts.fixme_count,
        todo_available: marker_counts.todo_available,
        fixme_available: marker_counts.fixme_available,
        repo_warnings: &marker_counts.warnings,
    });

    RepoSignal {
        full_name: draft.repo.full_name,
        repo_url: draft.repo.html_url,
        description: draft.repo.description.unwrap_or_default(),
        language: draft.repo.language.unwrap_or_else(|| "unknown".into()),
        stars: draft.repo.stargazers_count,
        open_issues: draft.repo.open_issues_count,
        sampled_issues: draft.issue_analysis.sampled_issues,
        stale_issues: draft.issue_analysis.stale_issues,
        unlabeled_issues: draft.issue_analysis.unlabeled_issues,
        stale_bug_issues: draft.issue_analysis.stale_bug_issues,
        stale_high_comment_issues: draft.issue_analysis.stale_high_comment_issues,
        duplicate_candidates: draft.issue_analysis.duplicate_candidates,
        recurring_bug_clusters: draft.issue_analysis.recurring_bug_clusters,
        todo_count: marker_counts.todo_count,
        fixme_count: marker_counts.fixme_count,
        todo_available: marker_counts.todo_available,
        fixme_available: marker_counts.fixme_available,
        priority_score,
        score_breakdown,
        summary,
        signals,
        issue_examples: draft.issue_analysis.issue_examples,
        warnings: marker_counts.warnings,
        trend: None,
    }
}

#[cfg(test)]
mod tests {
    use super::code_search_wait_seconds;

    #[test]
    fn available_code_search_capacity_never_waits() {
        assert_eq!(code_search_wait_seconds(1, 1_100, 1_000), 0);
    }

    #[test]
    fn exhausted_code_search_capacity_waits_through_reset() {
        assert_eq!(code_search_wait_seconds(0, 1_020, 1_000), 21);
        assert_eq!(code_search_wait_seconds(0, 999, 1_000), 1);
    }
}

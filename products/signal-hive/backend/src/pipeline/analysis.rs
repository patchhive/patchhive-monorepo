// analysis.rs — Per-repo analysis: issue draft, marker collection, signal finalization

use anyhow::Result;

use crate::models::{RepoSignal, ScanParams};

use super::scoring::{
    issue_signals, priority_score, summary_from_signals, MarkerCounts, RepoAnalysisDraft,
};

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

pub async fn collect_marker_counts(
    client: &reqwest::Client,
    full_name: &str,
    code_search_rate_limited: &mut bool,
) -> MarkerCounts {
    if *code_search_rate_limited {
        return MarkerCounts {
            warnings: vec![
                "GitHub code search was already rate-limited earlier in this scan, so later repos do not have TODO/FIXME counts.".into(),
            ],
            ..MarkerCounts::default()
        };
    }

    let mut counts = MarkerCounts::default();
    let todo_result = crate::github::search_code_marker(client, full_name, "TODO").await;
    counts.todo_count = todo_result.count;
    counts.todo_available = todo_result.available;
    if let Some(warning) = todo_result.warning {
        counts.warnings.push(warning);
    }
    if todo_result.rate_limited {
        *code_search_rate_limited = true;
        return counts;
    }

    let fixme_result = crate::github::search_code_marker(client, full_name, "FIXME").await;
    counts.fixme_count = fixme_result.count;
    counts.fixme_available = fixme_result.available;
    if let Some(warning) = fixme_result.warning {
        counts.warnings.push(warning);
    }
    if fixme_result.rate_limited {
        *code_search_rate_limited = true;
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
    let (summary, signals) = summary_from_signals(
        draft.repo.stargazers_count,
        draft.repo.open_issues_count,
        &draft.issue_analysis,
        marker_counts.todo_count,
        marker_counts.fixme_count,
        marker_counts.todo_available,
        marker_counts.fixme_available,
        &marker_counts.warnings,
    );

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

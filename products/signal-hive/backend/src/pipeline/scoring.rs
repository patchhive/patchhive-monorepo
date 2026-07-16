// scoring.rs — Issue analysis, duplicate detection, recurring bug clustering, priority scoring

use std::collections::HashSet;

use crate::models::{
    DuplicateCandidate, GitHubIssue, IssueSample, RecurringBugCluster, ScoreFactor,
};

use super::utils::{
    count_label, is_bug_issue, issue_age_days, recurring_bug_tokens, round1, title_tokens,
    to_issue_sample, tokenize_title,
};

// ---- Internal types ----

pub struct IssueAnalysis {
    pub sampled_issues: u32,
    pub stale_issues: u32,
    pub unlabeled_issues: u32,
    pub stale_bug_issues: u32,
    pub stale_high_comment_issues: u32,
    pub issue_examples: Vec<IssueSample>,
    pub duplicate_candidates: Vec<DuplicateCandidate>,
    pub recurring_bug_clusters: Vec<RecurringBugCluster>,
}

pub struct RepoAnalysisDraft {
    pub repo: crate::models::SearchRepo,
    pub issue_analysis: IssueAnalysis,
    pub issue_only_priority_score: f64,
}

#[derive(Default)]
pub struct MarkerCounts {
    pub todo_count: u32,
    pub fixme_count: u32,
    pub todo_available: bool,
    pub fixme_available: bool,
    pub warnings: Vec<String>,
}

// ---- Scoring functions ----

pub fn issue_signals(issues: &[GitHubIssue], stale_days: u32) -> IssueAnalysis {
    let sampled_issues = issues.len() as u32;
    let unlabeled_issues = issues
        .iter()
        .filter(|issue| issue.labels.is_empty())
        .count() as u32;
    let stale_issues = issues
        .iter()
        .filter(|issue| issue_age_days(&issue.updated_at) >= stale_days as i64)
        .count() as u32;
    let stale_bug_issues = issues
        .iter()
        .filter(|issue| {
            issue_age_days(&issue.updated_at) >= stale_days as i64 && is_bug_issue(issue)
        })
        .count() as u32;
    let stale_high_comment_issues = issues
        .iter()
        .filter(|issue| {
            issue_age_days(&issue.updated_at) >= stale_days as i64 && issue.comments >= 3
        })
        .count() as u32;

    IssueAnalysis {
        sampled_issues,
        stale_issues,
        unlabeled_issues,
        stale_bug_issues,
        stale_high_comment_issues,
        issue_examples: stale_issue_examples(issues, stale_days),
        duplicate_candidates: duplicate_candidates(issues),
        recurring_bug_clusters: recurring_bug_clusters(issues),
    }
}

pub fn priority_score(
    stars: u32,
    open_issues: u32,
    issue_analysis: &IssueAnalysis,
    todo_count: u32,
    fixme_count: u32,
) -> (f64, Vec<ScoreFactor>) {
    let sampled = issue_analysis.sampled_issues.max(1) as f64;
    let stale_ratio = issue_analysis.stale_issues as f64 / sampled;
    let unlabeled_ratio = issue_analysis.unlabeled_issues as f64 / sampled;
    let issue_density = (open_issues as f64 / stars.max(25) as f64) * 100.0;
    let duplicate_pressure = issue_analysis
        .duplicate_candidates
        .iter()
        .map(|pair| pair.similarity)
        .sum::<f64>();
    let recurring_bug_issue_count = issue_analysis
        .recurring_bug_clusters
        .iter()
        .map(|cluster| cluster.issue_count)
        .sum::<u32>();

    let mut breakdown = Vec::new();

    let stale_backlog_impact = (stale_ratio * 34.0).min(24.0)
        + (issue_analysis.stale_issues.min(6) as f64 * 2.2).min(12.0);
    if issue_analysis.stale_issues > 0 {
        breakdown.push(ScoreFactor {
            key: "stale_backlog".into(),
            label: "Stale backlog".into(),
            impact: round1(stale_backlog_impact),
            detail: format!(
                "{} of {} {} stale",
                issue_analysis.stale_issues,
                count_label(
                    issue_analysis.sampled_issues,
                    "sampled issue",
                    "sampled issues"
                ),
                if issue_analysis.stale_issues == 1 {
                    "is"
                } else {
                    "are"
                }
            ),
        });
    }

    let stale_bug_impact = (issue_analysis.stale_bug_issues.min(3) as f64 * 7.5).min(18.0);
    if issue_analysis.stale_bug_issues > 0 {
        breakdown.push(ScoreFactor {
            key: "stale_bug".into(),
            label: "Stale bug pressure".into(),
            impact: round1(stale_bug_impact),
            detail: format!(
                "{} {} still open",
                count_label(
                    issue_analysis.stale_bug_issues,
                    "stale bug-like issue",
                    "stale bug-like issues"
                ),
                if issue_analysis.stale_bug_issues == 1 {
                    "is"
                } else {
                    "are"
                }
            ),
        });
    }

    let stalled_discussion_impact =
        (issue_analysis.stale_high_comment_issues.min(3) as f64 * 4.8).min(14.4);
    if issue_analysis.stale_high_comment_issues > 0 {
        breakdown.push(ScoreFactor {
            key: "stalled_discussion".into(),
            label: "Stalled discussions".into(),
            impact: round1(stalled_discussion_impact),
            detail: format!(
                "{} {} 3 or more comments",
                count_label(
                    issue_analysis.stale_high_comment_issues,
                    "stale issue",
                    "stale issues"
                ),
                if issue_analysis.stale_high_comment_issues == 1 {
                    "has"
                } else {
                    "have"
                }
            ),
        });
    }

    let recurring_bug_impact = ((recurring_bug_issue_count.min(6) as f64) * 2.8
        + issue_analysis.recurring_bug_clusters.len() as f64 * 3.5)
        .min(18.0);
    if !issue_analysis.recurring_bug_clusters.is_empty() {
        let strongest = issue_analysis
            .recurring_bug_clusters
            .first()
            .map(|cluster| {
                format!(
                    "top pattern '{}' appears in {} issues",
                    cluster.label, cluster.issue_count
                )
            })
            .unwrap_or_else(|| "bug reports cluster around repeated symptoms".into());
        breakdown.push(ScoreFactor {
            key: "recurring_bugs".into(),
            label: "Recurring bug pattern".into(),
            impact: round1(recurring_bug_impact),
            detail: format!(
                "{} recurring bug clusters across {} issues; {}",
                issue_analysis.recurring_bug_clusters.len(),
                recurring_bug_issue_count,
                strongest
            ),
        });
    }

    let duplicate_impact = (duplicate_pressure * 10.0).min(14.0)
        + if issue_analysis.duplicate_candidates.len() >= 2 {
            3.0
        } else {
            0.0
        };
    if !issue_analysis.duplicate_candidates.is_empty() {
        let strongest = issue_analysis
            .duplicate_candidates
            .first()
            .map(|pair| {
                format!(
                    "strongest pair looks {}% alike",
                    (pair.similarity * 100.0).round() as i64
                )
            })
            .unwrap_or_else(|| "title overlap suggests duplicate work".into());
        breakdown.push(ScoreFactor {
            key: "duplicates".into(),
            label: "Duplicate issue pressure".into(),
            impact: round1(duplicate_impact),
            detail: format!(
                "{}; {}",
                count_label(
                    issue_analysis.duplicate_candidates.len() as u32,
                    "potential duplicate pair",
                    "potential duplicate pairs"
                ),
                strongest
            ),
        });
    }

    let unlabeled_impact = ((unlabeled_ratio * 18.0)
        + (issue_analysis.unlabeled_issues.min(4) as f64 * 1.4))
        .min(12.0);
    if issue_analysis.unlabeled_issues > 0 {
        breakdown.push(ScoreFactor {
            key: "triage_gap".into(),
            label: "Triage gap".into(),
            impact: round1(unlabeled_impact),
            detail: format!(
                "{} {} no labels",
                count_label(
                    issue_analysis.unlabeled_issues,
                    "sampled issue",
                    "sampled issues"
                ),
                if issue_analysis.unlabeled_issues == 1 {
                    "has"
                } else {
                    "have"
                }
            ),
        });
    }

    let marker_impact =
        (todo_count.min(20) as f64 * 0.45 + fixme_count.min(15) as f64 * 0.8).min(12.0);
    if todo_count > 0 || fixme_count > 0 {
        breakdown.push(ScoreFactor {
            key: "markers".into(),
            label: "Code markers".into(),
            impact: round1(marker_impact),
            detail: format!("{todo_count} TODO and {fixme_count} FIXME markers found"),
        });
    }

    let density_impact = ((issue_density - 10.0).max(0.0) * 0.35).min(10.0);
    if density_impact > 0.0 {
        breakdown.push(ScoreFactor {
            key: "issue_density".into(),
            label: "Open issue density".into(),
            impact: round1(density_impact),
            detail: format!(
                "{open_issues} open issues across {stars} stars ({:.1} per 100 stars)",
                issue_density
            ),
        });
    }

    breakdown.sort_by(|a, b| {
        b.impact
            .partial_cmp(&a.impact)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total = breakdown
        .iter()
        .map(|factor| factor.impact)
        .sum::<f64>()
        .min(100.0);
    (round1(total), breakdown)
}

pub struct SummarySignalInput<'a> {
    pub stars: u32,
    pub open_issues: u32,
    pub issue_analysis: &'a IssueAnalysis,
    pub todo_count: u32,
    pub fixme_count: u32,
    pub todo_available: bool,
    pub fixme_available: bool,
    pub repo_warnings: &'a [String],
}

pub fn summary_from_signals(input: SummarySignalInput<'_>) -> (String, Vec<String>) {
    let SummarySignalInput {
        stars,
        open_issues,
        issue_analysis,
        todo_count,
        fixme_count,
        todo_available,
        fixme_available,
        repo_warnings,
    } = input;
    let mut signals = Vec::new();

    if let Some(cluster) = issue_analysis.recurring_bug_clusters.first() {
        signals.push(format!(
            "Recurring bug pattern '{}' appears across {}",
            cluster.label,
            count_label(cluster.issue_count, "sampled issue", "sampled issues")
        ));
    }
    if issue_analysis.stale_bug_issues > 0 {
        signals.push(format!(
            "{} {} still open",
            count_label(
                issue_analysis.stale_bug_issues,
                "stale bug-like issue",
                "stale bug-like issues"
            ),
            if issue_analysis.stale_bug_issues == 1 {
                "is"
            } else {
                "are"
            }
        ));
    }
    if issue_analysis.stale_issues > 0 {
        signals.push(format!(
            "{} of {} {} stale",
            issue_analysis.stale_issues,
            count_label(
                issue_analysis.sampled_issues,
                "sampled issue",
                "sampled issues"
            ),
            if issue_analysis.stale_issues == 1 {
                "looks"
            } else {
                "look"
            }
        ));
    }
    if issue_analysis.stale_high_comment_issues > 0 {
        signals.push(format!(
            "{} {} 3 or more comments",
            count_label(
                issue_analysis.stale_high_comment_issues,
                "stale issue",
                "stale issues"
            ),
            if issue_analysis.stale_high_comment_issues == 1 {
                "has"
            } else {
                "have"
            }
        ));
    }
    if !issue_analysis.duplicate_candidates.is_empty() {
        signals.push(format!(
            "{} {} found",
            count_label(
                issue_analysis.duplicate_candidates.len() as u32,
                "potential duplicate issue pair",
                "potential duplicate issue pairs"
            ),
            if issue_analysis.duplicate_candidates.len() == 1 {
                "was"
            } else {
                "were"
            }
        ));
    }
    if issue_analysis.unlabeled_issues >= 2 {
        signals.push(format!(
            "{} {} unlabeled, which points to triage drift",
            count_label(
                issue_analysis.unlabeled_issues,
                "sampled issue",
                "sampled issues"
            ),
            if issue_analysis.unlabeled_issues == 1 {
                "is"
            } else {
                "are"
            }
        ));
    }
    if todo_available && fixme_available && (todo_count > 0 || fixme_count > 0) {
        signals.push(format!(
            "{todo_count} TODO and {fixme_count} FIXME markers were found in code search"
        ));
    } else if !todo_available && !fixme_available {
        signals.push("TODO/FIXME marker counts were unavailable for this repo in this scan".into());
    } else if !todo_available {
        signals.push("TODO marker counts were unavailable for this repo in this scan".into());
    } else if !fixme_available {
        signals.push("FIXME marker counts were unavailable for this repo in this scan".into());
    }

    let issue_density = (open_issues as f64 / stars.max(25) as f64) * 100.0;
    if issue_density >= 18.0 {
        signals.push(format!(
            "Open issue density is high for repo size ({:.1} per 100 stars)",
            issue_density
        ));
    }
    if signals.is_empty() {
        signals.push("No major maintenance signals found in the current sample".into());
    }

    for warning in repo_warnings {
        if !signals.iter().any(|signal| signal == warning) {
            signals.push(warning.clone());
        }
    }

    let summary = signals
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" · ");
    (summary, signals)
}

// ---- Detection helpers ----

fn stale_issue_examples(issues: &[GitHubIssue], stale_days: u32) -> Vec<IssueSample> {
    let mut examples = issues
        .iter()
        .filter_map(|issue| {
            let age_days = issue_age_days(&issue.updated_at);
            if age_days < stale_days as i64 {
                return None;
            }

            Some(IssueSample {
                age_days,
                ..to_issue_sample(issue)
            })
        })
        .collect::<Vec<_>>();

    examples.sort_by(|a, b| {
        b.age_days
            .cmp(&a.age_days)
            .then_with(|| b.comments.cmp(&a.comments))
    });
    examples.truncate(3);
    examples
}

fn duplicate_candidates(issues: &[GitHubIssue]) -> Vec<DuplicateCandidate> {
    let mut pairs = Vec::new();

    for left_index in 0..issues.len() {
        let left = &issues[left_index];
        let left_tokens = tokenize_title(&left.title);
        let left_phrase = title_tokens(&left.title).join(" ");
        if left_tokens.is_empty() {
            continue;
        }

        for right in issues.iter().skip(left_index + 1) {
            let right_tokens = tokenize_title(&right.title);
            let right_phrase = title_tokens(&right.title).join(" ");
            if right_tokens.is_empty() {
                continue;
            }

            let union = left_tokens.union(&right_tokens).count() as f64;
            let shared = left_tokens.intersection(&right_tokens).count() as f64;
            if union == 0.0 || shared < 2.0 {
                continue;
            }

            let contains_match = left_phrase.len() > 10
                && right_phrase.len() > 10
                && (left_phrase.contains(&right_phrase) || right_phrase.contains(&left_phrase));

            let mut similarity = shared / union;
            if contains_match {
                similarity = similarity.max(0.78);
            }

            if similarity >= 0.72 {
                pairs.push(DuplicateCandidate {
                    left_number: left.number,
                    right_number: right.number,
                    left_title: left.title.clone(),
                    right_title: right.title.clone(),
                    similarity: (similarity * 100.0).round() / 100.0,
                });
            }
        }
    }

    pairs.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    pairs.truncate(3);
    pairs
}

fn recurring_bug_clusters(issues: &[GitHubIssue]) -> Vec<RecurringBugCluster> {
    let bug_issues = issues
        .iter()
        .filter(|issue| is_bug_issue(issue))
        .collect::<Vec<_>>();

    if bug_issues.len() < 2 {
        return Vec::new();
    }

    let token_sets = bug_issues
        .iter()
        .map(|issue| {
            recurring_bug_tokens(&issue.title)
                .into_iter()
                .collect::<HashSet<_>>()
        })
        .collect::<Vec<_>>();

    let mut adjacency = vec![Vec::<usize>::new(); bug_issues.len()];
    for left_index in 0..bug_issues.len() {
        if token_sets[left_index].is_empty() {
            continue;
        }

        for right_index in (left_index + 1)..bug_issues.len() {
            if token_sets[right_index].is_empty() {
                continue;
            }

            let union = token_sets[left_index]
                .union(&token_sets[right_index])
                .count() as f64;
            let shared = token_sets[left_index]
                .intersection(&token_sets[right_index])
                .count() as f64;

            if union == 0.0 {
                continue;
            }

            let similarity = shared / union;
            if shared >= 2.0 && similarity >= 0.34 {
                adjacency[left_index].push(right_index);
                adjacency[right_index].push(left_index);
            }
        }
    }

    let mut visited = vec![false; bug_issues.len()];
    let mut clusters = Vec::new();

    for start in 0..bug_issues.len() {
        if visited[start] {
            continue;
        }

        let mut stack = vec![start];
        let mut component = Vec::new();
        while let Some(index) = stack.pop() {
            if visited[index] {
                continue;
            }
            visited[index] = true;
            component.push(index);
            for neighbor in &adjacency[index] {
                if !visited[*neighbor] {
                    stack.push(*neighbor);
                }
            }
        }

        if component.len() < 2 {
            continue;
        }

        let mut term_counts = std::collections::HashMap::<String, u32>::new();
        let mut samples = component
            .iter()
            .map(|index| {
                for token in &token_sets[*index] {
                    *term_counts.entry(token.clone()).or_insert(0) += 1;
                }
                to_issue_sample(bug_issues[*index])
            })
            .collect::<Vec<_>>();

        let mut shared_terms = term_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .collect::<Vec<_>>();
        shared_terms.sort_by(|(left_term, left_count), (right_term, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| left_term.cmp(right_term))
        });

        let shared_terms = shared_terms
            .into_iter()
            .map(|(term, _)| term)
            .take(3)
            .collect::<Vec<_>>();

        samples.sort_by(|left, right| {
            right
                .comments
                .cmp(&left.comments)
                .then_with(|| right.age_days.cmp(&left.age_days))
        });

        let label = if shared_terms.is_empty() {
            "Repeated bug pattern".to_string()
        } else {
            shared_terms.join(" / ")
        };

        clusters.push(RecurringBugCluster {
            label,
            issue_count: component.len() as u32,
            shared_terms,
            examples: samples.into_iter().take(3).collect(),
        });
    }

    clusters.sort_by(|left, right| {
        right.issue_count.cmp(&left.issue_count).then_with(|| {
            right
                .examples
                .first()
                .map(|example| example.comments)
                .unwrap_or(0)
                .cmp(
                    &left
                        .examples
                        .first()
                        .map(|example| example.comments)
                        .unwrap_or(0),
                )
        })
    });
    clusters.truncate(3);
    clusters
}

#[cfg(test)]
mod tests {
    use super::{
        duplicate_candidates, priority_score, summary_from_signals, IssueAnalysis,
        SummarySignalInput,
    };
    use crate::models::GitHubIssue;

    fn issue(number: u32, title: &str) -> GitHubIssue {
        GitHubIssue {
            number,
            title: title.into(),
            ..GitHubIssue::default()
        }
    }

    #[test]
    fn sibling_feature_requests_are_not_duplicate_candidates() {
        let issues = vec![
            issue(206, "Add an output filter for bun"),
            issue(203, "Add an output filter for glab"),
            issue(202, "Add an output filter for playwright"),
        ];

        assert!(duplicate_candidates(&issues).is_empty());
    }

    #[test]
    fn strongly_overlapping_issue_titles_remain_duplicate_candidates() {
        let issues = vec![
            issue(41, "Crash when loading workspace configuration"),
            issue(42, "Crash when loading workspace configuration file"),
        ];

        let candidates = duplicate_candidates(&issues);
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].similarity >= 0.72);
    }

    #[test]
    fn generated_evidence_uses_singular_issue_grammar() {
        let analysis = IssueAnalysis {
            sampled_issues: 1,
            stale_issues: 1,
            unlabeled_issues: 1,
            stale_bug_issues: 1,
            stale_high_comment_issues: 1,
            issue_examples: Vec::new(),
            duplicate_candidates: Vec::new(),
            recurring_bug_clusters: Vec::new(),
        };

        let (_, breakdown) = priority_score(100, 1, &analysis, 0, 0);
        assert!(breakdown
            .iter()
            .any(|factor| factor.detail == "1 sampled issue has no labels"));

        let (summary, _) = summary_from_signals(SummarySignalInput {
            stars: 100,
            open_issues: 1,
            issue_analysis: &analysis,
            todo_count: 0,
            fixme_count: 0,
            todo_available: true,
            fixme_available: true,
            repo_warnings: &[],
        });
        assert!(summary.contains("1 stale bug-like issue is still open"));
        assert!(summary.contains("1 of 1 sampled issue looks stale"));
    }
}

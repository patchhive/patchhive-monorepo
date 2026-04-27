use chrono::Utc;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

use crate::github::GitHubReviewContext;
use crate::models::{
    ChecklistEvidence, ChecklistItem, ReviewMetrics, ReviewResult,
    GitHubReviewContext as ReviewTriggerContext,
};

use super::analysis::*;

#[derive(Default)]
pub(crate) struct ChecklistCluster {
    pub(crate) category: String,
    pub(crate) title: String,
    pub(crate) open_threads: u32,
    pub(crate) resolved_threads: u32,
    pub(crate) outdated_threads: u32,
    pub(crate) comment_count: u32,
    pub(crate) path_hints: BTreeSet<String>,
    pub(crate) commenter_logins: BTreeSet<String>,
    pub(crate) evidence: Vec<ChecklistEvidence>,
}

pub fn build_review_result(
    context: &GitHubReviewContext,
    trigger: String,
    event: String,
    action: String,
) -> ReviewResult {
    let created_at = Utc::now().to_rfc3339();
    let mut reviewer_logins = BTreeSet::new();
    let mut requested_changes_reviews = 0u32;
    let mut approval_reviews = 0u32;
    let mut comment_reviews = 0u32;
    let mut clusters: HashMap<String, ChecklistCluster> = HashMap::new();
    let mut actionable_threads = 0u32;

    for review in &context.reviews {
        if !review.author_login.trim().is_empty() {
            reviewer_logins.insert(review.author_login.trim().to_string());
        }

        match review.state.as_str() {
            "CHANGES_REQUESTED" => requested_changes_reviews += 1,
            "APPROVED" => approval_reviews += 1,
            "COMMENTED" => comment_reviews += 1,
            _ => {}
        }

        if review.state != "APPROVED" && actionable_text(&review.body) {
            let path_hint = "general".to_string();
            let (category, _) = classify_category(&review.body);
            let key = format!("{category}:{path_hint}");
            let cluster = clusters.entry(key).or_default();
            cluster.category = category.into();
            if cluster.title.is_empty() {
                cluster.title = checklist_title(category, &path_hint);
            }
            cluster.open_threads += 1;
            cluster.comment_count += 1;
            if !review.author_login.trim().is_empty() {
                cluster
                    .commenter_logins
                    .insert(review.author_login.trim().to_string());
            }
            push_evidence(
                &mut cluster.evidence,
                ChecklistEvidence {
                    author_login: review.author_login.clone(),
                    url: review.html_url.clone(),
                    path: String::new(),
                    excerpt: truncate(&collapse_whitespace(&review.body), 220),
                    resolved: false,
                    outdated: false,
                },
            );
        }
    }

    for thread in &context.threads {
        let actionable_comments = thread
            .comments
            .iter()
            .filter(|comment| actionable_text(&comment.body))
            .collect::<Vec<_>>();

        if actionable_comments.is_empty() {
            continue;
        }

        actionable_threads += 1;
        let combined = actionable_comments
            .iter()
            .map(|comment| comment.body.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let (category, _) = classify_category(&combined);
        let path_hint = if thread.path.trim().is_empty() {
            "general".into()
        } else {
            path_bucket(&thread.path)
        };
        let key = format!("{category}:{path_hint}");
        let cluster = clusters.entry(key).or_default();
        cluster.category = category.into();
        if cluster.title.is_empty() {
            cluster.title = checklist_title(category, &path_hint);
        }
        if thread.is_resolved {
            cluster.resolved_threads += 1;
        } else {
            cluster.open_threads += 1;
        }
        if thread.is_outdated {
            cluster.outdated_threads += 1;
        }
        if path_hint != "general" {
            cluster.path_hints.insert(path_hint.clone());
        }

        for comment in actionable_comments {
            cluster.comment_count += 1;
            if !comment.author_login.trim().is_empty() {
                reviewer_logins.insert(comment.author_login.trim().to_string());
                cluster
                    .commenter_logins
                    .insert(comment.author_login.trim().to_string());
            }
            push_evidence(
                &mut cluster.evidence,
                ChecklistEvidence {
                    author_login: comment.author_login.clone(),
                    url: comment.url.clone(),
                    path: thread.path.clone(),
                    excerpt: truncate(&collapse_whitespace(&comment.body), 220),
                    resolved: thread.is_resolved,
                    outdated: thread.is_outdated,
                },
            );
        }
    }

    let mut checklist = clusters
        .into_iter()
        .map(|(_, cluster)| {
            let category = cluster.category.clone();
            into_checklist_item(category, cluster)
        })
        .collect::<Vec<_>>();
    checklist.sort_by(|left, right| {
        status_rank(&right.status)
            .cmp(&status_rank(&left.status))
            .then_with(|| right.open_threads.cmp(&left.open_threads))
            .then_with(|| right.comment_count.cmp(&left.comment_count))
            .then_with(|| left.title.cmp(&right.title))
    });

    let open_items = checklist
        .iter()
        .filter(|item| item.status == "open" || item.status == "mixed")
        .count() as u32;
    let resolved_items = checklist
        .iter()
        .filter(|item| item.status == "resolved")
        .count() as u32;
    let metrics = ReviewMetrics {
        review_count: context.reviews.len() as u32,
        requested_changes_reviews,
        approval_reviews,
        comment_reviews,
        thread_count: context.threads.len() as u32,
        actionable_threads,
        open_items,
        resolved_items,
        reviewer_count: reviewer_logins.len() as u32,
    };
    let status = overall_status(&metrics, requested_changes_reviews);
    let summary = build_summary(&metrics, &checklist, &status);
    let prompt_suggestions = checklist
        .iter()
        .filter(|item| item.status != "resolved")
        .map(|item| item.prompt_hint.clone())
        .take(4)
        .collect::<Vec<_>>();

    ReviewResult {
        id: Uuid::new_v4().to_string(),
        repo: context.pr.repo.clone(),
        pr_number: context.pr.number,
        pr_title: context.pr.title.clone(),
        pr_url: context.pr.html_url.clone(),
        status,
        summary,
        created_at,
        metrics,
        reviewers: reviewer_logins.into_iter().collect(),
        prompt_suggestions,
        checklist,
        github: Some(ReviewTriggerContext {
            repo: context.pr.repo.clone(),
            pr_number: context.pr.number,
            pr_title: context.pr.title.clone(),
            pr_url: context.pr.html_url.clone(),
            head_sha: context.pr.head_sha.clone(),
            head_ref: context.pr.head_ref.clone(),
            base_ref: context.pr.base_ref.clone(),
            event,
            action,
            trigger,
        }),
        github_report: None,
    }
}

pub(crate) fn into_checklist_item(category: String, cluster: ChecklistCluster) -> ChecklistItem {
    let mut path_hints = cluster.path_hints.into_iter().collect::<Vec<_>>();
    path_hints.sort();
    let mut commenter_logins = cluster.commenter_logins.into_iter().collect::<Vec<_>>();
    commenter_logins.sort();
    let status = if cluster.open_threads == 0 {
        "resolved"
    } else if cluster.resolved_threads > 0 || cluster.outdated_threads > 0 {
        "mixed"
    } else {
        "open"
    };
    let prompt_hint = prompt_hint(&category, &path_hints);
    let summary = checklist_summary(
        &cluster.title,
        status,
        cluster.open_threads,
        cluster.resolved_threads,
        cluster.outdated_threads,
        cluster.comment_count,
        &path_hints,
    );

    ChecklistItem {
        key: format!("{}:{}", category, path_hints.join("+")),
        title: cluster.title,
        category,
        status: status.into(),
        summary,
        prompt_hint,
        path_hints,
        commenter_logins,
        open_threads: cluster.open_threads,
        resolved_threads: cluster.resolved_threads,
        outdated_threads: cluster.outdated_threads,
        comment_count: cluster.comment_count,
        evidence: cluster.evidence,
    }
}

pub(crate) fn checklist_summary(
    title: &str,
    status: &str,
    open_threads: u32,
    resolved_threads: u32,
    outdated_threads: u32,
    comment_count: u32,
    path_hints: &[String],
) -> String {
    let mut parts = Vec::new();
    parts.push(match status {
        "resolved" => format!("{title} looks resolved right now."),
        "mixed" => format!("{title} still has some active follow-up."),
        _ => format!("{title} is still blocking clean review closure."),
    });
    parts.push(format!(
        "{} actionable comment{} across {} open thread{}, {} resolved thread{}",
        comment_count,
        plural_suffix(comment_count),
        open_threads,
        plural_suffix(open_threads),
        resolved_threads,
        plural_suffix(resolved_threads),
    ));
    if outdated_threads > 0 {
        parts.push(format!(
            "{} outdated thread{} still shape the context.",
            outdated_threads,
            plural_suffix(outdated_threads)
        ));
    }
    if !path_hints.is_empty() {
        parts.push(format!("Focus areas: {}.", path_hints.join(", ")));
    }
    parts.join(" ")
}

pub(crate) fn overall_status(metrics: &ReviewMetrics, requested_changes_reviews: u32) -> String {
    if metrics.open_items == 0 && metrics.actionable_threads == 0 {
        return "clear".into();
    }
    if metrics.open_items == 0 {
        return "resolved".into();
    }
    if requested_changes_reviews > 0 || metrics.open_items >= 3 {
        return "attention".into();
    }
    "follow-up".into()
}

pub(crate) fn build_summary(metrics: &ReviewMetrics, checklist: &[ChecklistItem], status: &str) -> String {
    if checklist.is_empty() {
        return "ReviewBee did not find actionable review feedback in the current PR threads. This PR looks close to merge from a comment-clustering perspective.".into();
    }

    let mut category_counts = HashMap::new();
    for item in checklist.iter().filter(|item| item.status != "resolved") {
        *category_counts
            .entry(item.category.as_str())
            .or_insert(0u32) += 1;
    }
    let mut top_categories = category_counts.into_iter().collect::<Vec<_>>();
    top_categories.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    let top_categories = top_categories
        .into_iter()
        .take(3)
        .map(|(category, _)| category_label(category))
        .collect::<Vec<_>>();

    let status_lead = match status {
        "clear" => "The active review queue looks clear.",
        "resolved" => "Most actionable feedback appears resolved already.",
        "attention" => "This PR still needs focused follow-up before it feels merge-ready.",
        _ => "This PR still has a manageable but real review follow-up list.",
    };

    format!(
        "{} {} open checklist item{} across {} actionable thread{} from {} reviewer{}. Biggest themes: {}.",
        status_lead,
        metrics.open_items,
        plural_suffix(metrics.open_items),
        metrics.actionable_threads,
        plural_suffix(metrics.actionable_threads),
        metrics.reviewer_count,
        plural_suffix(metrics.reviewer_count),
        if top_categories.is_empty() {
            "general review follow-up".into()
        } else {
            top_categories.join(", ")
        }
    )
}

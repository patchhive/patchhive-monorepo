export const SORT_OPTIONS = [
  { v: "priority", l: "Priority Score" },
  { v: "stale", l: "Most Stale Issues" },
  { v: "recurring", l: "Most Recurring Bugs" },
  { v: "triage", l: "Most Unlabeled" },
  { v: "duplicates", l: "Most Duplicate Pairs" },
  { v: "markers", l: "Most TODO / FIXME" },
];

export function sortRepos(repos, sortBy) {
  const items = [...(repos || [])];

  items.sort((left, right) => {
    switch (sortBy) {
      case "stale":
        return (
          right.stale_issues - left.stale_issues ||
          right.stale_bug_issues - left.stale_bug_issues ||
          right.priority_score - left.priority_score
        );
      case "recurring":
        return (
          right.recurring_bug_clusters.reduce((sum, cluster) => sum + cluster.issue_count, 0) -
            left.recurring_bug_clusters.reduce((sum, cluster) => sum + cluster.issue_count, 0) ||
          right.recurring_bug_clusters.length - left.recurring_bug_clusters.length ||
          right.priority_score - left.priority_score
        );
      case "triage":
        return (
          right.unlabeled_issues - left.unlabeled_issues ||
          right.stale_issues - left.stale_issues ||
          right.priority_score - left.priority_score
        );
      case "duplicates":
        return (
          right.duplicate_candidates.length - left.duplicate_candidates.length ||
          right.priority_score - left.priority_score
        );
      case "markers":
        return (
          right.todo_count + right.fixme_count - (left.todo_count + left.fixme_count) ||
          right.priority_score - left.priority_score
        );
      default:
        return (
          right.priority_score - left.priority_score ||
          right.stale_issues - left.stale_issues ||
          right.stars - left.stars
        );
    }
  });

  return items;
}

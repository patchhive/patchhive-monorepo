import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "./config.js";

export function markerCountsLabel(repo) {
  const parts = [];
  if (repo.stale_issues) parts.push(`${repo.stale_issues} stale`);
  if (repo.todo_count) parts.push(`${repo.todo_count} TODO`);
  if (repo.duplicate_candidates?.length) parts.push(`${repo.duplicate_candidates.length} dupes`);
  if (repo.recurring_bug_clusters?.length) parts.push(`${repo.recurring_bug_clusters.length} bug clusters`);
  return parts.join(", ");
}

export function summarizeScanHighlights(scan) {
  if (!scan?.repos) return [];
  const highlights = [];
  let totalStale = 0, totalTodos = 0, totalDupes = 0, totalBugs = 0;
  for (const repo of scan.repos) {
    totalStale += repo.stale_issues || 0;
    totalTodos += repo.todo_count || 0;
    totalDupes += repo.duplicate_candidates?.length || 0;
    totalBugs += repo.recurring_bug_clusters?.length || 0;
  }
  if (totalStale) highlights.push(`${totalStale} stale issues`);
  if (totalTodos) highlights.push(`${totalTodos} TODO/FIXME markers`);
  if (totalDupes) highlights.push(`${totalDupes} duplicate groups`);
  if (totalBugs) highlights.push(`${totalBugs} bug clusters`);
  return highlights;
}

export function buildDashboardSummary(scan) {
  if (!scan) return "";
  const lines = [`# SignalHive Scan — ${scan.id || "unknown"}`, "", "## Repos Scanned"];
  const stats = { repos: 0, stale: 0, todos: 0, dupes: 0, bugs: 0 };
  for (const repo of scan.repos || []) {
    stats.repos++;
    stats.stale += repo.stale_issues || 0;
    stats.todos += repo.todo_count || 0;
    stats.dupes += repo.duplicate_candidates?.length || 0;
    stats.bugs += repo.recurring_bug_clusters?.length || 0;
    lines.push(`- **${repo.full_name}** — ${repo.stale_issues || 0} stale, ${repo.todo_count || 0} TODOs`);
  }
  lines.push("", "## Summary", `- **${stats.repos}** repos scanned`, `- **${stats.stale}** stale issues`, `- **${stats.todos}** TODO markers`, `- **${stats.dupes}** duplicate groups`, `- **${stats.bugs}** bug clusters`);
  return lines.join("\n");
}

export function downloadTextFile(filename, content, mimeType = "text/plain") {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export function timeAgo(dateValue) {
  if (!dateValue) return "never";
  const now = Date.now();
  const then = new Date(dateValue).getTime();
  if (isNaN(then)) return dateValue;
  const diff = now - then;
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return new Date(dateValue).toLocaleDateString();
}

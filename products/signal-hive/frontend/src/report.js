export function downloadTextFile(filename, content, mimeType) {
  const blob = new Blob([content], { type: mimeType });
  const href = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = href;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(href);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

export function buildDashboardSummary(scan) {
  const topRepo = scan?.repos?.[0];
  const topRecurring = [...(scan?.repos || [])].sort(
    (left, right) =>
      right.recurring_bug_clusters.reduce((sum, cluster) => sum + cluster.issue_count, 0) -
      left.recurring_bug_clusters.reduce((sum, cluster) => sum + cluster.issue_count, 0),
  )[0];
  const topStale = [...(scan?.repos || [])].sort(
    (left, right) => right.stale_issues - left.stale_issues,
  )[0];

  const lines = [
    `SignalHive scan ${scan.id}`,
    `${scan.summary.total_repos} repos scanned, ${scan.summary.total_signals} signals found.`,
    topRepo ? `Top repo: ${topRepo.full_name} (${Math.round(topRepo.priority_score)} priority).` : null,
    topStale ? `Most stale backlog: ${topStale.full_name} (${topStale.stale_issues} stale issues).` : null,
    topRecurring
      ? `Strongest recurring bug pressure: ${topRecurring.full_name} (${topRecurring.recurring_bug_clusters.length} clusters).`
      : null,
    scan.trend
      ? `Compared to the previous similar scan: signals ${scan.trend.total_signals_delta >= 0 ? "+" : ""}${scan.trend.total_signals_delta}, rising repos ${scan.trend.rising_repos}, improving repos ${scan.trend.improving_repos}.`
      : null,
  ].filter(Boolean);

  return lines.join(" ");
}

export function buildDashboardHtml(scan, timeline) {
  const topRepos = (scan?.repos || []).slice(0, 5);
  const timelineRows = (timeline?.points || [])
    .map(
      (point) => `
        <tr>
          <td>${escapeHtml(new Date(point.created_at).toLocaleString())}</td>
          <td>${escapeHtml(point.trigger_type || "manual")}</td>
          <td>${point.total_repos}</td>
          <td>${point.total_signals}</td>
          <td>${point.total_stale_issues}</td>
          <td>${point.avg_priority_score.toFixed(1)}</td>
        </tr>`,
    )
    .join("");

  const repoRows = topRepos
    .map(
      (repo) => `
        <tr>
          <td>${escapeHtml(repo.full_name)}</td>
          <td>${Math.round(repo.priority_score)}</td>
          <td>${repo.stale_issues}</td>
          <td>${repo.duplicate_candidates.length}</td>
          <td>${repo.recurring_bug_clusters.length}</td>
          <td>${repo.todo_count + repo.fixme_count}</td>
        </tr>`,
    )
    .join("");

  const trendSummary = scan.trend
    ? `
      <div class="card">
        <h2>Trend vs previous similar scan</h2>
        <p>Signals ${scan.trend.total_signals_delta >= 0 ? "+" : ""}${scan.trend.total_signals_delta}, repos ${scan.trend.total_repos_delta >= 0 ? "+" : ""}${scan.trend.total_repos_delta}</p>
        <p>${scan.trend.new_repos} new, ${scan.trend.dropped_repos} dropped, ${scan.trend.rising_repos} rising, ${scan.trend.improving_repos} improving.</p>
      </div>`
    : "";

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>SignalHive Dashboard ${escapeHtml(scan.id)}</title>
  <style>
    :root {
      --bg: #0b101a;
      --panel: #111827;
      --text: #e8eef8;
      --muted: #9fb1c7;
      --border: #243245;
      --accent: #2a6aaa;
      --gold: #d6a756;
      --green: #29a36a;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      padding: 32px;
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      background: radial-gradient(circle at top, #15253d, var(--bg) 50%);
      color: var(--text);
    }
    .wrap { display: grid; gap: 16px; max-width: 1120px; margin: 0 auto; }
    .card {
      background: color-mix(in srgb, var(--panel) 88%, black);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 18px;
    }
    .hero { display: flex; justify-content: space-between; gap: 16px; flex-wrap: wrap; }
    .stats { display: flex; gap: 12px; flex-wrap: wrap; }
    .stat { padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px; min-width: 120px; }
    .label { color: var(--muted); font-size: 12px; margin-bottom: 6px; }
    .value { font-size: 24px; font-weight: 700; }
    h1, h2 { margin: 0 0 10px; }
    p { margin: 0; line-height: 1.6; color: var(--muted); }
    table { width: 100%; border-collapse: collapse; margin-top: 12px; }
    th, td { text-align: left; padding: 10px 8px; border-bottom: 1px solid var(--border); font-size: 13px; }
    th { color: var(--muted); font-weight: 600; }
  </style>
</head>
<body>
  <div class="wrap">
    <div class="card hero">
      <div>
        <h1>SignalHive Dashboard</h1>
        <p>Scan ${escapeHtml(scan.id)} • ${escapeHtml(new Date(scan.created_at).toLocaleString())}</p>
        <p>${escapeHtml(buildDashboardSummary(scan))}</p>
      </div>
      <div class="stats">
        <div class="stat"><div class="label">Repos</div><div class="value">${scan.summary.total_repos}</div></div>
        <div class="stat"><div class="label">Signals</div><div class="value">${scan.summary.total_signals}</div></div>
        <div class="stat"><div class="label">Top Repo</div><div class="value" style="font-size:16px;">${escapeHtml(scan.summary.top_repo)}</div></div>
      </div>
    </div>
    ${trendSummary}
    <div class="card">
      <h2>Timeline</h2>
      <table>
        <thead>
          <tr>
            <th>Scan</th>
            <th>Trigger</th>
            <th>Repos</th>
            <th>Signals</th>
            <th>Stale Issues</th>
            <th>Avg Priority</th>
          </tr>
        </thead>
        <tbody>${timelineRows}</tbody>
      </table>
    </div>
    <div class="card">
      <h2>Top Queue</h2>
      <table>
        <thead>
          <tr>
            <th>Repo</th>
            <th>Priority</th>
            <th>Stale</th>
            <th>Duplicates</th>
            <th>Recurring</th>
            <th>Markers</th>
          </tr>
        </thead>
        <tbody>${repoRows}</tbody>
      </table>
    </div>
  </div>
</body>
</html>`;
}

export function exportDashboardHtml(scan, timeline) {
  const filename = `signalhive-dashboard-${scan.id.slice(0, 8)}.html`;
  downloadTextFile(filename, buildDashboardHtml(scan, timeline), "text/html;charset=utf-8");
}

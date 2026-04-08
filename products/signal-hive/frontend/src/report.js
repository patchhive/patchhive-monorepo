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

function fmtSigned(value, digits = 0) {
  const rounded = digits > 0 ? Number(value).toFixed(digits) : `${Math.round(value)}`;
  return `${value >= 0 ? "+" : ""}${rounded}`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

export function summarizeScanHighlights(scan) {
  const topRepo = scan?.repos?.[0];
  const topRecurring = [...(scan?.repos || [])].sort(
    (left, right) =>
      right.recurring_bug_clusters.reduce((sum, cluster) => sum + cluster.issue_count, 0) -
      left.recurring_bug_clusters.reduce((sum, cluster) => sum + cluster.issue_count, 0),
  )[0];
  const topStale = [...(scan?.repos || [])].sort(
    (left, right) => right.stale_issues - left.stale_issues,
  )[0];
  const topDuplicates = [...(scan?.repos || [])].sort(
    (left, right) => right.duplicate_candidates.length - left.duplicate_candidates.length,
  )[0];
  const topMarkers = [...(scan?.repos || [])].sort(
    (left, right) => (right.todo_count + right.fixme_count) - (left.todo_count + left.fixme_count),
  )[0];
  const risingRepos = (scan?.repos || []).filter((repo) => repo.trend?.status === "rising");
  const improvingRepos = (scan?.repos || []).filter((repo) => repo.trend?.status === "improving");

  return {
    topRepo,
    topRecurring,
    topStale,
    topDuplicates,
    topMarkers,
    risingRepos,
    improvingRepos,
  };
}

export function buildDashboardSummary(scan) {
  const {
    topRepo,
    topRecurring,
    topStale,
    risingRepos,
    improvingRepos,
  } = summarizeScanHighlights(scan);

  const lines = [
    `SignalHive scan ${scan.id.slice(0, 8)} scanned ${scan.summary.total_repos} repos and surfaced ${scan.summary.total_signals} maintenance signals.`,
    topRepo ? `Highest-risk repo: ${topRepo.full_name} at ${Math.round(topRepo.priority_score)} priority.` : null,
    topStale ? `Largest stale backlog spike: ${topStale.full_name} with ${topStale.stale_issues} stale issues.` : null,
    topRecurring
      ? `Strongest recurring bug pressure: ${topRecurring.full_name} with ${topRecurring.recurring_bug_clusters.length} recurring clusters.`
      : null,
    scan.trend
      ? `Compared to the previous similar scan: signals ${fmtSigned(scan.trend.total_signals_delta)}, rising repos ${risingRepos.length}, improving repos ${improvingRepos.length}.`
      : null,
  ].filter(Boolean);

  return lines.join(" ");
}

export function buildDashboardHtml(scan, timeline) {
  const {
    topRepo,
    topRecurring,
    topStale,
    topDuplicates,
    topMarkers,
    risingRepos,
    improvingRepos,
  } = summarizeScanHighlights(scan);
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

  const moverList = (repos, fallback) =>
    repos.length > 0
      ? repos
          .slice(0, 4)
          .map(
            (repo) => `<li><strong>${escapeHtml(repo.full_name)}</strong> <span>${repo.trend ? fmtSigned(repo.trend.priority_delta, 1) : ""}</span></li>`,
          )
          .join("")
      : `<li class="muted">${escapeHtml(fallback)}</li>`;

  const trendSummary = scan.trend
    ? `
      <div class="card">
        <h2>Trend vs previous similar scan</h2>
        <div class="pill-row">
          <span class="pill ${scan.trend.total_signals_delta > 0 ? "hot" : scan.trend.total_signals_delta < 0 ? "cool" : "neutral"}">Signals ${fmtSigned(scan.trend.total_signals_delta)}</span>
          <span class="pill ${scan.trend.total_repos_delta > 0 ? "hot" : scan.trend.total_repos_delta < 0 ? "cool" : "neutral"}">Repos ${fmtSigned(scan.trend.total_repos_delta)}</span>
          <span class="pill hot">${scan.trend.rising_repos} rising</span>
          <span class="pill cool">${scan.trend.improving_repos} improving</span>
          <span class="pill neutral">${scan.trend.steady_repos} steady</span>
        </div>
        <p>Compared to ${escapeHtml(new Date(scan.trend.compared_to_created_at).toLocaleString())}, this queue has ${scan.trend.new_repos} new repos and ${scan.trend.dropped_repos} dropped repos.</p>
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
      --bg: #08101a;
      --panel: rgba(12, 22, 36, 0.9);
      --panel-2: rgba(17, 29, 47, 0.92);
      --text: #edf4ff;
      --muted: #9fb2ca;
      --border: rgba(74, 106, 150, 0.28);
      --accent: #2a6aaa;
      --gold: #d6a756;
      --green: #29a36a;
      --glow: rgba(42, 106, 170, 0.14);
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      padding: 32px;
      font-family: "IBM Plex Mono", ui-monospace, SFMono-Regular, Menlo, monospace;
      background:
        radial-gradient(circle at top, rgba(60, 105, 170, 0.26), transparent 36%),
        linear-gradient(180deg, #0e1a2d 0%, var(--bg) 48%);
      color: var(--text);
    }
    .wrap { display: grid; gap: 16px; max-width: 1120px; margin: 0 auto; }
    .card {
      background: linear-gradient(180deg, var(--panel), var(--panel-2));
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 20px;
      box-shadow: 0 18px 50px rgba(0, 0, 0, 0.22);
    }
    .hero { display: flex; justify-content: space-between; gap: 18px; flex-wrap: wrap; position: relative; overflow: hidden; }
    .hero::after {
      content: "";
      position: absolute;
      inset: 0;
      background: radial-gradient(circle at top right, var(--glow), transparent 32%);
      pointer-events: none;
    }
    .stats { display: flex; gap: 12px; flex-wrap: wrap; }
    .stat { padding: 12px 14px; border: 1px solid var(--border); border-radius: 12px; min-width: 132px; background: rgba(8, 15, 25, 0.38); }
    .label { color: var(--muted); font-size: 12px; margin-bottom: 6px; }
    .value { font-size: 24px; font-weight: 700; }
    h1, h2, h3 { margin: 0 0 10px; }
    h1 { font-size: 34px; letter-spacing: -0.04em; }
    p { margin: 0; line-height: 1.7; color: var(--muted); }
    .eyebrow { color: var(--accent); font-size: 11px; letter-spacing: 0.18em; text-transform: uppercase; margin-bottom: 10px; }
    .grid { display: grid; gap: 16px; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); }
    .pill-row { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; }
    .pill {
      display: inline-flex;
      border: 1px solid var(--border);
      border-radius: 999px;
      padding: 5px 10px;
      font-size: 11px;
      color: var(--text);
      background: rgba(8, 15, 25, 0.38);
    }
    .pill.hot { color: #ff9ba8; border-color: rgba(196, 30, 58, 0.35); }
    .pill.cool { color: #8ed7aa; border-color: rgba(41, 163, 106, 0.35); }
    .pill.neutral { color: #ffd591; border-color: rgba(214, 167, 86, 0.35); }
    .section-title { display: flex; justify-content: space-between; gap: 12px; align-items: center; margin-bottom: 12px; }
    table { width: 100%; border-collapse: collapse; margin-top: 12px; }
    th, td { text-align: left; padding: 10px 8px; border-bottom: 1px solid var(--border); font-size: 13px; }
    th { color: var(--muted); font-weight: 600; }
    ul { margin: 0; padding-left: 18px; }
    li { margin: 8px 0; color: var(--text); }
    li span { color: var(--muted); }
    .muted { color: var(--muted); }
    .brand {
      display: inline-flex;
      gap: 8px;
      align-items: center;
      padding: 6px 10px;
      border: 1px solid var(--border);
      border-radius: 999px;
      color: var(--accent);
      font-size: 11px;
      margin-bottom: 12px;
      background: rgba(8, 15, 25, 0.38);
    }
    @media print {
      body { padding: 0; background: #fff; color: #111; }
      .card { break-inside: avoid; box-shadow: none; background: #fff; border-color: #d7dfeb; }
      p, .label, th { color: #445; }
    }
  </style>
</head>
<body>
  <div class="wrap">
    <div class="card hero">
      <div>
        <div class="brand">📡 SignalHive by PatchHive</div>
        <div class="eyebrow">Maintenance Reconnaissance</div>
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
    <div class="grid">
      <div class="card">
        <div class="section-title"><h2>Pressure Map</h2><span class="pill neutral">Executive readout</span></div>
        <ul>
          <li><strong>Top queue leader:</strong> ${topRepo ? `${escapeHtml(topRepo.full_name)} at ${Math.round(topRepo.priority_score)} priority` : '<span>No top repo</span>'}</li>
          <li><strong>Most stale backlog:</strong> ${topStale ? `${escapeHtml(topStale.full_name)} with ${topStale.stale_issues} stale issues` : '<span>No stale backlog spike</span>'}</li>
          <li><strong>Recurring bug pressure:</strong> ${topRecurring ? `${escapeHtml(topRecurring.full_name)} with ${topRecurring.recurring_bug_clusters.length} clusters` : '<span>No major recurring cluster</span>'}</li>
          <li><strong>Duplicate issue pressure:</strong> ${topDuplicates ? `${escapeHtml(topDuplicates.full_name)} with ${topDuplicates.duplicate_candidates.length} likely duplicate pairs` : '<span>No duplicate hotspot</span>'}</li>
          <li><strong>Code marker pressure:</strong> ${topMarkers ? `${escapeHtml(topMarkers.full_name)} with ${topMarkers.todo_count + topMarkers.fixme_count} TODO/FIXME markers` : '<span>No marker hotspot</span>'}</li>
        </ul>
      </div>
      <div class="card">
        <div class="section-title"><h2>Queue Movement</h2><span class="pill hot">${risingRepos.length} rising</span></div>
        <div class="grid" style="grid-template-columns: 1fr 1fr;">
          <div>
            <h3>Rising</h3>
            <ul>${moverList(risingRepos, "No sharply rising repos in this scan.")}</ul>
          </div>
          <div>
            <h3>Improving</h3>
            <ul>${moverList(improvingRepos, "No strong improvement movement yet.")}</ul>
          </div>
        </div>
      </div>
    </div>
    <div class="card">
      <div class="section-title"><h2>Timeline</h2><span class="pill neutral">${timeline?.points?.length || 0} matching scans</span></div>
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
      <div class="section-title"><h2>Top Queue</h2><span class="pill hot">${topRepos.length} repos shown</span></div>
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

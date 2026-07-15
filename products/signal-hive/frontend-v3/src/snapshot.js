function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function markerCount(repo) {
  return Number(repo.todo_count || 0) + Number(repo.fixme_count || 0);
}

function safeHttpUrl(value) {
  try {
    const url = new URL(String(value || ""));
    return url.protocol === "https:" || url.protocol === "http:" ? url.href : "#";
  } catch {
    return "#";
  }
}

function repositoryRows(repositories) {
  if (!repositories.length) return '<tr><td colspan="7">No repositories were returned.</td></tr>';
  return repositories.map((repo) => `
    <tr>
      <td><a href="${escapeHtml(safeHttpUrl(repo.repo_url))}">${escapeHtml(repo.full_name)}</a></td>
      <td>${Math.round(Number(repo.priority_score || 0))}</td>
      <td>${Number(repo.stale_issues || 0)}</td>
      <td>${repo.duplicate_candidates?.length || 0}</td>
      <td>${repo.recurring_bug_clusters?.length || 0}</td>
      <td>${markerCount(repo)}</td>
      <td>${escapeHtml(repo.summary)}</td>
    </tr>`).join("");
}

function timelineRows(points) {
  if (!points.length) return '<tr><td colspan="5">No comparable scans were available.</td></tr>';
  return points.map((point) => `
    <tr>
      <td>${escapeHtml(new Date(point.created_at).toLocaleString())}</td>
      <td>${escapeHtml(point.trigger_type || "operator")}</td>
      <td>${Number(point.total_repos || 0)}</td>
      <td>${Number(point.total_signals || 0)}</td>
      <td>${escapeHtml(point.top_repo || "none")}</td>
    </tr>`).join("");
}

export function buildDashboardSnapshot(scan, timeline, scopeLabel) {
  const repositories = scan?.repos || [];
  const warnings = scan?.warnings || [];
  const generatedAt = new Date().toLocaleString();
  const warningMarkup = warnings.length
    ? `<section class="warning"><h2>Coverage warnings</h2><ul>${warnings.map((warning) => `<li>${escapeHtml(warning)}</li>`).join("")}</ul></section>`
    : "";

  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>SignalHive snapshot · ${escapeHtml(scopeLabel)}</title>
<style>
:root{color-scheme:light dark;--bg:#f5f3ee;--panel:#fffdf8;--text:#17202a;--muted:#64717d;--line:#d8d4ca;--accent:#2563eb;--warning:#92400e}
@media(prefers-color-scheme:dark){:root{--bg:#090d12;--panel:#11171d;--text:#edf3f8;--muted:#91a0ae;--line:#28323b;--accent:#60a5fa;--warning:#fbbf24}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:14px/1.5 ui-sans-serif,system-ui,sans-serif}.wrap{max-width:1180px;margin:auto;padding:32px 20px 48px}header,section{background:var(--panel);border:1px solid var(--line);border-radius:16px;padding:22px;margin-bottom:16px}h1{font-size:30px;margin:4px 0 8px}h2{font-size:17px;margin:0 0 12px}.eyebrow{color:var(--accent);font-size:11px;font-weight:700;letter-spacing:.14em;text-transform:uppercase}.muted{color:var(--muted)}.metrics{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:10px}.metric{border:1px solid var(--line);border-radius:12px;padding:12px}.metric strong{display:block;font-size:23px;margin-top:3px}table{width:100%;border-collapse:collapse}th,td{text-align:left;vertical-align:top;border-bottom:1px solid var(--line);padding:10px 8px}th{color:var(--muted);font-size:10px;letter-spacing:.1em;text-transform:uppercase}a{color:var(--accent)}.warning{border-color:color-mix(in srgb,var(--warning) 45%,var(--line))}.warning h2{color:var(--warning)}footer{display:flex;justify-content:space-between;gap:16px;color:var(--muted);font-size:12px;padding:8px 4px}@media(max-width:720px){.table-wrap{overflow:auto}table{min-width:760px}}
</style></head><body><main class="wrap">
<header><div class="eyebrow">SignalHive by PatchHive</div><h1>${escapeHtml(scopeLabel)}</h1><div class="muted">Read-only maintenance reconnaissance · scan ${escapeHtml(scan.id)} · generated ${escapeHtml(generatedAt)}</div></header>
<section><h2>Scan summary</h2><div class="metrics">
<div class="metric"><span class="muted">Repositories</span><strong>${scan.summary?.total_repos || 0}</strong></div>
<div class="metric"><span class="muted">Signals</span><strong>${scan.summary?.total_signals || 0}</strong></div>
<div class="metric"><span class="muted">Warnings</span><strong>${warnings.length}</strong></div>
<div class="metric"><span class="muted">Trigger</span><strong>${escapeHtml(scan.trigger_type || "operator")}</strong></div>
</div></section>
${warningMarkup}
<section><h2>Ranked repository queue</h2><div class="table-wrap"><table><thead><tr><th>Repository</th><th>Priority</th><th>Stale</th><th>Duplicates</th><th>Recurring</th><th>Markers</th><th>Summary</th></tr></thead><tbody>${repositoryRows(repositories)}</tbody></table></div></section>
<section><h2>Comparable scan timeline</h2><div class="table-wrap"><table><thead><tr><th>Recorded</th><th>Trigger</th><th>Repos</th><th>Signals</th><th>Top repo</th></tr></thead><tbody>${timelineRows(timeline?.points || [])}</tbody></table></div></section>
<footer><span>SignalHive by <a href="https://github.com/patchhive">PatchHive</a></span><span>Autonomous maintenance suite</span></footer>
</main></body></html>`;
}

export function downloadDashboardSnapshot(scan, timeline, scopeLabel) {
  const blob = new Blob([buildDashboardSnapshot(scan, timeline, scopeLabel)], { type: "text/html;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `signalhive-snapshot-${String(scan.id).slice(0, 8)}.html`;
  link.click();
  URL.revokeObjectURL(url);
}

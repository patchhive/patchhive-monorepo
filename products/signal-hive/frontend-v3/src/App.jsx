import { useEffect, useMemo, useState } from "react";
import { Database, GitBranch, Radar, ShieldCheck, TrendingUp, Workflow } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  ActivityTimeline,
  CopyMarkdownButton,
  GitHubPermissionGuidance,
  IntegratedProductApp,
  ProductLoginScreen,
  ProductShell,
  ScanWarnings,
  V3_TEXT,
  countLabel,
} from "@patchhivehq/ui-v3";
import ControlsPanel from "./ControlsPanel.jsx";
import { API } from "./config.js";
import { downloadDashboardSnapshot } from "./snapshot.js";

const CHIP_TONES = {
  hot: "border-red-900/30 bg-red-900/10 text-red-800 dark:border-red-400/25 dark:bg-red-500/10 dark:text-red-300",
  warn: "border-amber-900/30 bg-amber-900/10 text-amber-800 dark:border-amber-400/25 dark:bg-amber-500/10 dark:text-amber-300",
  ok: "border-emerald-900/30 bg-emerald-900/10 text-emerald-800 dark:border-emerald-400/25 dark:bg-emerald-500/10 dark:text-emerald-300",
  neutral: "border-stone-800/20 bg-stone-800/5 text-stone-700 dark:border-stone-400/20 dark:bg-stone-400/5 dark:text-stone-300",
};

function Chip({ children, tone = "neutral" }) {
  return <span className={`inline-flex items-center justify-center rounded-full border px-2.5 py-1 text-[10px] leading-none uppercase tracking-wider ${CHIP_TONES[tone]}`}>{children}</span>;
}

function Fact({ detail, label, value }) {
  return <div className="surface-inset rounded-xl p-3"><div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</div><div className={`mt-1 font-display text-[18px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value ?? 0}</div>{detail ? <div className={`mt-1 text-[9px] ${V3_TEXT.mute}`}>{detail}</div> : null}</div>;
}

function toList(value) {
  return String(value || "").split(/[\n,]/).map((entry) => entry.trim()).filter(Boolean);
}

function storedTarget(searchQuery = "") {
  const value = String(searchQuery).trim();
  return value.startsWith("repo:") ? value.slice(5).trim() : "";
}

function triggerMode(value) {
  if (value === "manual") return "operator";
  if (value === "scheduled") return "schedule";
  return value || "operator";
}

function triggerLabel(value) {
  return triggerMode(value) === "schedule" ? "scheduled" : triggerMode(value) === "operator" ? "operator run" : triggerMode(value);
}

function targetSelectionMode(value) {
  return value?.target_selection_mode || (storedTarget(value?.search_query || value?.params?.search_query) ? "direct" : "discovery");
}

function toFormParams(params = {}) {
  const targetRepo = storedTarget(params.search_query);
  return {
    target_repo: targetRepo,
    search_query: targetRepo ? "" : params.search_query || "",
    topics: (params.topics || []).join(", "),
    languages: (params.languages || []).join(", "),
    min_stars: String(params.min_stars ?? 25),
    max_repos: String(params.max_repos ?? 8),
    issues_per_repo: String(params.issues_per_repo ?? 30),
    stale_days: String(params.stale_days ?? 45),
  };
}

function serialize(form) {
  const targetRepo = form.target_repo?.trim() || "";
  return {
    search_query: targetRepo ? `repo:${targetRepo}` : form.search_query?.trim() || "",
    topics: toList(form.topics),
    languages: toList(form.languages),
    min_stars: Number(form.min_stars),
    max_repos: Number(form.max_repos),
    issues_per_repo: Number(form.issues_per_repo),
    stale_days: Number(form.stale_days),
  };
}

function markerCount(repo) {
  return Number(repo.todo_count || 0) + Number(repo.fixme_count || 0);
}

function markerEvidence(repo) {
  const availableFeeds = Number(Boolean(repo.todo_available)) + Number(Boolean(repo.fixme_available));
  if (!availableFeeds) return "marker search unavailable";
  return `${markerCount(repo)} observed markers${availableFeeds < 2 ? " · partial coverage" : ""}`;
}

function duplicateCount(repo) {
  return repo.duplicate_candidates?.length || 0;
}

function recurringCount(repo) {
  return repo.recurring_bug_clusters?.length || 0;
}

function scoreTone(score) {
  if (Number(score) >= 75) return "hot";
  if (Number(score) >= 45) return "warn";
  return "ok";
}

function trendTone(status) {
  if (status === "rising" || status === "new") return "hot";
  if (status === "improving") return "ok";
  return "neutral";
}

function trendLabel(repo) {
  return repo.trend?.status || "baseline";
}

function scanScope(scan, fallback = "discovery scope") {
  if (!scan?.params) return fallback;
  const targetRepo = storedTarget(scan.params.search_query);
  if (targetRepo) return targetRepo;
  const parts = [scan.params.search_query, ...(scan.params.topics || []).map((topic) => `#${topic}`), ...(scan.params.languages || [])].filter(Boolean);
  return parts.length ? parts.join(" · ") : "allowlisted repositories";
}

function buildScanMarkdown(scan) {
  if (!scan) return "";
  const signalCount = scan.summary?.total_signals || 0;
  const repositoryCount = scan.summary?.total_repos || 0;
  const lines = [
    `# SignalHive scan · ${scanScope(scan)}`,
    "",
    `SignalHive found **${countLabel(signalCount, "maintenance signal")}** across **${countLabel(repositoryCount, "repository")}**.`,
    "",
    `- Scan: ${scan.id}`,
    `- Trigger: ${triggerMode(scan.trigger_type)}`,
    `- Top repository: ${scan.summary?.top_repo || "none"}`,
    `- Warnings: ${scan.warnings?.length || 0}`,
  ];
  if (scan.repos?.length) {
    lines.push("", "## Ranked repositories", "");
    scan.repos.forEach((repo) => lines.push(`- **${repo.full_name}** — ${Math.round(repo.priority_score || 0)}/100 · ${repo.stale_issues || 0} stale · ${duplicateCount(repo)} duplicate candidates · ${markerEvidence(repo)}`));
  }
  if (scan.warnings?.length) {
    lines.push("", "## Scan warnings", "", ...scan.warnings.map((warning) => `- ${warning}`));
  }
  lines.push("", "*SignalHive by [PatchHive](https://github.com/patchhive)*");
  return lines.join("\n");
}

function repoEvidence(repo) {
  return [
    ...(repo.signals || []),
    ...(repo.warnings || []).map((warning) => `Coverage warning: ${warning}`),
    ...(repo.score_breakdown || []).map((factor) => `${factor.label}: ${factor.detail} (${factor.impact >= 0 ? "+" : ""}${factor.impact})`),
    ...(repo.issue_examples || []).map((issue) => `Issue #${issue.number}: ${issue.title} · ${issue.age_days} days old · ${issue.comments} comments`),
    ...(repo.duplicate_candidates || []).map((candidate) => `Possible duplicate: #${candidate.left_number} ${candidate.left_title} ↔ #${candidate.right_number} ${candidate.right_title} · ${Math.round((candidate.similarity || 0) * 100)}% similar`),
    ...(repo.recurring_bug_clusters || []).map((cluster) => `Recurring cluster: ${cluster.label} · ${cluster.issue_count} issues · ${(cluster.shared_terms || []).join(", ")}`),
  ];
}

function repoLinks(repo) {
  return [
    ...(repo.repo_url ? [{ label: "Open repository", url: repo.repo_url }] : []),
    ...(repo.issue_examples || []).filter((issue) => issue.url).map((issue) => ({ label: `Issue #${issue.number}`, url: issue.url })),
  ];
}

function WorkspaceDetails({ fetcher, health, onError, result }) {
  const [timeline, setTimeline] = useState(null);
  const [report, setReport] = useState("");

  useEffect(() => {
    let active = true;
    setTimeline(null);
    setReport("");
    if (!result?.id) return () => { active = false; };
    Promise.allSettled([
      fetcher(`${API}/history/${encodeURIComponent(result.id)}/timeline`).then((response) => response.ok ? response.json() : Promise.reject(new Error("Timeline unavailable"))),
      fetcher(`${API}/history/${encodeURIComponent(result.id)}/report`).then((response) => response.ok ? response.json() : Promise.reject(new Error("Report unavailable"))),
    ]).then(([timelineResult, reportResult]) => {
      if (!active) return;
      if (timelineResult.status === "fulfilled") setTimeline(timelineResult.value);
      if (reportResult.status === "fulfilled") setReport(reportResult.value.markdown || "");
    });
    return () => { active = false; };
  }, [fetcher, result?.id]);

  if (!result) return null;
  const repos = result.repos || [];
  const totals = repos.reduce((current, repo) => ({
    stale: current.stale + Number(repo.stale_issues || 0),
    duplicates: current.duplicates + duplicateCount(repo),
    recurring: current.recurring + recurringCount(repo),
    markers: current.markers + markerCount(repo),
    examples: current.examples + (repo.issue_examples?.length || 0),
  }), { stale: 0, duplicates: 0, recurring: 0, markers: 0, examples: 0 });
  const markerCoverage = repos.filter((repo) => repo.todo_available || repo.fixme_available).length;
  const trend = result.trend;
  const events = (timeline?.points || []).map((point) => ({
    id: point.id,
    kind: point.trigger_type === "schedule" ? "schedule" : "scan",
    at: point.created_at,
    actor: point.schedule_name ? `schedule ${point.schedule_name}` : "SignalHive",
    message: `${countLabel(point.total_signals, "signal")} across ${countLabel(point.total_repos, "repository")} · top ${point.top_repo || "none"} at ${Math.round(point.top_priority_score || 0)}/100`,
  }));

  function downloadReport() {
    const content = report || buildScanMarkdown(result);
    const blob = new Blob([content], { type: "text/markdown;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `signalhive-report-${result.id}.md`;
    link.click();
    URL.revokeObjectURL(url);
  }

  return <div className="mt-8 space-y-6">
    <section className="surface p-5 sm:p-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Radar size={12} /> Repository scan evidence</div><h2 className={`mt-2 font-display text-[27px] font-semibold ${V3_TEXT.strong}`}>{scanScope(result)}</h2><p className={`mt-2 max-w-4xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>SignalHive found {countLabel(result.summary?.total_signals, "maintenance signal")} across {countLabel(result.summary?.total_repos, "repository")}. The queue remains reconnaissance evidence, not authorization to modify any repository.</p></div><div className="flex flex-wrap gap-2"><CopyMarkdownButton content={report || buildScanMarkdown(result)} label="Copy report Markdown" onError={() => onError("Could not copy the SignalHive report.")} /><button className={`surface-inset h-9 rounded-full px-3 text-[11px] ${V3_TEXT.body}`} onClick={downloadReport} type="button">Download report</button><button className={`surface-inset h-9 rounded-full px-3 text-[11px] ${V3_TEXT.body}`} onClick={() => downloadDashboardSnapshot(result, timeline, scanScope(result))} type="button">Export snapshot</button></div></div>
      <div className="mt-5 flex flex-wrap gap-2"><Chip>{String(result.id).slice(0, 8)}</Chip><Chip tone="ok">read only</Chip><Chip tone={result.warnings?.length ? "warn" : "ok"}>{result.warnings?.length ? `${result.warnings.length} warnings` : "complete evidence"}</Chip><Chip>{triggerLabel(result.trigger_type)}</Chip>{result.schedule_name ? <Chip>{result.schedule_name}</Chip> : null}</div>
    </section>
    <section className="surface p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Workflow size={12} /> Complete scan metrics</div><div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5"><Fact label="Repositories" value={repos.length} /><Fact label="Signals" value={result.summary?.total_signals} /><Fact label="Stale issues" value={totals.stale} /><Fact label="Duplicate pairs" value={totals.duplicates} /><Fact label="Recurring clusters" value={totals.recurring} /><Fact detail={`${markerCoverage}/${repos.length} repos searched`} label="TODO / FIXME observed" value={totals.markers} /><Fact label="Issue examples" value={totals.examples} /><Fact label="Warnings" value={result.warnings?.length || 0} /><Fact label="Min stars" value={result.params?.min_stars} /><Fact label="Stale window" value={`${result.params?.stale_days || 0}d`} /></div></section>
    {trend ? <section className="surface p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><TrendingUp size={12} /> Change since comparable scan</div><p className={`mt-2 text-[12px] ${V3_TEXT.mute}`}>Compared with {new Date(trend.compared_to_created_at).toLocaleString()}.</p><div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-4 lg:grid-cols-7"><Fact label="Signal delta" value={trend.total_signals_delta} /><Fact label="Repo delta" value={trend.total_repos_delta} /><Fact label="New repos" value={trend.new_repos} /><Fact label="Dropped" value={trend.dropped_repos} /><Fact label="Rising" value={trend.rising_repos} /><Fact label="Improving" value={trend.improving_repos} /><Fact label="Steady" value={trend.steady_repos} /></div></section> : null}
    {events.length ? <ActivityTimeline caption="Comparable scans" eventTypes={["scan", "schedule"]} events={events} /> : null}
    <ScanWarnings warnings={result.warnings || []} />
    {!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub could not verify the configured token. Repository, issue, and code-search coverage may be incomplete." : "Configure repository, issue, and code read access before treating discovery coverage as complete."}</GitHubPermissionGuidance> : null}
  </div>;
}

function ChecksDetails({ health }) {
  const github = health.github || {};
  const lists = health.repo_lists || {};
  const schedules = health.schedules || {};
  return <section className="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-2"><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitBranch size={12} /> GitHub repository-read path</div><div className="mt-5 grid grid-cols-2 gap-3"><Fact label="Token" value={github.token_verified ? "verified" : github.token_configured ? "unverified" : "missing"} /><Fact label="Repositories" value="read" /><Fact label="Issues" value="read" /><Fact label="Code search" value="best effort" /></div><GitHubPermissionGuidance>Repository metadata and issue reads power the base signal queue. TODO/FIXME counts depend on GitHub code-search availability; unavailable marker evidence must remain visible rather than silently becoming zero.</GitHubPermissionGuidance></article><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Database size={12} /> Product state</div><div className="surface-inset mt-5 rounded-xl p-3"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Database path</div><div className={`mt-1 break-all text-[12px] ${V3_TEXT.strong}`}>{health.db_path || "unknown"}</div></div><div className="mt-4 flex flex-wrap gap-2"><Chip tone={health.db_ok ? "ok" : "hot"}>database {health.db_ok ? "ready" : "unavailable"}</Chip><Chip tone={health.auth_enabled ? "ok" : "warn"}>auth {health.auth_enabled ? "enabled" : "disabled"}</Chip><Chip tone="ok">{countLabel(health.scan_count, "scan")}</Chip><Chip>{schedules.enabled || 0} active schedules</Chip></div><div className="mt-3 flex flex-wrap gap-2"><Chip>{lists.allowlist || 0} allowlisted</Chip><Chip tone="warn">{lists.denylist || 0} denied</Chip><Chip tone="hot">{lists.opt_out || 0} opted out</Chip></div></article></section>;
}

function SourcesDetails({ health }) {
  return <section className="surface mt-6 p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12} /> Repository safety</div><div className="mt-4 grid gap-3 sm:grid-cols-3"><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Read only</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>SignalHive searches public GitHub evidence and saves local scan results. It does not comment, change code, open issues, or create pull requests.</p></div><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Scope controls win</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Allowlists constrain direct and discovery scans. Denylists and repository opt-outs always exclude a repository.</p></div><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Coverage gaps stay visible</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Rate limits and unavailable code search are reported as warnings, never interpreted as proof that maintenance pressure is absent.</p></div></div>{!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub token verification failed. Review startup evidence before scanning." : "Add a GitHub token with repository, issue, and code read access."}</GitHubPermissionGuidance> : null}</section>;
}

const DEFAULT_FORM = { target_repo: "", search_query: "", topics: "", languages: "rust, typescript, python", min_stars: "25", max_repos: "8", issues_per_repo: "30", stale_days: "45" };

const baseConfig = {
  productKey: "signal-hive",
  name: "SignalHive",
  subtitle: "maintenance reconnaissance",
  icon: Radar,
  workspaceLabel: "Recon",
  eyebrow: "Maintenance discovery",
  queueLabel: "Repository signal queue",
  description: "Surfaces stale backlog pressure, duplicate issues, recurring bugs, and TODO/FIXME hotspots in one repository or across a bounded GitHub discovery scope—without changing them.",
  runLabel: "Run scan",
  runningLabel: "Scanning…",
  actionPath: "/scan",
  requiresRepo: false,
  canRun: (form) => Boolean(form.target_repo?.trim() || form.search_query?.trim() || toList(form.topics).length || toList(form.languages).length),
  formTitle: "Choose a repository or discovery scope.",
  sourceHelp: "Enter one repository for a direct scan, or leave it blank and let SignalHive discover repositories from the bounded scope.",
  searchPlaceholder: "Search repository, signal, issue, language…",
  emptyLabel: "No repositories match this view.",
  defaultForm: DEFAULT_FORM,
  fields: [
    { key: "target_repo", label: "Target repository", placeholder: "owner/repository", primary: true, help: "Optional. When present, SignalHive scans only this repository." },
    { key: "search_query", label: "GitHub discovery query", placeholder: "archived:false good first issues", help: "Used only when Target repository is blank." },
    { key: "topics", label: "Topics", placeholder: "developer-tools, maintenance", help: "Comma- or newline-separated GitHub topics." },
    { key: "languages", label: "Languages", placeholder: "rust, typescript, python", help: "Comma- or newline-separated languages." },
    { key: "min_stars", label: "Minimum stars", type: "number", min: 1, max: 1000000 },
    { key: "max_repos", label: "Repository limit", type: "number", min: 1, max: 25 },
    { key: "issues_per_repo", label: "Issues per repository", type: "number", min: 5, max: 100 },
    { key: "stale_days", label: "Stale after days", type: "number", min: 1, max: 730 },
  ],
  validate: (form) => {
    if (form.target_repo?.trim() && !/^[^/\s]+\/[^/\s]+$/.test(form.target_repo.trim())) return "Target repository must use owner/repository format.";
    if (!form.target_repo?.trim() && !form.search_query?.trim() && !toList(form.topics).length && !toList(form.languages).length) return "Provide a target repository or a discovery query, topic, or language before scanning.";
    const ranges = [["Minimum stars", form.min_stars, 1, 1000000], ["Repository limit", form.max_repos, 1, 25], ["Issues per repository", form.issues_per_repo, 5, 100], ["Stale days", form.stale_days, 1, 730]];
    const invalid = ranges.find(([, input, min, max]) => !Number.isInteger(Number(input)) || Number(input) < min || Number(input) > max);
    return invalid ? `${invalid[0]} must be a whole number from ${invalid[2]} through ${invalid[3]}.` : "";
  },
  serialize,
  formFromResult: (result) => toFormParams(result?.params),
  targetLabel: (result, form) => result ? scanScope(result) : scanScope({ params: serialize(form) }),
  items: (result) => result?.repos || [],
  mapItem: (repo) => {
    const signalTags = (repo.signals || []).filter((signal) => signal !== repo.summary).slice(0, 5);
    const tagSet = new Set(signalTags);
    return {
      id: repo.full_name,
      title: repo.full_name,
      meta: [repo.language || "unknown language", countLabel(repo.stars, "star"), `${countLabel(repo.sampled_issues, "issue")} sampled`].join(" · "),
      summary: repo.summary,
      evidence: repoEvidence(repo).filter((entry) => entry !== repo.summary && !tagSet.has(entry)),
      links: repoLinks(repo),
      tags: [...signalTags, ...(repo.warnings?.length ? [countLabel(repo.warnings.length, "coverage warning")] : [])],
      facts: [
        { label: "Priority score", value: Math.round(repo.priority_score || 0) },
        { label: "Trend", value: trendLabel(repo) },
        { label: "Stale issues", value: repo.stale_issues || 0 },
        { label: "Unlabeled issues", value: repo.unlabeled_issues || 0 },
        { label: "Duplicate pairs", value: duplicateCount(repo) },
        { label: "Recurring clusters", value: recurringCount(repo) },
        { label: "TODO / FIXME", value: `${repo.todo_available ? repo.todo_count || 0 : "unavailable"} / ${repo.fixme_available ? repo.fixme_count || 0 : "unavailable"}` },
      ],
      status: trendLabel(repo),
      tone: repo.trend ? trendTone(repo.trend.status) : scoreTone(repo.priority_score),
      score: Math.round(repo.priority_score || 0),
      source: "GitHub repository evidence",
      link: repo.repo_url,
      language: repo.language || "unknown",
      trend: trendLabel(repo),
      stale: repo.stale_issues || 0,
      duplicates: duplicateCount(repo),
      markers: markerCount(repo),
    };
  },
  metrics: (result, overview, health) => [
    { label: "Repositories", value: result?.summary?.total_repos ?? overview?.counts?.repositories ?? 0, footerLeft: "ranked", footerRight: `${result?.repos?.length || 0} in queue`, tone: "from-sky-600/70 to-blue-900/60" },
    { label: "Signals", value: result?.summary?.total_signals ?? overview?.counts?.signals ?? 0, footerLeft: "maintenance", footerRight: "visible pressure", tone: "from-orange-700/70 to-red-900/60" },
    { label: "Scans", value: health.scan_count || overview?.counts?.scans || 0, footerLeft: "saved", footerRight: "local history", tone: "from-slate-500/70 to-slate-800/60" },
    { label: "Warnings", value: result?.warnings?.length ?? overview?.counts?.warnings ?? 0, footerLeft: "coverage", footerRight: result?.warnings?.length ? "review" : "complete", tone: result?.warnings?.length ? "from-amber-600/70 to-yellow-800/50" : "from-emerald-700/70 to-teal-900/60" },
  ],
  hero: (result) => {
    const repositories = result?.summary?.total_repos ?? 0;
    const signals = result?.summary?.total_signals ?? 0;
    return { lead: repositories, middle: repositories === 1 ? "repository surfaces" : "repositories surface", highlight: `${countLabel(signals, "signal")}.` };
  },
  status: (result) => {
    const count = Number(result?.summary?.total_signals || 0);
    const warnings = result?.warnings?.length || 0;
    return { label: warnings ? "partial" : count ? "pressure" : "quiet", detail: warnings ? "Some repository evidence was unavailable; warnings remain visible." : count ? "Maintenance pressure is ranked for review." : "No strong maintenance signals in this scan.", progress: `${Math.min(100, Math.max(12, count))}%`, stats: [["repos", result?.summary?.total_repos || 0], ["signals", count], ["warnings", warnings]] };
  },
  chips: (result, health) => [scanScope(result), `${result?.params?.stale_days || 45} day stale window`, targetSelectionMode(result) === "direct" ? "direct target" : `${result?.params?.max_repos || 8} repo cap`, health.github_ready ? "GitHub verified" : "coverage pending"],
  targetSubtitle: (result) => result ? `${triggerLabel(result.trigger_type)} · ${countLabel(result.summary?.total_repos, "repository")}` : "No scan loaded",
  historyItems: (payload) => payload.scans || [],
  historyTitle: (entry) => entry.top_repo || scanScope({ params: { search_query: entry.search_query, topics: entry.topics, languages: entry.languages } }),
  historySummary: (entry) => `${countLabel(entry.total_signals, "maintenance signal")} across ${countLabel(entry.total_repos, "repository")}${entry.warning_count ? ` with ${countLabel(entry.warning_count, "coverage warning")}` : ""}.`,
  historyMeta: (entry) => `${triggerLabel(entry.trigger_type)}${entry.schedule_name ? ` · ${entry.schedule_name}` : ""} · ${targetSelectionMode(entry) === "direct" ? "direct target" : `${entry.max_repos || 0} repo cap`}`,
  historyIdentity: (entry) => `scan ${String(entry.id).slice(0, 8)}`,
  historySearchText: (entry) => `${entry.top_repo} ${entry.search_query || ""} ${(entry.topics || []).join(" ")} ${(entry.languages || []).join(" ")} ${entry.trigger_type} ${entry.schedule_name || ""}`,
  historyBadges: (entry) => [{ label: countLabel(entry.total_repos, "repo"), tone: "neutral" }, { label: countLabel(entry.total_signals, "signal"), tone: entry.total_signals ? "warn" : "ok" }, { label: countLabel(entry.warning_count, "warning"), tone: entry.warning_count ? "warn" : "neutral" }],
  dashboard: {
    defaultView: { language: "all", trend: "all", sort: "priority" },
    initialCount: 6,
    itemLabel: "repositories",
    filters: (items, view) => [
      { key: "language", label: "Language", value: view.language, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.language).filter(Boolean))].sort().map((language) => ({ value: language, label: language }))] },
      { key: "trend", label: "Trend", value: view.trend, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.trend).filter(Boolean))].sort().map((trend) => ({ value: trend, label: trend }))] },
    ],
    filterItem: (item, view) => (view.language === "all" || item.language === view.language) && (view.trend === "all" || item.trend === view.trend),
    sortItems: (left, right, sort) => sort === "name" ? left.title.localeCompare(right.title) : sort === "stale" ? right.stale - left.stale : sort === "duplicates" ? right.duplicates - left.duplicates : sort === "markers" ? right.markers - left.markers : right.score - left.score,
    sortOptions: [{ value: "priority", label: "Priority score" }, { value: "stale", label: "Stale issues" }, { value: "duplicates", label: "Duplicate pressure" }, { value: "markers", label: "TODO / FIXME" }, { value: "name", label: "Repository name" }],
  },
  historyDashboard: {
    defaultView: { trigger: "all", coverage: "all", sort: "newest" },
    initialCount: 6,
    searchPlaceholder: "Search scan, repository, topic, language…",
    filters: (_entries, view) => [{ key: "trigger", label: "Trigger", value: view.trigger, options: [{ value: "all", label: "All" }, { value: "operator", label: "Operator run" }, { value: "schedule", label: "Scheduled" }] }, { key: "coverage", label: "Coverage", value: view.coverage, options: [{ value: "all", label: "All" }, { value: "complete", label: "Complete" }, { value: "warnings", label: "Warnings" }] }],
    filterEntry: (entry, view) => (view.trigger === "all" || triggerMode(entry.trigger_type) === view.trigger) && (view.coverage === "all" || (view.coverage === "warnings" ? entry.warning_count > 0 : !entry.warning_count)),
    sortEntries: (left, right, sort) => sort === "oldest" ? new Date(left.created_at) - new Date(right.created_at) : sort === "signals" ? right.total_signals - left.total_signals : sort === "repos" ? right.total_repos - left.total_repos : new Date(right.created_at) - new Date(left.created_at),
    sortOptions: [{ value: "newest", label: "Newest first" }, { value: "oldest", label: "Oldest first" }, { value: "signals", label: "Most signals" }, { value: "repos", label: "Most repositories" }],
  },
  ChecksDetails,
  SourcesDetails,
};

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "signal_api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const config = useMemo(() => ({
    ...baseConfig,
    WorkspaceDetails: (props) => <WorkspaceDetails {...props} fetcher={fetcher} />,
    extraTabs: [{ id: "controls", label: "Controls", render: (props) => <ControlsPanel {...props} serialize={serialize} toFormParams={toFormParams} /> }],
  }), [fetcher]);
  if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />;
  return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />;
}

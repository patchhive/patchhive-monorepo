import { useMemo } from "react";
import { Bug, Database, GitBranch, ShieldCheck, TrendingDown, TrendingUp, Workflow } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  CopyMarkdownButton,
  countLabel,
  GitHubPermissionGuidance,
  IntegratedProductApp,
  ProductLoginScreen,
  ProductShell,
  V3_TEXT,
} from "@patchhivehq/ui-v3";
import { API } from "./config.js";

const CHIP_TONES = {
  hot: "border-red-900/30 bg-red-900/10 text-red-800 dark:border-red-400/25 dark:bg-red-500/10 dark:text-red-300",
  warn: "border-amber-900/30 bg-amber-900/10 text-amber-800 dark:border-amber-400/25 dark:bg-amber-500/10 dark:text-amber-300",
  ok: "border-emerald-900/30 bg-emerald-900/10 text-emerald-800 dark:border-emerald-400/25 dark:bg-emerald-500/10 dark:text-emerald-300",
  neutral: "border-stone-800/20 bg-stone-800/5 text-stone-700 dark:border-stone-400/20 dark:bg-stone-400/5 dark:text-stone-300",
};

function value(input, fallback = "—") {
  return input === null || input === undefined || input === "" ? fallback : String(input);
}

function Chip({ children, tone = "neutral" }) {
  return <span className={`inline-flex items-center justify-center rounded-full border px-2.5 py-1 text-[10px] leading-none uppercase tracking-wider ${CHIP_TONES[tone] || CHIP_TONES.neutral}`}>{children}</span>;
}

function Fact({ label, value: factValue }) {
  return <div className="surface-inset rounded-xl p-3"><div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</div><div className={`mt-1 font-display text-[18px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value(factValue, "0")}</div></div>;
}

function signalTone(signal) {
  if (signal.status === "quarantine" || Number(signal.score) >= 80) return "hot";
  if (signal.status === "suspect" || signal.status === "watch" || Number(signal.score) >= 50) return "warn";
  return "ok";
}

function trendTone(status) {
  if (status === "rising") return "hot";
  if (status === "improving") return "ok";
  if (status === "shifted") return "warn";
  return "neutral";
}

function statusRank(status) {
  if (status === "quarantine") return 3;
  if (status === "suspect" || status === "watch") return 2;
  return 1;
}

function signedCount(input) {
  const count = Number(input || 0);
  return count > 0 ? `+${count}` : String(count);
}

function evidenceLinks(signal) {
  const urls = (signal.evidence || []).flatMap((entry) => String(entry).match(/https?:\/\/[^\s),]+/g) || []);
  return [...new Set(urls)].map((url, index) => ({ label: urls.length === 1 ? "Open workflow evidence" : `Evidence ${index + 1}`, url }));
}

function buildScanMarkdown(scan) {
  const metrics = scan?.metrics || {};
  const lines = [
    `# FlakeSting scan for ${scan?.repo || "repository"}`,
    "",
    scan?.summary || "FlakeSting workflow scan.",
    "",
    `- Branch: ${scan?.branch || "all branches"}`,
    `- Workflow filter: ${scan?.workflow_name || "all workflows"}`,
    `- Workflow runs: ${metrics.workflow_runs || 0}`,
    `- Completed runs: ${metrics.completed_runs || 0}`,
    `- Successful runs: ${metrics.successful_runs || 0}`,
    `- Failed runs: ${metrics.failed_runs || 0}`,
    `- Rerun-like runs: ${metrics.rerun_like_runs || 0}`,
    `- Flaky signals: ${metrics.flaky_signals || 0}`,
    `- Quarantine candidates: ${metrics.quarantine_candidates || 0}`,
  ];
  if (scan?.trend) {
    lines.push(
      "",
      "## Trend",
      "",
      `- Status: ${scan.trend.status || "baseline"}`,
      `- Signal delta: ${signedCount(scan.trend.flaky_signal_delta)}`,
      `- Quarantine delta: ${signedCount(scan.trend.quarantine_delta)}`,
      `- Rerun delta: ${signedCount(scan.trend.rerun_delta)}`,
      `- New signals: ${scan.trend.new_signal_count || 0}`,
      `- Cleared signals: ${scan.trend.cleared_signal_count || 0}`,
    );
  }
  if (scan?.signals?.length) {
    lines.push("", "## Top signals", "");
    [...scan.signals]
      .sort((left, right) => statusRank(right.status) - statusRank(left.status) || (right.score || 0) - (left.score || 0) || (right.failure_count || 0) - (left.failure_count || 0))
      .slice(0, 8)
      .forEach((signal) => lines.push(`- [${signal.status || "signal"}] ${signal.step_name || signal.job_name || signal.workflow_name} — ${signal.summary || signal.evidence?.[0] || "No summary available."}`));
  }
  lines.push("", "*FlakeSting by [PatchHive](https://github.com/patchhive)*");
  return lines.join("\n");
}

function WorkspaceDetails({ health, onError, onLoad, result }) {
  if (!result) return null;
  const metrics = result.metrics || {};
  const trend = result.trend;
  const TrendIcon = trend?.status === "improving" ? TrendingDown : TrendingUp;
  return (
    <div className="mt-8 space-y-6">
      <section className="surface p-5 sm:p-6">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Bug size={12} /> Workflow scan evidence</div><h2 className={`mt-2 font-display text-[27px] font-semibold ${V3_TEXT.strong}`}>{result.repo}</h2><p className={`mt-2 max-w-4xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>{result.summary || "Saved CI instability scan."}</p></div>
          <CopyMarkdownButton content={buildScanMarkdown(result)} label="Copy scan Markdown" onError={() => onError("Could not copy the FlakeSting scan summary.")} />
        </div>
        <div className="mt-5 flex flex-wrap gap-2"><Chip>{value(result.id).slice(0, 8)}</Chip><Chip tone="ok">read only</Chip><Chip tone={metrics.quarantine_candidates ? "hot" : metrics.flaky_signals ? "warn" : "ok"}>{metrics.quarantine_candidates ? "quarantine review" : metrics.flaky_signals ? "suspect queue" : "stable signal"}</Chip></div>
      </section>

      <section className="surface p-5 sm:p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Workflow size={12} /> Complete workflow metrics</div>
        <div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
          <Fact label="Workflow runs" value={metrics.workflow_runs} /><Fact label="Completed" value={metrics.completed_runs} /><Fact label="Successful" value={metrics.successful_runs} /><Fact label="Failed" value={metrics.failed_runs} /><Fact label="Rerun-like" value={metrics.rerun_like_runs} /><Fact label="Flaky signals" value={metrics.flaky_signals} /><Fact label="Quarantine" value={metrics.quarantine_candidates} /><Fact label="Evidence links" value={(result.signals || []).reduce((total, signal) => total + evidenceLinks(signal).length, 0)} />
        </div>
      </section>

      {trend ? <section className="surface p-5 sm:p-6"><div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><TrendIcon size={12} /> Change since comparable scan</div><div className="mt-3 flex flex-wrap gap-2"><Chip tone={trendTone(trend.status)}>{trend.status || "baseline"}</Chip><Chip>{signedCount(trend.flaky_signal_delta)} signals</Chip><Chip>{signedCount(trend.quarantine_delta)} quarantine</Chip><Chip>{signedCount(trend.rerun_delta)} reruns</Chip><Chip tone="warn">{trend.new_signal_count || 0} new</Chip><Chip tone="ok">{trend.cleared_signal_count || 0} cleared</Chip></div><p className={`mt-3 text-[11px] ${V3_TEXT.mute}`}>{trend.compared_to_created_at ? `Compared with ${new Date(trend.compared_to_created_at).toLocaleString()}.` : "Baseline saved. A trend appears after the next comparable scan."}</p></div>{trend.compared_to_scan_id ? <button className={`surface-inset h-9 rounded-full px-4 text-[11px] ${V3_TEXT.body}`} onClick={() => onLoad(trend.compared_to_scan_id)} type="button">Load previous scan</button> : null}</div>{trend.new_signals?.length || trend.cleared_signals?.length ? <div className="mt-5 grid gap-3 sm:grid-cols-2"><div className="surface-inset rounded-xl p-4"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>New signals</div><div className={`mt-2 space-y-1 text-[11px] ${V3_TEXT.body}`}>{trend.new_signals?.length ? trend.new_signals.map((entry) => <div key={entry}>{entry}</div>) : <div>None</div>}</div></div><div className="surface-inset rounded-xl p-4"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Cleared signals</div><div className={`mt-2 space-y-1 text-[11px] ${V3_TEXT.body}`}>{trend.cleared_signals?.length ? trend.cleared_signals.map((entry) => <div key={entry}>{entry}</div>) : <div>None</div>}</div></div></div> : null}</section> : null}

      {!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub could not verify the configured token. Actions history may be unavailable until the token is corrected." : "Configure repository metadata and Actions read access before running a live FlakeSting scan."}</GitHubPermissionGuidance> : null}
    </div>
  );
}

function ChecksDetails({ health }) {
  const github = health.github || {};
  return <section className="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-2"><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitBranch size={12} /> GitHub Actions read path</div><div className="mt-5 grid grid-cols-2 gap-3"><Fact label="Token" value={github.token_verified ? "verified" : github.token_configured ? "unverified" : "missing"} /><Fact label="Repository" value="read" /><Fact label="Actions" value="read" /><Fact label="Mode" value="read only" /></div><GitHubPermissionGuidance>Repository metadata and Actions read access power workflow-run, job, and step evidence. FlakeSting does not rerun or mutate workflows.</GitHubPermissionGuidance></article><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Database size={12} /> Product state</div><div className="surface-inset mt-5 rounded-xl p-3"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Database path</div><div className={`mt-1 break-all text-[12px] ${V3_TEXT.strong}`}>{health.db_path || "unknown"}</div></div><div className="mt-4 flex flex-wrap gap-2"><Chip tone={health.db_ok ? "ok" : "hot"}>database {health.db_ok ? "ready" : "unavailable"}</Chip><Chip tone={health.auth_enabled ? "ok" : "warn"}>auth {health.auth_enabled ? "enabled" : "disabled"}</Chip><Chip tone="ok">{countLabel(health.scan_count, "scan")}</Chip><Chip>{countLabel(health.repo_count, "repo")}</Chip></div><div className="mt-3 flex flex-wrap gap-2"><Chip tone="warn">{countLabel(health.flaky_signal_count, "signal")}</Chip><Chip tone="hot">{countLabel(health.quarantine_candidate_count, "quarantine candidate")}</Chip></div></article></section>;
}

function SourcesDetails({ health }) {
  return <section className="surface mt-6 p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12} /> Workflow-scan safety</div><div className="mt-4 grid gap-3 sm:grid-cols-3"><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Read only</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>FlakeSting reads workflow history. It does not rerun jobs, edit CI, suppress checks, or quarantine tests.</p></div><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Test-like evidence</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Fail/pass swings are ranked only for test-like jobs and steps, with direct workflow evidence preserved.</p></div><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Trend needs context</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Trend comparisons use the prior matching repository, branch, and workflow scope instead of unrelated scans.</p></div></div>{!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub token verification failed. Review startup evidence before scanning." : "Add a GitHub token with repository metadata and Actions read access."}</GitHubPermissionGuidance> : null}</section>;
}

const config = {
  productKey: "flake-sting",
  name: "FlakeSting",
  subtitle: "CI trust",
  icon: Bug,
  workspaceLabel: "Instability",
  eyebrow: "Workflow instability",
  queueLabel: "Flaky signal queue",
  description: "Reads GitHub Actions history and explains fail/pass swings, rerun pressure, and likely quarantine candidates.",
  runLabel: "Run scan",
  runningLabel: "Scanning…",
  actionPath: "/scan/github/actions",
  formTitle: "Choose a workflow scope.",
  sourceHelp: "The unified backend needs repository metadata and GitHub Actions read access. FlakeSting remains read-only and evidence-first.",
  searchPlaceholder: "Search workflow, job, step, runner hint…",
  emptyLabel: "No flaky signals match this view.",
  defaultForm: { repo: "", branch: "", workflow_name: "", lookback_runs: "25" },
  fields: [
    { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
    { key: "branch", label: "Branch", placeholder: "All branches when blank", help: "Optional exact branch filter." },
    { key: "workflow_name", label: "Workflow", placeholder: "All workflows when blank", help: "Optional case-insensitive workflow-name match." },
    { key: "lookback_runs", label: "Lookback runs", type: "number", min: 5, max: 40, help: "Inspect the newest 5–40 matching workflow runs." },
  ],
  validate: (form) => {
    if (!/^[^/\s]+\/[^/\s]+$/.test(form.repo?.trim() || "")) return "Enter a repository in owner/name format before scanning Actions history.";
    const lookback = Number(form.lookback_runs);
    if (!Number.isInteger(lookback) || lookback < 5 || lookback > 40) return "Lookback runs must be a whole number from 5 through 40.";
    return "";
  },
  serialize: (form) => ({ repo: form.repo.trim(), branch: form.branch.trim(), workflow_name: form.workflow_name.trim(), lookback_runs: Number(form.lookback_runs) }),
  formFromResult: (result) => ({ repo: result.repo || "", branch: result.branch || "", workflow_name: result.workflow_name || "" }),
  items: (result) => result?.signals || [],
  mapItem: (item) => ({
    id: item.key || `${item.workflow_name}-${item.job_name}-${item.step_name}`,
    title: item.step_name || item.job_name || item.workflow_name || "Workflow signal",
    meta: [item.workflow_name, item.job_name, item.kind].filter(Boolean).join(" · "),
    summary: item.summary || item.evidence?.[0],
    evidence: item.evidence || [],
    links: evidenceLinks(item),
    tags: item.environment_hints || [],
    facts: [
      { label: "Workflow", value: value(item.workflow_name) }, { label: "Job", value: value(item.job_name) }, { label: "Step", value: value(item.step_name) }, { label: "Kind", value: value(item.kind) }, { label: "Failures", value: item.failure_count || 0 }, { label: "Passes", value: item.success_count || 0 }, { label: "Rerun hits", value: item.rerun_hits || 0 },
    ],
    source: "GitHub Actions",
    score: item.score || 0,
    status: item.status || item.kind || "signal",
    signalStatus: item.status || "signal",
    kind: item.kind || "unknown",
    workflow: item.workflow_name || "unknown",
    failureCount: item.failure_count || 0,
    successCount: item.success_count || 0,
    rerunHits: item.rerun_hits || 0,
    tone: signalTone(item),
  }),
  dashboard: {
    defaultView: { status: "all", kind: "all", workflow: "all", sort: "risk" },
    initialCount: 6,
    itemLabel: "signals",
    filters: (items, view) => [
      { key: "status", label: "Status", value: view.status, options: [{ value: "all", label: "All" }, { value: "quarantine", label: "Quarantine" }, { value: "suspect", label: "Suspect" }] },
      { key: "kind", label: "Kind", value: view.kind, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.kind).filter(Boolean))].sort().map((kind) => ({ value: kind, label: kind }))] },
      { key: "workflow", label: "Workflow", value: view.workflow, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.workflow).filter(Boolean))].sort().map((workflow) => ({ value: workflow, label: workflow }))] },
    ],
    filterItem: (item, view) => (view.status === "all" || item.signalStatus === view.status) && (view.kind === "all" || item.kind === view.kind) && (view.workflow === "all" || item.workflow === view.workflow),
    sortItems: (left, right, sort) => {
      if (sort === "failures") return right.failureCount - left.failureCount || right.score - left.score;
      if (sort === "reruns") return right.rerunHits - left.rerunHits || right.score - left.score;
      if (sort === "workflow") return left.workflow.localeCompare(right.workflow) || left.title.localeCompare(right.title);
      return statusRank(right.signalStatus) - statusRank(left.signalStatus) || right.score - left.score || right.failureCount - left.failureCount || right.rerunHits - left.rerunHits;
    },
    sortOptions: [{ value: "risk", label: "Risk first" }, { value: "failures", label: "Most failures" }, { value: "reruns", label: "Most reruns" }, { value: "workflow", label: "Workflow" }],
  },
  metrics: (result, overview, health) => {
    const metrics = result?.metrics || {};
    const counts = overview?.counts || {};
    return result ? [
      { label: "Flaky signals", value: metrics.flaky_signals || 0, footerLeft: "current", footerRight: result.trend?.status || "baseline", tone: "from-orange-700/70 to-red-900/60" },
      { label: "Failed runs", value: metrics.failed_runs || 0, footerLeft: `${metrics.successful_runs || 0} passing`, footerRight: countLabel(metrics.workflow_runs, "run"), tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Reruns", value: metrics.rerun_like_runs || 0, footerLeft: "retry", footerRight: "pressure", tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Quarantine", value: metrics.quarantine_candidates || 0, footerLeft: "candidates", footerRight: "review", tone: "from-orange-700/70 to-red-900/60" },
    ] : [
      { label: "Scans", value: counts.scans || health.scan_count || 0, footerLeft: "saved", footerRight: countLabel(counts.repos || health.repo_count, "repo"), tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Flaky signals", value: counts.flaky_signals || health.flaky_signal_count || 0, footerLeft: "saved", footerRight: "CI pressure", tone: "from-orange-700/70 to-red-900/60" },
      { label: "Quarantine", value: counts.quarantine_candidates || health.quarantine_candidate_count || 0, footerLeft: "saved", footerRight: "candidates", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "GitHub", value: health.github_ready ? "on" : "off", footerLeft: "Actions", footerRight: "read", tone: "from-slate-500/70 to-slate-800/60" },
    ];
  },
  hero: (result) => result ? { lead: countLabel(result.metrics?.flaky_signals, "signal"), middle: Number(result.metrics?.flaky_signals || 0) === 1 ? "needs" : "need", highlight: "attention." } : { lead: "Unstable checks", middle: "need", highlight: "evidence." },
  status: (result, overview) => ({ label: result?.metrics?.quarantine_candidates ? "quarantine" : result?.metrics?.flaky_signals ? "suspect" : result ? "stable" : "—", detail: result?.summary || "Scan workflow history to begin", progress: result ? "100%" : "8%", stats: [["Runs", result?.metrics?.workflow_runs || 0], ["Reruns", result?.metrics?.rerun_like_runs || 0], ["Scans", overview?.counts?.scans || 0]] }),
  chips: (result, health) => [result?.repo || "No repository selected", result?.branch || "All branches", result?.workflow_name || "All workflows", health.github_ready ? "GitHub verified" : health.github?.token_configured ? "GitHub unverified" : "Token missing"],
  targetSubtitle: (result) => result ? `${result.branch || "all branches"} · ${result.workflow_name || "all workflows"}` : "Actions history",
  historyTitle: (entry) => `${entry.repo} · ${entry.workflow_name || entry.branch || "Actions"}`,
  historySummary: (entry) => entry.summary,
  historyMeta: (entry) => `${countLabel(entry.flaky_signals, "signal")} · ${countLabel(entry.quarantine_candidates, "quarantine candidate")} · ${entry.trend?.status || "baseline"}`,
  historyIdentity: (entry) => `scan ${String(entry.id || "unknown").slice(0, 8)}`,
  historySearchText: (entry) => `${entry.workflow_name || "all workflows"} ${entry.trend?.status || "baseline"} ${entry.flaky_signals || 0} signals ${entry.quarantine_candidates || 0} quarantine`,
  historyBadges: (entry) => [{ label: entry.trend?.status || "baseline", tone: trendTone(entry.trend?.status) }, { label: countLabel(entry.flaky_signals, "signal"), tone: entry.flaky_signals ? "warn" : "neutral" }, { label: countLabel(entry.quarantine_candidates, "quarantine"), tone: entry.quarantine_candidates ? "hot" : "neutral" }],
  historyDashboard: {
    defaultView: { trend: "all", repo: "all", workflow: "all", sort: "newest" },
    initialCount: 6,
    searchPlaceholder: "Search repository, workflow, summary, trend…",
    filters: (items, view) => [
      { key: "trend", label: "Trend", value: view.trend, options: [{ value: "all", label: "All" }, { value: "rising", label: "Rising" }, { value: "improving", label: "Improving" }, { value: "steady", label: "Steady" }, { value: "baseline", label: "Baseline" }] },
      { key: "repo", label: "Repository", value: view.repo, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((entry) => entry.repo).filter(Boolean))].sort().map((repo) => ({ value: repo, label: repo }))] },
      { key: "workflow", label: "Workflow", value: view.workflow, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((entry) => entry.workflow_name || "all workflows"))].sort().map((workflow) => ({ value: workflow, label: workflow }))] },
    ],
    filterEntry: (entry, view) => (view.trend === "all" || (entry.trend?.status || "baseline") === view.trend) && (view.repo === "all" || entry.repo === view.repo) && (view.workflow === "all" || (entry.workflow_name || "all workflows") === view.workflow),
    sortEntries: (left, right, sort) => {
      if (sort === "oldest") return new Date(left.created_at) - new Date(right.created_at);
      if (sort === "pressure") return (right.flaky_signals || 0) - (left.flaky_signals || 0) || (right.quarantine_candidates || 0) - (left.quarantine_candidates || 0) || new Date(right.created_at) - new Date(left.created_at);
      if (sort === "quarantine") return (right.quarantine_candidates || 0) - (left.quarantine_candidates || 0) || (right.flaky_signals || 0) - (left.flaky_signals || 0);
      if (sort === "repo") return left.repo.localeCompare(right.repo) || new Date(right.created_at) - new Date(left.created_at);
      return new Date(right.created_at) - new Date(left.created_at);
    },
    sortOptions: [{ value: "newest", label: "Newest first" }, { value: "oldest", label: "Oldest first" }, { value: "pressure", label: "Highest pressure" }, { value: "quarantine", label: "Most quarantine" }, { value: "repo", label: "Repository" }],
  },
  WorkspaceDetails,
  ChecksDetails,
  SourcesDetails,
};

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "flake-sting_api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />;
  return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />;
}

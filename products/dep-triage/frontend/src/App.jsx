import { useMemo } from "react";
import { Database, GitBranch, PackageSearch, ShieldCheck, Workflow } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  countLabel,
  CopyMarkdownButton,
  GitHubPermissionGuidance,
  IntegratedProductApp,
  ProductLoginScreen,
  ProductShell,
  ScanWarnings,
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

function recommendationLabel(input) {
  return String(input || "watch").replaceAll("_", " ");
}

function recommendationTone(input) {
  const normalized = String(input || "").toLowerCase();
  if (normalized === "update_now" || normalized.includes("update now")) return "hot";
  if (normalized === "watch") return "warn";
  if (normalized === "ignore_for_now" || normalized.includes("ignore")) return "ok";
  return "neutral";
}

function recommendationRank(input) {
  if (input === "update_now") return 3;
  if (input === "watch") return 2;
  return 1;
}

function Chip({ children, tone = "neutral" }) {
  return <span className={`inline-flex items-center justify-center rounded-full border px-2.5 py-1 text-[10px] leading-none uppercase tracking-wider ${CHIP_TONES[tone] || CHIP_TONES.neutral}`}>{children}</span>;
}

function Fact({ label, value: factValue }) {
  return (
    <div className="surface-inset rounded-xl p-3">
      <div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</div>
      <div className={`mt-1 font-display text-[18px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value(factValue, "0")}</div>
    </div>
  );
}

function warningLabel(warning) {
  const text = String(warning || "");
  if (text.includes("PATCHHIVE_GITHUB_TOKEN_RO is not set") || text.includes("BOT_GITHUB_TOKEN is not set") || text.includes("GITHUB_TOKEN is not set")) {
    return "Dependabot alerts were skipped because GitHub token access is not configured.";
  }
  if (text.includes("/dependabot/alerts") && text.includes("Dependabot alerts are disabled")) {
    return "DepTriage tried to read Dependabot alerts, but GitHub says alerts are disabled for this repository. Security pressure remains unavailable until that repository setting is enabled.";
  }
  if (text.includes("/dependabot/alerts") && (text.includes("403 Forbidden") || text.includes("Resource not accessible"))) {
    return "Dependabot alerts could not be read. The token needs Dependabot alert read access for this repository.";
  }
  return text;
}

function scanSummary(scan) {
  const repo = scan?.repo || "this repository";
  const warnings = scan?.warnings || [];
  const hasItems = Number(scan?.metrics?.tracked_items || 0) > 0 || Boolean(scan?.items?.length);
  if (!hasItems && warnings.some((warning) => warningLabel(warning).includes("token needs Dependabot"))) {
    return `DepTriage checked dependency pull requests for ${repo}, but could not read Dependabot security alerts with the current token.`;
  }
  if (!hasItems && warnings.length) {
    return `DepTriage did not receive actionable dependency items from every readable source for ${repo}. Review the scan warnings before treating this as a clean queue.`;
  }
  return scan?.summary || "Saved dependency triage scan.";
}

function buildScanMarkdown(scan) {
  const metrics = scan?.metrics || {};
  const lines = [
    `# DepTriage scan for ${scan?.repo || "repository"}`,
    "",
    scanSummary(scan),
    "",
    `- Tracked items: ${metrics.tracked_items || 0}`,
    `- Update now: ${metrics.update_now || 0}`,
    `- Watch: ${metrics.watch || 0}`,
    `- Ignore for now: ${metrics.ignore_for_now || 0}`,
    `- Dependency PRs: ${metrics.dependency_pull_requests || 0}`,
    `- Open alerts: ${metrics.open_alerts || 0}`,
  ];
  if (scan?.items?.length) {
    lines.push("", "## Top queue", "");
    [...scan.items]
      .sort((left, right) => (right.score || 0) - (left.score || 0))
      .slice(0, 8)
      .forEach((item) => lines.push(`- [${recommendationLabel(item.recommendation)}] ${item.package_name || item.key} — ${item.summary || item.reasons?.[0] || "Dependency evidence"}`));
  }
  if (scan?.warnings?.length) {
    lines.push("", "## Warnings", "");
    scan.warnings.forEach((warning) => lines.push(`- ${warningLabel(warning)}`));
  }
  lines.push("", "*DepTriage by [PatchHive](https://github.com/patchhive)*");
  return lines.join("\n");
}

function itemEvidence(item) {
  const backendEvidence = (item.evidence || []).filter((entry) => {
    const text = String(entry);
    return !/^PR #\d+/i.test(text) && !/^(?:Dependabot )?alert #\d+/i.test(text);
  });
  return [
    ...(item.reasons || []),
    ...backendEvidence,
    ...(item.changed_paths || []).map((path) => `Changed path: ${path}`),
    ...(item.pull_requests || []).map((pr) => `PR #${pr.number}: ${pr.title}${pr.from_version || pr.to_version ? ` · ${pr.from_version || "?"} → ${pr.to_version || "?"}` : ""}${pr.author ? ` · @${pr.author}` : ""}`),
    ...(item.alerts || []).map((alert) => `Alert #${alert.number} [${alert.severity || "unknown"}]: ${alert.summary}${alert.first_patched_version ? ` · first patched ${alert.first_patched_version}` : ""}`),
  ];
}

function itemLinks(item) {
  return [
    ...(item.pull_requests || []).filter((pr) => pr.html_url).map((pr) => ({ label: `PR #${pr.number}`, url: pr.html_url })),
    ...(item.alerts || []).filter((alert) => alert.html_url).map((alert) => ({ label: `Alert #${alert.number}`, url: alert.html_url })),
  ];
}

function WorkspaceDetails({ health, onError, result }) {
  if (!result) return null;
  const metrics = result.metrics || {};
  return (
    <div className="mt-8 space-y-6">
      <section className="surface p-5 sm:p-6">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><PackageSearch size={12} /> Dependency scan evidence</div>
            <h2 className={`mt-2 font-display text-[27px] font-semibold ${V3_TEXT.strong}`}>{result.repo}</h2>
            <p className={`mt-2 max-w-4xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>{scanSummary(result)}</p>
          </div>
          <CopyMarkdownButton content={buildScanMarkdown(result)} label="Copy scan Markdown" onError={() => onError("Could not copy the DepTriage scan summary.")} />
        </div>
        <div className="mt-5 flex flex-wrap gap-2"><Chip>{value(result.id).slice(0, 8)}</Chip><Chip tone="ok">read only</Chip><Chip tone={metrics.update_now ? "hot" : metrics.watch ? "warn" : "ok"}>{metrics.update_now ? "action needed" : metrics.watch ? "watch queue" : "safe defer"}</Chip></div>
      </section>

      <section className="surface p-5 sm:p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Workflow size={12} /> Complete dependency metrics</div>
        <div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
          <Fact label="Scanned PRs" value={metrics.scanned_pull_requests} />
          <Fact label="Dependency PRs" value={metrics.dependency_pull_requests} />
          <Fact label="Open alerts" value={metrics.open_alerts} />
          <Fact label="Tracked items" value={metrics.tracked_items} />
          <Fact label="Update now" value={metrics.update_now} />
          <Fact label="Watch" value={metrics.watch} />
          <Fact label="Ignore for now" value={metrics.ignore_for_now} />
          <Fact label="Runtime updates" value={metrics.runtime_updates} />
          <Fact label="Major updates" value={metrics.major_updates} />
          <Fact label="Evidence links" value={(result.items || []).reduce((total, item) => total + itemLinks(item).length, 0)} />
        </div>
      </section>

      <ScanWarnings formatWarning={warningLabel} warnings={result.warnings || []} />
      {!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub could not verify the configured token. Dependency PR and alert coverage may be incomplete." : "Configure pull-request read access for the base queue and Dependabot alerts read access for security enrichment."}</GitHubPermissionGuidance> : null}
    </div>
  );
}

function ChecksDetails({ health }) {
  const github = health.github || {};
  return (
    <section className="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-2">
      <article className="surface p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitBranch size={12} /> GitHub dependency-read path</div>
        <div className="mt-5 grid grid-cols-2 gap-3">
          <Fact label="Token" value={github.token_verified ? "verified" : github.token_configured ? "unverified" : "missing"} />
          <Fact label="Pull requests" value="read" />
          <Fact label="Dependabot" value="best effort" />
          <Fact label="Mode" value="read only" />
        </div>
        <GitHubPermissionGuidance>Pull-request read access powers the base queue. Dependabot alerts require the target repository to be selected and Dependabot alerts read permission; unavailable alerts must remain visible as a warning.</GitHubPermissionGuidance>
      </article>
      <article className="surface p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Database size={12} /> Product state</div>
        <div className="surface-inset mt-5 rounded-xl p-3"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Database path</div><div className={`mt-1 break-all text-[12px] ${V3_TEXT.strong}`}>{health.db_path || "unknown"}</div></div>
        <div className="mt-4 flex flex-wrap gap-2"><Chip tone={health.db_ok ? "ok" : "hot"}>database {health.db_ok ? "ready" : "unavailable"}</Chip><Chip tone={health.auth_enabled ? "ok" : "warn"}>auth {health.auth_enabled ? "enabled" : "disabled"}</Chip><Chip tone="ok">{countLabel(health.scan_count, "scan")}</Chip><Chip>{countLabel(health.repo_count, "repo")}</Chip></div>
        <div className="mt-3 flex flex-wrap gap-2"><Chip tone="hot">{value(health.update_now_count, "0")} now</Chip><Chip tone="warn">{value(health.watch_count, "0")} watch</Chip><Chip tone="ok">{value(health.ignore_count, "0")} ignore</Chip><Chip>{value(health.tracked_item_count, "0")} tracked</Chip></div>
      </article>
    </section>
  );
}

function SourcesDetails({ health }) {
  return (
    <section className="surface mt-6 p-5 sm:p-6">
      <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12} /> Dependency-scan safety</div>
      <div className="mt-4 grid gap-3 sm:grid-cols-3">
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Read only</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>DepTriage ranks existing PRs and alerts. It does not merge, close, update, or rewrite dependency work.</p></div>
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>PR queue first</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Pull-request evidence remains useful when security alerts are disabled or inaccessible.</p></div>
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Alert gaps stay visible</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>A missing Dependabot feed is reported as unavailable evidence, never as proof that risk is absent.</p></div>
      </div>
      {!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub token verification failed. Review startup evidence before scanning." : "Add a GitHub token for dependency pull-request reads and optional Dependabot alert enrichment."}</GitHubPermissionGuidance> : null}
    </section>
  );
}

const config = {
  productKey: "dep-triage",
  name: "DepTriage",
  subtitle: "dependency queue",
  icon: PackageSearch,
  workspaceLabel: "Triage",
  eyebrow: "Dependency decisions",
  queueLabel: "Update queue",
  description: "Turns dependency pull requests and Dependabot alerts into update-now, watch, and safe-defer decisions.",
  runLabel: "Run scan",
  runningLabel: "Scanning…",
  actionPath: "/scan/github/dependencies",
  formTitle: "Choose a dependency scope.",
  sourceHelp: "The unified backend reads dependency pull requests and optionally Dependabot alerts without changing repository code.",
  searchPlaceholder: "Search package, manifest, reason, PR, alert…",
  emptyLabel: "No dependency items match this view.",
  defaultForm: { repo: "", pr_limit: "25", include_alerts: false },
  fields: (health) => {
    const alertsReady = Boolean(health.github_ready || health.github?.token_verified);
    return [
      { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
      { key: "pr_limit", label: "Pull request limit", type: "number", min: 5, max: 60, help: "Read the newest 5–60 open pull requests." },
      { key: "include_alerts", label: "Try Dependabot alerts", type: "checkbox", disabled: !alertsReady, help: alertsReady ? "Adds GitHub security pressure when this repository exposes Dependabot alerts." : "A verified GitHub token is required before alert enrichment can be requested." },
    ];
  },
  validate: (form) => {
    if (!/^[^/\s]+\/[^/\s]+$/.test(form.repo?.trim() || "")) return "Enter a repository in owner/name format before scanning dependencies.";
    const limit = Number(form.pr_limit);
    if (!Number.isInteger(limit) || limit < 5 || limit > 60) return "Pull request limit must be a whole number from 5 through 60.";
    return "";
  },
  serialize: (form, health) => ({ repo: form.repo.trim(), pr_limit: Number(form.pr_limit), include_alerts: Boolean(form.include_alerts) && Boolean(health.github_ready || health.github?.token_verified) }),
  formFromResult: (result) => ({ repo: result.repo || "" }),
  items: (result) => result?.items || [],
  mapItem: (item) => ({
    id: item.key || item.package_name,
    title: item.package_name || item.key || "Dependency",
    meta: [item.ecosystem, item.update_kind, item.runtime_impact].filter(Boolean).join(" · "),
    summary: item.summary || item.reasons?.[0],
    evidence: itemEvidence(item),
    links: itemLinks(item),
    tags: item.manifests || [],
    facts: [
      { label: "Recommendation", value: recommendationLabel(item.recommendation) },
      { label: "Update kind", value: value(item.update_kind) },
      { label: "Runtime impact", value: value(item.runtime_impact) },
      { label: "Staleness", value: `${item.stale_days || 0} ${item.stale_days === 1 ? "day" : "days"}` },
      { label: "Pull requests", value: item.pull_requests?.length || 0 },
      { label: "Alerts", value: item.alerts?.length || 0 },
    ],
    source: item.source || item.ecosystem,
    score: item.score || 0,
    status: recommendationLabel(item.recommendation),
    recommendation: item.recommendation,
    ecosystem: item.ecosystem || "unknown",
    impact: item.runtime_impact || "unknown",
    staleDays: item.stale_days || 0,
    tone: recommendationTone(item.recommendation),
  }),
  dashboard: {
    defaultView: { recommendation: "all", ecosystem: "all", impact: "all", sort: "risk" },
    initialCount: 6,
    itemLabel: "dependencies",
    filters: (items, view) => [
      { key: "recommendation", label: "Decision", value: view.recommendation, options: [{ value: "all", label: "All" }, { value: "update_now", label: "Update now" }, { value: "watch", label: "Watch" }, { value: "ignore_for_now", label: "Ignore for now" }] },
      { key: "ecosystem", label: "Ecosystem", value: view.ecosystem, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.ecosystem).filter(Boolean))].sort().map((ecosystem) => ({ value: ecosystem, label: ecosystem }))] },
      { key: "impact", label: "Impact", value: view.impact, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.impact).filter(Boolean))].sort().map((impact) => ({ value: impact, label: impact }))] },
    ],
    filterItem: (item, view) => (view.recommendation === "all" || item.recommendation === view.recommendation) && (view.ecosystem === "all" || item.ecosystem === view.ecosystem) && (view.impact === "all" || item.impact === view.impact),
    sortItems: (left, right, sort) => {
      if (sort === "recommendation") return recommendationRank(right.recommendation) - recommendationRank(left.recommendation) || right.score - left.score || right.staleDays - left.staleDays;
      if (sort === "stale") return right.staleDays - left.staleDays || right.score - left.score;
      if (sort === "package") return left.title.localeCompare(right.title) || right.score - left.score;
      return right.score - left.score || recommendationRank(right.recommendation) - recommendationRank(left.recommendation) || right.staleDays - left.staleDays;
    },
    sortOptions: [
      { value: "risk", label: "Risk first" },
      { value: "recommendation", label: "Recommendation" },
      { value: "stale", label: "Stalest first" },
      { value: "package", label: "Package name" },
    ],
  },
  metrics: (result, overview, health) => {
    const metrics = result?.metrics || {};
    const counts = overview?.counts || {};
    return result ? [
      { label: "Update now", value: metrics.update_now || 0, footerLeft: "urgent", footerRight: `${metrics.major_updates || 0} major`, tone: "from-orange-700/70 to-red-900/60" },
      { label: "Watch", value: metrics.watch || 0, footerLeft: "monitor", footerRight: "batch later", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Safe defers", value: metrics.ignore_for_now || 0, footerLeft: "ignore", footerRight: "low churn", tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Runtime", value: metrics.runtime_updates || 0, footerLeft: countLabel(metrics.dependency_pull_requests, "dep PR"), footerRight: countLabel(metrics.open_alerts, "alert"), tone: "from-slate-500/70 to-slate-800/60" },
    ] : [
      { label: "Scans", value: counts.scans || health.scan_count || 0, footerLeft: "saved", footerRight: countLabel(counts.repos || health.repo_count, "repo"), tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Update now", value: counts.update_now || health.update_now_count || 0, footerLeft: "saved", footerRight: "urgent", tone: "from-orange-700/70 to-red-900/60" },
      { label: "Watch", value: counts.watch || health.watch_count || 0, footerLeft: "saved", footerRight: "monitor", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Safe defers", value: counts.ignore_for_now || health.ignore_count || 0, footerLeft: "saved", footerRight: "ignore", tone: "from-emerald-700/70 to-teal-900/60" },
    ];
  },
  hero: () => ({ lead: "Dependency work", middle: "ranked by", highlight: "urgency." }),
  status: (result) => ({ label: result?.metrics?.update_now ? "act" : result?.metrics?.watch ? "watch" : result ? "defer" : "—", detail: result ? scanSummary(result) : "Scan a repository to begin", progress: result ? "100%" : "8%" }),
  chips: (result, health) => [result?.repo || "No repository selected", countLabel(result?.metrics?.dependency_pull_requests, "dependency PR"), countLabel(result?.metrics?.open_alerts, "alert"), health.github_ready ? "GitHub verified" : health.github?.token_configured ? "GitHub unverified" : "Token missing"],
  targetSubtitle: (result) => result ? countLabel(result.metrics?.tracked_items, "tracked dependency item") : "Dependency intake",
  historyTitle: (entry) => entry.repo,
  historySummary: (entry) => entry.summary,
  historyMeta: (entry) => `${entry.tracked_items || 0} tracked · ${entry.update_now || 0} now · ${entry.watch || 0} watch · ${entry.ignore_for_now || 0} ignore`,
  historyIdentity: (entry) => `scan ${String(entry.id || "unknown").slice(0, 8)}`,
  historyBadges: (entry) => [
    { label: `${entry.update_now || 0} now`, tone: entry.update_now ? "hot" : "neutral" },
    { label: `${entry.watch || 0} watch`, tone: entry.watch ? "warn" : "neutral" },
    { label: `${entry.ignore_for_now || 0} ignore`, tone: entry.ignore_for_now ? "ok" : "neutral" },
  ],
  historyDashboard: {
    defaultView: { pressure: "all", repo: "all", sort: "newest" },
    initialCount: 6,
    searchPlaceholder: "Search repository, summary, pressure…",
    filters: (entries, view) => [
      { key: "pressure", label: "Pressure", value: view.pressure, options: [{ value: "all", label: "All" }, { value: "now", label: "Update now" }, { value: "watch", label: "Watch" }, { value: "defer", label: "Safe defer" }] },
      { key: "repo", label: "Repository", value: view.repo, options: [{ value: "all", label: "All" }, ...[...new Set(entries.map((entry) => entry.repo).filter(Boolean))].sort().map((repo) => ({ value: repo, label: repo }))] },
    ],
    filterEntry: (entry, view) => {
      const pressureMatches = view.pressure === "all" || (view.pressure === "now" && entry.update_now > 0) || (view.pressure === "watch" && entry.update_now === 0 && entry.watch > 0) || (view.pressure === "defer" && entry.update_now === 0 && entry.watch === 0);
      return pressureMatches && (view.repo === "all" || entry.repo === view.repo);
    },
    sortEntries: (left, right, sort) => {
      if (sort === "oldest") return new Date(left.created_at) - new Date(right.created_at);
      if (sort === "repo") return left.repo.localeCompare(right.repo) || new Date(right.created_at) - new Date(left.created_at);
      if (sort === "pressure") return (right.update_now || 0) - (left.update_now || 0) || (right.watch || 0) - (left.watch || 0) || new Date(right.created_at) - new Date(left.created_at);
      if (sort === "tracked") return (right.tracked_items || 0) - (left.tracked_items || 0) || new Date(right.created_at) - new Date(left.created_at);
      return new Date(right.created_at) - new Date(left.created_at);
    },
    sortOptions: [
      { value: "newest", label: "Newest first" },
      { value: "oldest", label: "Oldest first" },
      { value: "pressure", label: "Highest pressure" },
      { value: "tracked", label: "Most tracked" },
      { value: "repo", label: "Repository" },
    ],
  },
  WorkspaceDetails,
  ChecksDetails,
  SourcesDetails,
};

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "dep-triage_api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />;
  return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />;
}

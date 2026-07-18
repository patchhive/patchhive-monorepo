import { useMemo } from "react";
import {
  Compass,
  Database,
  FolderSearch,
  GitBranch,
  ShieldCheck,
  Workflow,
} from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  CopyMarkdownButton,
  countLabel,
  GuidanceNotice,
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

function Chip({ children, tone = "neutral" }) {
  return <span className={`inline-flex items-center justify-center rounded-full border px-2.5 py-1 text-[10px] leading-none uppercase tracking-wider ${CHIP_TONES[tone] || CHIP_TONES.neutral}`}>{children}</span>;
}

function Fact({ label, value: factValue }) {
  return <div className="surface-inset rounded-xl p-3"><div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</div><div className={`mt-1 font-display text-[18px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value(factValue, "0")}</div></div>;
}

function safetyTone(safety) {
  return safety === "high" ? "ok" : safety === "medium" ? "warn" : "neutral";
}

function safetyRank(safety) {
  return safety === "high" ? 2 : safety === "medium" ? 1 : 0;
}

function targetKind(repoPath) {
  return /^[^/\s]+\/[^/\s]+$/.test(repoPath || "") ? "temporary GitHub clone" : "allowed local path";
}

function lineRange(opportunity) {
  const start = Number(opportunity.line_start || 0);
  const end = Number(opportunity.line_end || 0);
  if (!start) return "file-wide";
  return end && end !== start ? `${start}–${end}` : String(start);
}

function buildScanMarkdown(scan) {
  const metrics = scan?.metrics || {};
  const lines = [
    `# RefactorScout scan for ${scan?.repo_name || scan?.repo_path || "repository"}`,
    "",
    scan?.summary || "RefactorScout cleanup scan.",
    "",
    `- Source: ${scan?.repo_path || "unknown"}`,
    `- Files scanned: ${metrics.files_scanned || 0}`,
    `- Files skipped: ${metrics.files_skipped || 0}`,
    `- Opportunities: ${metrics.opportunities || 0}`,
    `- Opportunities retained: ${metrics.returned_opportunities || scan?.opportunities?.length || 0}`,
    `- High-safety leads: ${metrics.high_safety || 0}`,
    `- Medium-safety leads: ${metrics.medium_safety || 0}`,
  ];

  if (scan?.opportunities?.length) {
    lines.push("", "## Ranked opportunities", "");
    scan.opportunities.slice(0, 10).forEach((opportunity, index) => {
      lines.push(
        `${index + 1}. **${opportunity.title}** — ${opportunity.summary}`,
        `   - ${opportunity.path}:${lineRange(opportunity)} · ${opportunity.kind} · ${opportunity.safety} safety · score ${opportunity.score}`,
        `   - Suggested first move: ${opportunity.suggestion}`,
      );
    });
  }

  if (scan?.warnings?.length) {
    lines.push("", "## Scan warnings", "");
    scan.warnings.forEach((warning) => lines.push(`- ${warning}`));
  }

  lines.push("", "*RefactorScout by [PatchHive](https://github.com/patchhive)*");
  return lines.join("\n");
}

function WorkspaceDetails({ health, onError, result }) {
  if (!result) return null;
  const metrics = result.metrics || {};
  return (
    <div className="mt-8 space-y-6">
      <section className="surface p-5 sm:p-6">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Compass size={12} /> Refactor scan evidence</div>
            <h2 className={`mt-2 font-display text-[27px] font-semibold ${V3_TEXT.strong}`}>{result.repo_name || result.repo_path}</h2>
            <p className={`mt-2 max-w-4xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>{result.summary || "Saved structural cleanup scan."}</p>
          </div>
          <CopyMarkdownButton content={buildScanMarkdown(result)} label="Copy scan Markdown" onError={() => onError("Could not copy the RefactorScout scan summary.")} />
        </div>
        <div className="mt-5 flex flex-wrap gap-2">
          <Chip>{value(result.id).slice(0, 8)}</Chip>
          <Chip tone="ok">read only</Chip>
          <Chip>{targetKind(result.repo_path)}</Chip>
          <Chip tone={metrics.high_safety ? "ok" : metrics.medium_safety ? "warn" : "neutral"}>{metrics.high_safety ? "high-safety queue" : metrics.medium_safety ? "review queue" : "no clear lead"}</Chip>
        </div>
      </section>

      <section className="surface p-5 sm:p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Workflow size={12} /> Complete scan metrics</div>
        <div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
          <Fact label="Files scanned" value={metrics.files_scanned} />
          <Fact label="Files skipped" value={metrics.files_skipped} />
          <Fact label="Opportunities" value={metrics.opportunities} />
          {metrics.opportunities_truncated ? <Fact label="Historically retained" value={metrics.returned_opportunities || result.opportunities?.length || 0} /> : null}
          <Fact label="High safety" value={metrics.high_safety} />
          <Fact label="Medium safety" value={metrics.medium_safety} />
          <Fact label="Large files" value={metrics.large_file_count} />
          <Fact label="Long functions" value={metrics.long_function_count} />
          <Fact label="Repeated literals" value={metrics.repeated_literal_count} />
        </div>
      </section>

      <ScanWarnings warnings={result.warnings || []} />
      {metrics.opportunities_truncated ? <GuidanceNotice label="Historical retention gap">This older saved run retained only {metrics.returned_opportunities || result.opportunities?.length || 0} of {metrics.opportunities} detected opportunities. Rerun the scan to retain and inspect every finding.</GuidanceNotice> : null}

      <section className="surface p-5 sm:p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12} /> Inspection boundary</div>
        <GuidanceNotice label="Filesystem access">Local scans are restricted to the configured allowlist. Public GitHub targets are shallow-cloned into a temporary directory, inspected read-only, and removed after the scan.</GuidanceNotice>
        {health.remote_fs_enabled ? <GuidanceNotice label="Remote callers">Remote filesystem scans are enabled. Confirm this is intentional before exposing the backend beyond localhost.</GuidanceNotice> : null}
      </section>
    </div>
  );
}

function ChecksDetails({ health }) {
  const allowedRoots = health.allowed_roots || [];
  return (
    <section className="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-2">
      <article className="surface p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><FolderSearch size={12} /> Repository-read path</div>
        <div className="mt-5 grid grid-cols-2 gap-3">
          <Fact label="Local roots" value={allowedRoots.length} />
          <Fact label="Local caller" value={health.remote_fs_enabled ? "not required" : "required"} />
          <Fact label="GitHub repos" value="temporary clone" />
          <Fact label="Mode" value="read only" />
        </div>
        <GuidanceNotice label="Allowed roots">{allowedRoots.length ? allowedRoots.join(" · ") : "No readable roots are configured. Local scans remain blocked until REFACTOR_SCOUT_ALLOWED_ROOTS is set."}</GuidanceNotice>
      </article>
      <article className="surface p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Database size={12} /> Product state</div>
        <div className="surface-inset mt-5 rounded-xl p-3"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Database path</div><div className={`mt-1 break-all text-[12px] ${V3_TEXT.strong}`}>{health.db_path || "unknown"}</div></div>
        <div className="mt-4 flex flex-wrap gap-2">
          <Chip tone={health.db_ok ? "ok" : "hot"}>database {health.db_ok ? "ready" : "unavailable"}</Chip>
          <Chip tone={health.auth_enabled ? "ok" : "warn"}>auth {health.auth_enabled ? "enabled" : "disabled"}</Chip>
          <Chip tone={health.remote_fs_enabled ? "warn" : "ok"}>{health.remote_fs_enabled ? "remote FS enabled" : "localhost only"}</Chip>
          <Chip>{countLabel(health.scan_count, "scan")}</Chip>
          <Chip>{countLabel(health.repo_count, "repo")}</Chip>
        </div>
        <div className="mt-3 flex flex-wrap gap-2"><Chip tone="ok">{countLabel(health.high_safety_count, "high-safety lead")}</Chip><Chip tone="warn">{countLabel(health.medium_safety_count, "medium-safety lead")}</Chip></div>
      </article>
    </section>
  );
}

function SourcesDetails({ health }) {
  const allowedRoots = health.allowed_roots || [];
  return (
    <section className="surface mt-6 p-5 sm:p-6">
      <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12} /> Refactor-scan safety</div>
      <div className="mt-4 grid gap-3 sm:grid-cols-3">
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Read only</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>RefactorScout reads source text and saves local evidence. It does not edit files, run tests, commit, push, or open pull requests.</p></div>
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Local allowlist</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>{allowedRoots.length ? `${allowedRoots.length} configured root${allowedRoots.length === 1 ? "" : "s"} constrain local scans.` : "No local roots are configured, so local path scans remain blocked."}</p></div>
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Temporary clones</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Public owner/repository targets are shallow-cloned without credentials and deleted when analysis finishes.</p></div>
      </div>
      <GuidanceNotice label="Result meaning">High safety means the current deterministic heuristic expects a bounded structural cleanup. It is still a review priority, not permission to change code automatically.</GuidanceNotice>
    </section>
  );
}

const config = {
  productKey: "refactor-scout",
  name: "RefactorScout",
  subtitle: "safe refactor queue",
  icon: Compass,
  workspaceLabel: "Refactor map",
  eyebrow: "Structural cleanup",
  queueLabel: "Refactor opportunity queue",
  description: "Inspects an allowed local repository or temporary public GitHub clone, then ranks bounded structural cleanup leads without changing code.",
  runLabel: "Scan repo",
  runningLabel: "Scanning…",
  actionPath: "/scan/local",
  formTitle: "Choose a repository to inspect.",
  sourceHelp: "Enter an allowed local path or a public owner/repository target. RefactorScout reads source files and saves local evidence without modifying the repository.",
  searchPlaceholder: "Search title, path, kind, language, evidence…",
  emptyLabel: "No refactor opportunities match this view.",
  defaultForm: { repo_path: "", max_files: "250" },
  requiresRepo: false,
  fields: [
    { key: "repo_path", label: "Repository path or GitHub repository", placeholder: "/home/you/code/project or owner/repository", primary: true, fullWidth: true, help: "Local paths must be within a configured allowed root. Public GitHub repositories use a temporary shallow clone." },
    { key: "max_files", label: "Maximum source files", type: "number", min: 25, max: 1500, help: "Inspect 25–1500 supported source files." },
  ],
  validate: (form) => {
    if (!form.repo_path?.trim()) return "Enter an allowed local repository path or a public owner/repository target.";
    const maxFiles = Number(form.max_files);
    if (!Number.isInteger(maxFiles) || maxFiles < 25 || maxFiles > 1500) return "Maximum source files must be a whole number from 25 through 1500.";
    return "";
  },
  canRun: (form) => Boolean(form.repo_path?.trim()),
  serialize: (form) => ({ repo_path: form.repo_path.trim(), max_files: Number(form.max_files) }),
  formFromResult: (result) => ({ repo_path: result.repo_path || "" }),
  items: (result) => result?.opportunities || [],
  queueTotal: (result, items) => result?.metrics?.opportunities ?? items.length,
  queueTotalLabel: (result, items) => (Number(result?.metrics?.opportunities || 0) > items.length ? "found" : "tracked"),
  mapItem: (item) => ({
    id: item.id || `${item.kind}-${item.path}-${item.line_start}`,
    title: item.title || "Refactor opportunity",
    meta: [item.path, item.language, item.kind].filter(Boolean).join(" · "),
    summary: item.summary || item.suggestion,
    evidence: [item.suggestion ? `Suggested first move: ${item.suggestion}` : "", ...(item.evidence || [])].filter(Boolean),
    tags: [item.kind, item.language, item.effort].filter(Boolean),
    facts: [
      { label: "Safety", value: value(item.safety) },
      { label: "Effort", value: value(item.effort) },
      { label: "Kind", value: value(item.kind) },
      { label: "Language", value: value(item.language) },
      { label: "Location", value: value(item.path) },
      { label: "Lines", value: lineRange(item) },
    ],
    source: "RefactorScout heuristics",
    score: item.score || 0,
    status: item.safety ? `${item.safety} safety` : "review",
    safety: item.safety || "unknown",
    kind: item.kind || "unknown",
    language: item.language || "unknown",
    effort: item.effort || "unknown",
    path: item.path || "",
    tone: safetyTone(item.safety),
  }),
  dashboard: {
    defaultView: { safety: "all", kind: "all", language: "all", sort: "priority" },
    initialCount: 6,
    itemLabel: "opportunities",
    filters: (items, view) => [
      { key: "safety", label: "Safety", value: view.safety, options: [{ value: "all", label: "All" }, { value: "high", label: "High" }, { value: "medium", label: "Medium" }] },
      { key: "kind", label: "Kind", value: view.kind, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.kind).filter(Boolean))].sort().map((kind) => ({ value: kind, label: kind.replaceAll("_", " ") }))] },
      { key: "language", label: "Language", value: view.language, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.language).filter(Boolean))].sort().map((language) => ({ value: language, label: language }))] },
    ],
    filterItem: (item, view) => (view.safety === "all" || item.safety === view.safety) && (view.kind === "all" || item.kind === view.kind) && (view.language === "all" || item.language === view.language),
    sortItems: (left, right, sort) => {
      if (sort === "score") return right.score - left.score || safetyRank(right.safety) - safetyRank(left.safety);
      if (sort === "path") return left.path.localeCompare(right.path) || right.score - left.score;
      if (sort === "safety") return safetyRank(right.safety) - safetyRank(left.safety) || right.score - left.score;
      if (sort === "effort") return left.effort.localeCompare(right.effort) || right.score - left.score;
      return safetyRank(right.safety) - safetyRank(left.safety) || right.score - left.score || left.path.localeCompare(right.path);
    },
    sortOptions: [{ value: "priority", label: "Review priority" }, { value: "score", label: "Highest score" }, { value: "safety", label: "Safest first" }, { value: "path", label: "Path" }, { value: "effort", label: "Effort" }],
  },
  metrics: (result, overview, health) => {
    const metrics = result?.metrics || {};
    return result ? [
      { label: "High safety", value: metrics.high_safety || 0, footerLeft: "bounded", footerRight: "first", tone: "from-emerald-600/70 to-teal-800/60" },
      { label: "Medium safety", value: metrics.medium_safety || 0, footerLeft: "review", footerRight: "carefully", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Files scanned", value: metrics.files_scanned || 0, footerLeft: `${metrics.files_skipped || 0} skipped`, footerRight: "source files", tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Hotspots", value: (metrics.large_file_count || 0) + (metrics.long_function_count || 0) + (metrics.repeated_literal_count || 0), footerLeft: `${metrics.large_file_count || 0} files`, footerRight: `${metrics.long_function_count || 0} functions`, tone: "from-teal-600/70 to-emerald-900/60" },
    ] : [
      { label: "High safety", value: overview.high_safety_count || health.high_safety_count || 0, footerLeft: "saved", footerRight: "priorities", tone: "from-emerald-600/70 to-teal-800/60" },
      { label: "Medium safety", value: overview.medium_safety_count || health.medium_safety_count || 0, footerLeft: "saved", footerRight: "review", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Scans", value: overview.scan_count || health.scan_count || 0, footerLeft: "saved", footerRight: countLabel(overview.repo_count || health.repo_count, "repo"), tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Allowed roots", value: (overview.allowed_roots || health.allowed_roots || []).length, footerLeft: "local", footerRight: "boundary", tone: "from-teal-600/70 to-emerald-900/60" },
    ];
  },
  hero: () => ({ lead: "Refactor evidence", middle: "ranked for", highlight: "review." }),
  assessmentLabel: "Current assessment",
  status: (result) => {
    const top = [...(result?.opportunities || [])].sort((left, right) => safetyRank(right.safety) - safetyRank(left.safety) || (right.score || 0) - (left.score || 0))[0];
    if (!result) return { label: "waiting", detail: "Choose a repository to build a bounded cleanup queue.", progress: "8%" };
    if (!top) return { label: "clear", detail: result.summary || "No clear low-risk cleanup lead was found.", progress: "100%" };
    return { label: "review priority", detail: `${top.title} leads the queue at ${top.score}/100 with ${top.safety || "unknown"} safety.`, progress: `${Math.max(10, Math.min(100, top.score || 0))}%` };
  },
  priorityLabel: "Review priorities",
  priorityEmptyLabel: "No active refactor priorities in this scan.",
  priorityItems: (items) => [...items].sort((left, right) => safetyRank(right.safety) - safetyRank(left.safety) || right.score - left.score),
  chips: (result, health) => [result?.repo_path || "No repository selected", result ? targetKind(result.repo_path) : "local or GitHub target", health.remote_fs_enabled ? "remote callers enabled" : "localhost only", "read only"],
  targetLabel: (result, form, overview) => result?.repo_name || result?.repo_path || form.repo_path || overview.last_repo || "no repository selected",
  targetSubtitle: (result) => {
    if (!result) return "local path or public GitHub repository";
    const total = Number(result.metrics?.opportunities || 0);
    const retained = Number(result.metrics?.returned_opportunities || result.opportunities?.length || 0);
    return total > retained
      ? `${targetKind(result.repo_path)} · historical run retained ${retained} / ${total}`
      : `${targetKind(result.repo_path)} · ${countLabel(total, "opportunity")}`;
  },
  connectionName: "Filesystem",
  connectionLabel: (health) => health.db_ok ? "Filesystem ready" : "Filesystem unavailable",
  connectionValue: (health) => health.remote_fs_enabled ? "remote enabled" : "local only",
  historyTitle: (entry) => entry.repo_name || entry.repo_path || "Saved repository scan",
  historySummary: (entry) => entry.summary,
  historyMeta: (entry) => `${countLabel(entry.opportunities, "opportunity")} · ${countLabel(entry.high_safety, "high-safety lead")} · ${countLabel(entry.medium_safety, "medium-safety lead")}`,
  historyIdentity: (entry) => `scan ${String(entry.id || "unknown").slice(0, 8)}`,
  historySearchText: (entry) => `${entry.repo_path || ""} ${entry.opportunities || 0} opportunities ${entry.high_safety || 0} high safety ${entry.medium_safety || 0} medium safety`,
  historyBadges: (entry) => [{ label: countLabel(entry.high_safety, "high"), tone: entry.high_safety ? "ok" : "neutral" }, { label: countLabel(entry.medium_safety, "medium"), tone: entry.medium_safety ? "warn" : "neutral" }, { label: countLabel(entry.opportunities, "lead"), tone: entry.opportunities ? "ok" : "neutral" }],
  historyDashboard: {
    defaultView: { safety: "all", repo: "all", sort: "newest" },
    initialCount: 6,
    searchPlaceholder: "Search repository, summary, opportunity count…",
    filters: (items, view) => [
      { key: "safety", label: "Safety", value: view.safety, options: [{ value: "all", label: "All" }, { value: "high", label: "Has high-safety leads" }, { value: "medium", label: "Medium only" }, { value: "empty", label: "No leads" }] },
      { key: "repo", label: "Repository", value: view.repo, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((entry) => entry.repo_name || entry.repo_path).filter(Boolean))].sort().map((repo) => ({ value: repo, label: repo }))] },
    ],
    filterEntry: (entry, view) => {
      const safetyMatches = view.safety === "all" || (view.safety === "high" && entry.high_safety > 0) || (view.safety === "medium" && entry.high_safety === 0 && entry.medium_safety > 0) || (view.safety === "empty" && entry.opportunities === 0);
      return safetyMatches && (view.repo === "all" || (entry.repo_name || entry.repo_path) === view.repo);
    },
    sortEntries: (left, right, sort) => {
      if (sort === "oldest") return new Date(left.created_at) - new Date(right.created_at);
      if (sort === "opportunities") return (right.opportunities || 0) - (left.opportunities || 0) || new Date(right.created_at) - new Date(left.created_at);
      if (sort === "safety") return (right.high_safety || 0) - (left.high_safety || 0) || (right.medium_safety || 0) - (left.medium_safety || 0);
      if (sort === "repo") return (left.repo_name || left.repo_path || "").localeCompare(right.repo_name || right.repo_path || "") || new Date(right.created_at) - new Date(left.created_at);
      return new Date(right.created_at) - new Date(left.created_at);
    },
    sortOptions: [{ value: "newest", label: "Newest first" }, { value: "oldest", label: "Oldest first" }, { value: "safety", label: "Safest first" }, { value: "opportunities", label: "Most opportunities" }, { value: "repo", label: "Repository" }],
  },
  WorkspaceDetails,
  ChecksDetails,
  SourcesDetails,
};

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "refactor-scout_api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />;
  return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />;
}

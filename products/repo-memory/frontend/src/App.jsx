import { useMemo } from "react";
import { BookOpenCheck, BrainCircuit, Database, GitBranch, History, ShieldCheck } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  CopyMarkdownButton,
  GitHubPermissionGuidance,
  GuidanceNotice,
  IntegratedProductApp,
  ProductLoginScreen,
  ProductShell,
  V3_TEXT,
  countLabel,
} from "@patchhivehq/ui-v3";
import FailGuardPanel from "./FailGuardPanel.jsx";
import { MemoryLibraryPanel, PromptPackPanel } from "./MemoryPanels.jsx";
import { API } from "./config.js";

const CHIP_TONES = {
  hot: "border-red-900/30 bg-red-900/10 text-red-800 dark:border-red-400/25 dark:bg-red-500/10 dark:text-red-300",
  warn: "border-amber-900/30 bg-amber-900/10 text-amber-800 dark:border-amber-400/25 dark:bg-amber-500/10 dark:text-amber-300",
  ok: "border-emerald-900/30 bg-emerald-900/10 text-emerald-800 dark:border-emerald-400/25 dark:bg-emerald-500/10 dark:text-emerald-300",
  neutral: "border-stone-800/20 bg-stone-800/5 text-stone-700 dark:border-stone-400/20 dark:bg-stone-400/5 dark:text-stone-300",
};

function Chip({ children, tone = "neutral" }) {
  return <span className={`inline-flex items-center justify-center rounded-full border px-2.5 py-1 text-[10px] leading-none uppercase tracking-wider ${CHIP_TONES[tone]}`}>{children}</span>;
}

function Fact({ label, value }) {
  return <div className="surface-inset rounded-xl p-3"><div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</div><div className={`mt-1 font-display text-[18px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value ?? 0}</div></div>;
}

function dispositionTone(entry) {
  if (entry.disposition === "policy" || entry.pinned) return "ok";
  if (entry.disposition === "suppressed") return "neutral";
  if (entry.kind === "failure_pattern") return "warn";
  return "neutral";
}

function kindLabel(kind) {
  return String(kind || "memory").replaceAll("_", " ");
}

function hasLinkedEvidence(entry) {
  return (entry.evidence || []).some((evidence) => evidence.url);
}

function frequencyFact(entry) {
  if (entry.kind !== "hotspot") {
    return { label: "Frequency", value: entry.frequency || 0 };
  }
  return hasLinkedEvidence(entry)
    ? { label: "Merged PRs touching path", value: entry.frequency || 0 }
    : { label: "Historical path touches", value: entry.frequency || 0 };
}

function evidenceText(evidence) {
  return [evidence.source_type, evidence.title, evidence.path, evidence.excerpt].filter(Boolean).join(" · ");
}

function mappedEvidence(entry) {
  const evidence = (entry.evidence || []).map(evidenceText);
  if (entry.kind === "hotspot" && evidence.length === 0) {
    return ["This historical ingest predates linked hotspot evidence. Re-ingest the repository to rebuild this memory with supporting merged-PR links."];
  }
  return evidence;
}

function buildRunMarkdown(result) {
  if (!result) return "";
  const summary = result.summary || {};
  const lines = [
    `# RepoMemory ingest · ${result.repo}`,
    "",
    summary.top_memory || "RepoMemory ingest completed.",
    "",
    `- Run: ${result.id}`,
    `- Merged PRs analyzed: ${summary.merged_prs_analyzed || 0}`,
    `- Review feedback items: ${summary.review_feedback_items || 0}`,
    `- Closed issues analyzed: ${summary.closed_issues_analyzed || 0}`,
    `- Partial reads: ${summary.partial_read_warnings || 0}`,
    `- Memories created: ${summary.memories_created || 0}`,
    `- Conventions: ${summary.conventions || 0}`,
    `- Failure patterns: ${summary.failures || 0}`,
    `- Hotspots: ${summary.hotspots || 0}`,
  ];
  if (result.entries?.length) {
    lines.push("", "## Durable memories", "");
    result.entries.forEach((entry) => lines.push(`- **${entry.title}** — ${entry.prompt_line}`));
  }
  lines.push("", "*RepoMemory by [PatchHive](https://github.com/patchhive)*");
  return lines.join("\n");
}

function WorkspaceDetails({ health, onError, result }) {
  if (!result) return null;
  const summary = result.summary || {};
  return <div className="mt-8 space-y-6"><section className="surface p-5 sm:p-6"><div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><BrainCircuit size={12} /> Memory ingest evidence</div><h2 className={`mt-2 font-display text-[27px] font-semibold ${V3_TEXT.strong}`}>{result.repo}</h2><p className={`mt-2 max-w-4xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>{summary.top_memory || "No strong memory signal crossed the confidence threshold."}</p></div><div className="flex shrink-0 flex-wrap gap-2"><Chip tone={result.entries?.length ? "ok" : "warn"}>{countLabel(result.entries?.length, "memory", "memories")}</Chip><CopyMarkdownButton content={buildRunMarkdown(result)} label="Copy ingest Markdown" onError={() => onError("Could not copy the RepoMemory ingest summary.")} /></div></div><div className="mt-5 flex flex-wrap gap-2"><Chip>run {String(result.id || "").slice(0, 8)}</Chip><Chip>{result.params?.since_days || 0} day lookback</Chip><Chip>{result.created_at ? new Date(result.created_at).toLocaleString() : "saved"}</Chip></div></section>
    {summary.partial_read_warnings ? <GuidanceNotice label="Partial GitHub evidence">{countLabel(summary.partial_read_warnings, "GitHub read")} could not be completed. The saved memory remains usable, but rerun with stronger repository access before treating coverage as complete.</GuidanceNotice> : null}
    <section className="surface p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><History size={12} /> Complete ingest metrics</div><div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5"><Fact label="Merged PRs" value={summary.merged_prs_analyzed} /><Fact label="Review feedback" value={summary.review_feedback_items} /><Fact label="Closed issues" value={summary.closed_issues_analyzed} /><Fact label="Memories" value={summary.memories_created} /><Fact label="Conventions" value={summary.conventions} /><Fact label="Failures" value={summary.failures} /><Fact label="Hotspots" value={summary.hotspots} /><Fact label="Partial reads" value={summary.partial_read_warnings} /></div></section>
    <section className="surface p-5 sm:p-6"><div className="flex flex-wrap items-center justify-between gap-3"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><BookOpenCheck size={12} /> Prompt pack preview</div><div className={`mt-1 font-display text-[24px] ${V3_TEXT.strong}`}>Agent-ready context.</div></div>{result.prompt_pack ? <CopyMarkdownButton content={result.prompt_pack} label="Copy prompt pack" onError={() => onError("Could not copy the prompt pack.")} /> : null}</div>{result.prompt_pack ? <details className="surface-inset mt-5 rounded-xl p-4"><summary className={`cursor-pointer text-[12px] font-semibold ${V3_TEXT.strong}`}>Preview prompt pack Markdown</summary><pre className={`mt-4 max-h-[520px] overflow-auto whitespace-pre-wrap text-[11px] leading-relaxed ${V3_TEXT.body}`}>{result.prompt_pack}</pre></details> : <div className={`py-12 text-center text-[12px] ${V3_TEXT.mute}`}>No prompt pack was produced for this run.</div>}</section>
    {!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub rejected or could not verify the configured token." : "Configure a GitHub token before ingesting repository history."} Existing stored memories and FailGuard review remain available without a live GitHub read.</GitHubPermissionGuidance> : null}
  </div>;
}

function ChecksDetails({ health }) {
  const counts = health.counts || {};
  return <section className="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-2"><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitBranch size={12} /> GitHub history-read path</div><div className="mt-5 grid grid-cols-2 gap-3"><Fact label="Token" value={health.github?.token_verified ? "verified" : health.github?.token_configured ? "unverified" : "missing"} /><Fact label="Repositories" value="read" /><Fact label="Pull requests" value="read" /><Fact label="Issues" value="read" /></div><GitHubPermissionGuidance>Repository metadata, merged pull requests, reviews, review comments, changed files, and closed issues power ingest. RepoMemory does not write to GitHub.</GitHubPermissionGuidance></article><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Database size={12} /> Product state</div><div className="surface-inset mt-5 rounded-xl p-3"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Database path</div><div className={`mt-1 break-all text-[12px] ${V3_TEXT.strong}`}>{health.db_path || "unknown"}</div></div><div className="mt-4 flex flex-wrap gap-2"><Chip tone={health.db_ok ? "ok" : "hot"}>database {health.db_ok ? "ready" : "unavailable"}</Chip><Chip tone={health.auth_enabled ? "ok" : "warn"}>auth {health.auth_enabled ? "enabled" : "disabled"}</Chip><Chip tone="ok">{countLabel(counts.runs, "run")}</Chip><Chip>{countLabel(counts.repos, "repo")}</Chip><Chip>{countLabel(counts.memories, "memory", "memories")}</Chip></div></article></section>;
}

function SourcesDetails({ health }) {
  return <section className="surface mt-6 p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12} /> Memory safety</div><div className="mt-4 grid gap-3 sm:grid-cols-3"><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>GitHub read only</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Ingest reads merged history and review evidence. It does not comment, change code, open pull requests, or mutate repositories.</p></div><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Durable local curation</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Pin, signal, and suppress choices update RepoMemory's local SQLite context. They do not alter original GitHub evidence.</p></div><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>FailGuard approval</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Suggested bad outcomes remain candidates until an operator promotes them into durable failure-pattern memory.</p></div></div>{!health.github_ready ? <GitHubPermissionGuidance>Add a GitHub token with repository metadata, Pull requests, and Issues read access before ingesting live history.</GitHubPermissionGuidance> : null}</section>;
}

const config = {
  productKey: "repo-memory",
  name: "RepoMemory",
  subtitle: "durable context",
  icon: BrainCircuit,
  workspaceLabel: "Memory core",
  eyebrow: "Repository knowledge",
  queueLabel: "Durable memories",
  description: "Turns merged history, review feedback, recurring failures, and file hotspots into reusable repo context for humans and PatchHive products.",
  runLabel: "Ingest repo",
  runningLabel: "Building memory…",
  actionPath: "/ingest",
  formTitle: "Choose a history scope.",
  sourceHelp: "The unified backend needs repository, pull-request, review, file, and issue read access. RepoMemory remains GitHub read-only.",
  searchPlaceholder: "Search memory, kind, prompt, evidence…",
  emptyLabel: "No durable memories were created in this ingest.",
  defaultForm: { repo: "", merged_pr_limit: "18", issue_limit: "24", since_days: "180" },
  fields: [
    { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
    { key: "merged_pr_limit", label: "Merged pull requests", type: "number", min: 5, max: 40, help: "Read the newest 5–40 merged pull requests." },
    { key: "issue_limit", label: "Closed issues", type: "number", min: 5, max: 40, help: "Read the newest 5–40 closed non-PR issues." },
    { key: "since_days", label: "Lookback days", type: "number", min: 30, max: 730 },
  ],
  validate: (form) => /^[^/\s]+\/[^/\s]+$/.test(form.repo?.trim() || "") ? "" : "Enter a repository in owner/name format before ingesting.",
  serialize: (form) => ({ repo: form.repo.trim(), merged_pr_limit: Number(form.merged_pr_limit) || 18, issue_limit: Number(form.issue_limit) || 24, since_days: Number(form.since_days) || 180 }),
  formFromResult: (result) => ({ repo: result.repo || "", merged_pr_limit: String(result.params?.merged_pr_limit || 18), issue_limit: String(result.params?.issue_limit || 24), since_days: String(result.params?.since_days || 180) }),
  historyItems: (payload) => payload?.history || [],
  items: (result) => result?.entries || [],
  mapItem: (entry) => ({
    id: entry.memory_ref || entry.id,
    title: entry.title || "Durable memory",
    meta: `${kindLabel(entry.kind)} · ${entry.repo}`,
    summary: entry.detail,
    evidence: mappedEvidence(entry),
    links: (entry.evidence || []).filter((evidence) => evidence.url).map((evidence, index) => ({ label: evidence.title || `Evidence ${index + 1}`, url: evidence.url })),
    tags: entry.tags || [],
    facts: [{ label: "Kind", value: kindLabel(entry.kind) }, { label: "Disposition", value: entry.disposition || "signal" }, { label: "Confidence", value: `${Math.round(entry.confidence || 0)}%` }, frequencyFact(entry), { label: "Pinned", value: entry.pinned ? "yes" : "no" }],
    source: "RepoMemory ingest",
    score: Math.round(entry.confidence || 0),
    status: entry.disposition || "signal",
    kind: entry.kind || "memory",
    disposition: entry.disposition || "signal",
    confidence: entry.confidence || 0,
    frequency: entry.frequency || 0,
    tone: dispositionTone(entry),
  }),
  dashboard: {
    defaultView: { kind: "all", disposition: "all", sort: "confidence" }, initialCount: 6, itemLabel: "memories",
    filters: (items, view) => [{ key: "kind", label: "Kind", value: view.kind, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.kind).filter(Boolean))].sort().map((kind) => ({ value: kind, label: kindLabel(kind) }))] }, { key: "disposition", label: "Disposition", value: view.disposition, options: [{ value: "all", label: "All" }, { value: "policy", label: "Policy" }, { value: "signal", label: "Signal" }, { value: "suppressed", label: "Suppressed" }] }],
    filterItem: (item, view) => (view.kind === "all" || item.kind === view.kind) && (view.disposition === "all" || item.disposition === view.disposition),
    sortItems: (left, right, sort) => sort === "frequency" ? right.frequency - left.frequency : sort === "title" ? left.title.localeCompare(right.title) : right.confidence - left.confidence,
    sortOptions: [{ value: "confidence", label: "Highest confidence" }, { value: "frequency", label: "Most repeated" }, { value: "title", label: "Title · A to Z" }],
  },
  metrics: (result, overview, health) => {
    const summary = result?.summary || {};
    const counts = overview?.counts || health.counts || {};
    return result ? [
      { label: "Memories", value: summary.memories_created || 0, footerLeft: countLabel(summary.conventions, "convention"), footerRight: countLabel(summary.failures, "failure"), tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Merged PRs", value: summary.merged_prs_analyzed || 0, footerLeft: "history", footerRight: "read", tone: "from-violet-700/70 to-purple-900/60" },
      { label: "Review asks", value: summary.review_feedback_items || 0, footerLeft: "feedback", footerRight: "clustered", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Partial reads", value: summary.partial_read_warnings || 0, footerLeft: "coverage", footerRight: summary.partial_read_warnings ? "review" : "complete", tone: summary.partial_read_warnings ? "from-orange-700/70 to-red-900/60" : "from-slate-500/70 to-slate-800/60" },
    ] : [
      { label: "Memories", value: counts.memories || 0, footerLeft: "durable", footerRight: countLabel(counts.runs, "ingest"), tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Repos", value: counts.repos || 0, footerLeft: "known", footerRight: "history", tone: "from-violet-700/70 to-purple-900/60" },
      { label: "GitHub", value: health.github_ready ? "on" : "off", footerLeft: "history", footerRight: "read", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "FailGuard", value: "review", footerLeft: "operator", footerRight: "promoted", tone: "from-slate-500/70 to-slate-800/60" },
    ];
  },
  hero: () => ({ lead: "Repository history", middle: "becomes", highlight: "durable context." }),
  status: (result) => ({ label: result ? result.entries?.length ? "memory-ready" : "baseline" : "idle", detail: result?.summary?.top_memory || "Ingest a repository to build durable context.", progress: result ? "100%" : "8%" }),
  priorityLabel: "Strongest memories",
  priorityEmptyLabel: "No durable memories were created in this ingest.",
  priorityItems: (items) => [...items].sort((left, right) => Number(right.score || 0) - Number(left.score || 0)),
  chips: (result, health) => [result?.repo || "No repository selected", result ? `${result.params?.since_days || 0} day lookback` : "Merged history", result ? countLabel(result.summary?.partial_read_warnings, "partial read") : health.github_ready ? "GitHub verified" : "GitHub unavailable", "FailGuard reviewed"],
  targetSubtitle: (result) => result ? `Run ${String(result.id).slice(0, 8)} · ${countLabel(result.entries?.length, "memory", "memories")}` : "Durable repository context",
  historyTitle: (entry) => entry.repo,
  historySummary: (entry) => entry.top_memory,
  historyMeta: (entry) => `${countLabel(entry.memories_created, "memory", "memories")} · ${countLabel(entry.partial_read_warnings, "partial read")}`,
  historyIdentity: (entry) => `ingest ${String(entry.id || "unknown").slice(0, 8)}`,
  historyBadges: (entry) => [{ label: countLabel(entry.memories_created, "memory", "memories"), tone: entry.memories_created ? "ok" : "neutral" }, { label: countLabel(entry.failures, "failure"), tone: entry.failures ? "warn" : "neutral" }, { label: countLabel(entry.partial_read_warnings, "partial"), tone: entry.partial_read_warnings ? "hot" : "neutral" }],
  historySearchText: (entry) => `${entry.top_memory} ${entry.conventions} ${entry.failures} ${entry.hotspots}`,
  historyDashboard: {
    defaultView: { repo: "all", coverage: "all", sort: "newest" }, initialCount: 6, searchPlaceholder: "Search repository, memory, run…",
    filters: (entries, view) => [{ key: "repo", label: "Repository", value: view.repo, options: [{ value: "all", label: "All" }, ...[...new Set(entries.map((entry) => entry.repo).filter(Boolean))].sort().map((repo) => ({ value: repo, label: repo }))] }, { key: "coverage", label: "Coverage", value: view.coverage, options: [{ value: "all", label: "All" }, { value: "complete", label: "Complete" }, { value: "partial", label: "Partial" }] }],
    filterEntry: (entry, view) => (view.repo === "all" || entry.repo === view.repo) && (view.coverage === "all" || (view.coverage === "partial" ? entry.partial_read_warnings > 0 : !entry.partial_read_warnings)),
    sortEntries: (left, right, sort) => sort === "oldest" ? new Date(left.created_at) - new Date(right.created_at) : sort === "memories" ? (right.memories_created || 0) - (left.memories_created || 0) : sort === "repo" ? left.repo.localeCompare(right.repo) : new Date(right.created_at) - new Date(left.created_at),
    sortOptions: [{ value: "newest", label: "Newest first" }, { value: "oldest", label: "Oldest first" }, { value: "memories", label: "Most memories" }, { value: "repo", label: "Repository" }],
  },
  extraTabs: [
    { id: "memory", label: "Memory", render: (props) => <MemoryLibraryPanel {...props} /> },
    { id: "failguard", label: "FailGuard", render: (props) => <FailGuardPanel {...props} /> },
    { id: "packs", label: "Prompt packs", render: (props) => <PromptPackPanel {...props} /> },
  ],
  WorkspaceDetails,
  ChecksDetails,
  SourcesDetails,
};

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "repo-memory_api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />;
  return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />;
}

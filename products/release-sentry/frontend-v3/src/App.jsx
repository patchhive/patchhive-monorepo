import { useMemo } from "react";
import {
  Database,
  GitBranch,
  PackageCheck,
  Rocket,
  ShieldCheck,
  Workflow,
} from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
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

function decisionTone(input) {
  const normalized = String(input || "").toLowerCase();
  if (["hold", "block", "blocked", "failed", "error"].includes(normalized)) return "hot";
  if (["watch", "warn", "warning", "pending"].includes(normalized)) return "warn";
  if (["ready", "pass", "passed", "ok", "verified"].includes(normalized)) return "ok";
  return "neutral";
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

function WorkspaceDetails({ health, result }) {
  if (!result) return null;
  const metrics = result.metrics || {};

  return (
    <div className="mt-8 space-y-6">
      <section className="surface p-5 sm:p-6">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><PackageCheck size={12} /> Release candidate evidence</div>
            <h2 className={`mt-2 font-display text-[27px] font-semibold ${V3_TEXT.strong}`}>{result.title || `${result.repo} release readiness`}</h2>
            <p className={`mt-2 max-w-4xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>{result.summary}</p>
          </div>
          <Chip tone={decisionTone(result.decision)}>{result.decision || "pending"}</Chip>
        </div>
        <div className="mt-5 flex flex-wrap gap-2">
          <Chip>{result.repo}</Chip>
          <Chip>branch: {value(result.branch)}</Chip>
          <Chip>version: {value(result.target_version, "current")}</Chip>
          <Chip>tag: {value(result.target_tag, "next release")}</Chip>
          <Chip tone={decisionTone(result.decision)}>score: {value(result.score, "0")}/100</Chip>
        </div>
      </section>

      <section className="surface p-5 sm:p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Workflow size={12} /> Complete release metrics</div>
        <div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
          <Fact label="Checks" value={metrics.checks} />
          <Fact label="Passed" value={metrics.passed} />
          <Fact label="Warned" value={metrics.warned} />
          <Fact label="Blocked" value={metrics.blocked} />
          <Fact label="Workflow runs" value={metrics.workflow_runs} />
          <Fact label="CI successes" value={metrics.workflow_successes} />
          <Fact label="CI failures" value={metrics.workflow_failures} />
          <Fact label="CI pending" value={metrics.workflow_pending} />
          <Fact label="Release blockers" value={metrics.release_blockers} />
          <Fact label="Tags seen" value={metrics.tags_seen} />
          <Fact label="Releases seen" value={metrics.releases_seen} />
          <Fact label="Evidence links" value={(result.checks || []).reduce((total, check) => total + (check.links?.length || 0), 0)} />
        </div>
      </section>

      <ScanWarnings warnings={result.warnings || []} />

      {!health.github_ready ? (
        <GitHubPermissionGuidance>
          {health.github?.token_configured
            ? "GitHub could not verify the configured token. Check startup evidence before trusting release coverage."
            : "Configure repository contents, releases, pull-request, issue, and Actions read access before a live release check."}
        </GitHubPermissionGuidance>
      ) : null}
    </div>
  );
}

function ChecksDetails({ health }) {
  const github = health.github || {};
  return (
    <section className="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-2">
      <article className="surface p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitBranch size={12} /> GitHub release-read path</div>
        <div className="mt-5 grid grid-cols-2 gap-3">
          <Fact label="Token" value={github.token_verified ? "verified" : github.token_configured ? "unverified" : "missing"} />
          <Fact label="Repository" value="read" />
          <Fact label="Actions" value="read" />
          <Fact label="Mode" value="read only" />
        </div>
        <GitHubPermissionGuidance>Repository metadata, contents, releases, tags, issues, pull requests, and Actions reads power the release decision. ReleaseSentry does not publish, tag, deploy, or mutate GitHub.</GitHubPermissionGuidance>
      </article>
      <article className="surface p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Database size={12} /> Product state</div>
        <div className="surface-inset mt-5 rounded-xl p-3"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Database path</div><div className={`mt-1 break-all text-[12px] ${V3_TEXT.strong}`}>{health.db_path || "unknown"}</div></div>
        <div className="mt-4 flex flex-wrap gap-2">
          <Chip tone={health.db_ok ? "ok" : "hot"}>database {health.db_ok ? "ready" : "unavailable"}</Chip>
          <Chip tone={health.auth_enabled ? "ok" : "warn"}>auth {health.auth_enabled ? "enabled" : "disabled"}</Chip>
          <Chip tone="ok">{value(health.run_count, "0")} runs</Chip>
          <Chip>{value(health.repo_count, "0")} repos</Chip>
        </div>
        <div className="mt-3 flex flex-wrap gap-2">
          <Chip tone="ok">{value(health.ready_count, "0")} ready</Chip>
          <Chip tone={health.watch_count ? "warn" : "neutral"}>{value(health.watch_count, "0")} watch</Chip>
          <Chip tone={health.hold_count ? "hot" : "neutral"}>{value(health.hold_count, "0")} hold</Chip>
        </div>
      </article>
    </section>
  );
}

function SourcesDetails({ health }) {
  return (
    <section className="surface mt-6 p-5 sm:p-6">
      <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12} /> Release-check safety</div>
      <div className="mt-4 grid gap-3 sm:grid-cols-3">
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Read only</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>ReleaseSentry reads release evidence and saves its decision locally. It does not publish, tag, or deploy.</p></div>
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Flexible target</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Leave version and tag blank to inspect current release posture, or set them to validate a specific candidate.</p></div>
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Explicit blockers</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>The blocker-label set controls which open GitHub issues create release pressure for this run.</p></div>
      </div>
      {!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub token verification failed. Review startup evidence before running a release check." : "Add a GitHub token with repository, contents, releases, issue, and Actions read access."}</GitHubPermissionGuidance> : null}
    </section>
  );
}

const config = {
  productKey: "release-sentry",
  name: "ReleaseSentry",
  subtitle: "release gate",
  icon: Rocket,
  workspaceLabel: "Release gate",
  eyebrow: "Ship decision",
  queueLabel: "Release evidence",
  description: "Combines tags, changelog state, workflow health, blockers, and release pressure into one ship or hold decision.",
  runLabel: "Check release",
  runningLabel: "Checking…",
  actionPath: "/check/github/release",
  formTitle: "Choose a release candidate.",
  sourceHelp: "The unified backend needs repository contents, releases, issue, pull-request, and Actions read access. ReleaseSentry remains read-only.",
  searchPlaceholder: "Search check, status, blocker, evidence…",
  emptyLabel: "No release evidence has been returned yet.",
  defaultForm: {
    repo: "",
    branch: "",
    target_version: "",
    target_tag: "",
    changelog_path: "CHANGELOG.md",
    workflow_run_limit: "20",
    blocker_labels: "release-blocker, blocker, critical, regression",
  },
  fields: [
    { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
    { key: "branch", label: "Branch", placeholder: "default branch" },
    { key: "target_version", label: "Target version", placeholder: "1.4.0" },
    { key: "target_tag", label: "Target tag", placeholder: "v1.4.0" },
    { key: "changelog_path", label: "Changelog path", placeholder: "CHANGELOG.md" },
    { key: "workflow_run_limit", label: "Workflow runs", type: "number", min: 5, max: 100 },
    { key: "blocker_labels", label: "Blocker labels", placeholder: "release-blocker, blocker, critical" },
  ],
  validate: (form) => {
    if (!/^[^/\s]+\/[^/\s]+$/.test(form.repo?.trim() || "")) return "Enter a repository in owner/name format before checking a release.";
    const limit = Number(form.workflow_run_limit);
    if (!Number.isInteger(limit) || limit < 5 || limit > 100) return "Workflow runs must be a whole number from 5 through 100.";
    return "";
  },
  serialize: (form) => ({
    repo: form.repo.trim(),
    branch: form.branch.trim(),
    target_version: form.target_version.trim(),
    target_tag: form.target_tag.trim(),
    changelog_path: form.changelog_path.trim(),
    workflow_run_limit: Number(form.workflow_run_limit),
    blocker_labels: String(form.blocker_labels || "").split(",").map((item) => item.trim()).filter(Boolean),
  }),
  formFromResult: (result) => ({
    repo: result.repo || "",
    branch: result.branch || "",
    target_version: result.target_version || "",
    target_tag: result.target_tag || "",
  }),
  items: (result) => result?.checks || [],
  mapItem: (item) => ({
    id: item.key || item.label,
    title: item.label || "Release check",
    meta: item.status || "evidence",
    summary: item.detail,
    evidence: item.evidence || [],
    links: item.links || [],
    score: decisionTone(item.status) === "ok" ? "✓" : decisionTone(item.status) === "hot" ? "!" : "?",
    status: item.status || "check",
    tone: decisionTone(item.status),
  }),
  dashboard: {
    defaultView: { status: "all", evidence: "all", sort: "status" },
    initialCount: 6,
    itemLabel: "checks",
    filters: (_items, view) => [
      { key: "status", label: "Status", value: view.status, options: [{ value: "all", label: "All" }, { value: "block", label: "Blocked" }, { value: "warn", label: "Warnings" }, { value: "pass", label: "Passed" }] },
      { key: "evidence", label: "Evidence", value: view.evidence, options: [{ value: "all", label: "All" }, { value: "with", label: "With evidence" }, { value: "without", label: "Without evidence" }, { value: "links", label: "With links" }] },
    ],
    filterItem: (item, view) => {
      if (view.status !== "all" && item.status !== view.status) return false;
      if (view.evidence === "with" && !item.evidence?.length) return false;
      if (view.evidence === "without" && item.evidence?.length) return false;
      if (view.evidence === "links" && !item.links?.length) return false;
      return true;
    },
    sortItems: (left, right, sort) => {
      if (sort === "title-asc") return left.title.localeCompare(right.title);
      if (sort === "title-desc") return right.title.localeCompare(left.title);
      if (sort === "evidence-desc") return (right.evidence?.length || 0) - (left.evidence?.length || 0) || left.title.localeCompare(right.title);
      const priority = { block: 0, warn: 1, pass: 2 };
      return (priority[left.status] ?? 3) - (priority[right.status] ?? 3) || left.title.localeCompare(right.title);
    },
    sortOptions: [
      { value: "status", label: "Status priority" },
      { value: "evidence-desc", label: "Most evidence" },
      { value: "title-asc", label: "Title · A to Z" },
      { value: "title-desc", label: "Title · Z to A" },
    ],
  },
  metrics: (result, overview, health) => {
    const metrics = result?.metrics || {};
    const counts = overview?.counts || {};
    return result ? [
      { label: "Passed", value: metrics.passed || 0, footerLeft: "checks", footerRight: `${metrics.checks || 0} total`, tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Warnings", value: metrics.warned || 0, footerLeft: "watch", footerRight: `${metrics.workflow_pending || 0} CI pending`, tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Blocked", value: metrics.blocked || 0, footerLeft: "gate", footerRight: `${metrics.release_blockers || 0} issue blockers`, tone: "from-orange-700/70 to-red-900/60" },
      { label: "CI failures", value: metrics.workflow_failures || 0, footerLeft: `${metrics.workflow_successes || 0} passing`, footerRight: `${metrics.workflow_runs || 0} runs`, tone: "from-slate-500/70 to-slate-800/60" },
    ] : [
      { label: "Runs", value: counts.runs || health.run_count || 0, footerLeft: "saved", footerRight: `${counts.repos || health.repo_count || 0} repos`, tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Ready", value: counts.ready || health.ready_count || 0, footerLeft: "release", footerRight: "ship", tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Watch", value: counts.watch || health.watch_count || 0, footerLeft: "release", footerRight: "review", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Hold", value: counts.hold || health.hold_count || 0, footerLeft: "release", footerRight: "stop", tone: "from-orange-700/70 to-red-900/60" },
    ];
  },
  hero: (result) => result
    ? { lead: result.target_tag || result.target_version || "Release", middle: "is", highlight: `${result.decision || "pending"}.` }
    : { lead: "Release candidates", middle: "need a", highlight: "ship call." },
  status: (result, overview) => ({
    label: result?.decision || "—",
    detail: result?.summary || "Check a release candidate to begin",
    progress: result ? `${result.score || 0}%` : "8%",
    stats: [["Score", result?.score || 0], ["Checks", result?.metrics?.checks || 0], ["Runs", overview?.counts?.runs || 0]],
  }),
  chips: (result, health) => [
    result?.repo || "No repository selected",
    result?.branch || "Default branch",
    result?.target_tag || result?.target_version || "Current posture",
    health.github_ready ? "GitHub verified" : health.github?.token_configured ? "GitHub unverified" : "Token missing",
  ],
  targetSubtitle: (result) => result?.target_tag || result?.target_version || result?.branch || "Release readiness",
  historyTitle: (entry) => `${entry.repo} · ${entry.target_tag || entry.target_version || entry.branch || "release"}`,
  historySummary: (entry) => entry.summary,
  historyMeta: (entry) => `${entry.branch || "default branch"} · ${entry.score || 0}/100 · ${entry.metrics?.blocked || 0} blocked · ${entry.metrics?.warned || 0} warned`,
  historyIdentity: (entry) => `run ${String(entry.id || "unknown").slice(0, 8)}`,
  historyBadges: (entry) => [
    { label: entry.decision || "saved", tone: decisionTone(entry.decision) },
    { label: `${entry.metrics?.blocked || 0} block`, tone: entry.metrics?.blocked ? "hot" : "neutral" },
    { label: `${entry.metrics?.warned || 0} warn`, tone: entry.metrics?.warned ? "warn" : "neutral" },
  ],
  historySearchText: (entry) => `${entry.metrics?.workflow_failures || 0} failures ${entry.metrics?.release_blockers || 0} blockers`,
  historyDashboard: {
    defaultView: { decision: "all", repo: "all", sort: "newest" },
    initialCount: 6,
    searchPlaceholder: "Search repository, target, branch, decision…",
    filters: (entries, view) => [
      { key: "decision", label: "Decision", value: view.decision, options: [{ value: "all", label: "All" }, ...[...new Set(entries.map((entry) => entry.decision).filter(Boolean))].sort().map((decision) => ({ value: decision, label: decision }))] },
      { key: "repo", label: "Repository", value: view.repo, options: [{ value: "all", label: "All" }, ...[...new Set(entries.map((entry) => entry.repo).filter(Boolean))].sort().map((repo) => ({ value: repo, label: repo }))] },
    ],
    filterEntry: (entry, view) => (view.decision === "all" || entry.decision === view.decision) && (view.repo === "all" || entry.repo === view.repo),
    sortEntries: (left, right, sort) => {
      if (sort === "oldest") return new Date(left.created_at) - new Date(right.created_at);
      if (sort === "repo") return left.repo.localeCompare(right.repo) || String(left.target_tag || left.target_version || left.branch).localeCompare(String(right.target_tag || right.target_version || right.branch));
      if (sort === "score") return (right.score || 0) - (left.score || 0) || new Date(right.created_at) - new Date(left.created_at);
      if (sort === "decision") {
        const priority = { hold: 0, watch: 1, ready: 2 };
        return (priority[left.decision] ?? 3) - (priority[right.decision] ?? 3) || new Date(right.created_at) - new Date(left.created_at);
      }
      return new Date(right.created_at) - new Date(left.created_at);
    },
    sortOptions: [
      { value: "newest", label: "Newest first" },
      { value: "oldest", label: "Oldest first" },
      { value: "decision", label: "Decision priority" },
      { value: "score", label: "Highest score" },
      { value: "repo", label: "Repository" },
    ],
  },
  WorkspaceDetails,
  ChecksDetails,
  SourcesDetails,
};

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "release-sentry_api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />;
  return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />;
}

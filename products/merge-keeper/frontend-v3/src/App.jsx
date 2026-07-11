import { useMemo } from "react";
import {
  CircleCheck,
  Database,
  ExternalLink,
  GitBranch,
  GitPullRequest,
  Link2,
  ShieldCheck,
  Users,
  Workflow,
} from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  CopyMarkdownButton,
  GitHubPermissionGuidance,
  IntegratedProductApp,
  ProductLoginScreen,
  ProductShell,
  V3_TEXT,
} from "@patchhivehq/ui-v3";
import { API } from "./config.js";

const CHIP_TONES = {
  blocked: "border-red-900/30 bg-red-900/10 text-red-800 dark:border-red-400/25 dark:bg-red-500/10 dark:text-red-300",
  hold: "border-amber-900/30 bg-amber-900/10 text-amber-800 dark:border-amber-400/25 dark:bg-amber-500/10 dark:text-amber-300",
  ready: "border-emerald-900/30 bg-emerald-900/10 text-emerald-800 dark:border-emerald-400/25 dark:bg-emerald-500/10 dark:text-emerald-300",
  neutral: "border-stone-800/20 bg-stone-800/5 text-stone-700 dark:border-stone-400/20 dark:bg-stone-400/5 dark:text-stone-300",
};

function value(value, fallback = "—") {
  return value === null || value === undefined || value === "" ? fallback : String(value);
}

function stateTone(state) {
  const normalized = String(state || "").toLowerCase();
  if (["ready", "safe", "clear", "approved", "delivered", "clean", "ok"].includes(normalized)) return "ready";
  if (["blocked", "block", "error", "failed", "report_failed", "missing_token", "changes_requested", "dirty"].includes(normalized)) return "blocked";
  if (["hold", "warn", "warning", "pending", "attention", "mixed", "unstable", "behind"].includes(normalized)) return "hold";
  return "neutral";
}

function mergePolicyTone(result) {
  const state = String(result?.mergeable_state || "").toLowerCase();
  if (result?.mergeable === "no" || state === "dirty") return "blocked";
  if (["blocked", "unknown", "unstable", "behind", "has_hooks"].includes(state)) return "hold";
  return "ready";
}

function mergePostureLabel(result) {
  const state = String(result?.mergeable_state || "").toLowerCase();
  if (result?.mergeable === "yes" && state === "blocked") return "Merge policy hold";
  if (state === "dirty" || result?.mergeable === "no") return "Merge conflict";
  return state ? `Merge ${state}` : "Merge state unknown";
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

function ExternalAction({ href, children }) {
  if (!href) return null;
  return <a className={`surface-inset inline-flex h-9 items-center gap-2 rounded-full px-3 text-[11px] ${V3_TEXT.body}`} href={href} rel="noreferrer" target="_blank">{children}<ExternalLink size={12} /></a>;
}

function ContextCard({ badge, children, icon: Icon, summary, title }) {
  return (
    <article className="surface p-5">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2">
          <Icon size={14} style={{ color: "var(--accent-2)" }} />
          <div className={`font-display text-[17px] font-semibold ${V3_TEXT.strong}`}>{title}</div>
        </div>
        <Chip tone={stateTone(badge)}>{badge || "context"}</Chip>
      </div>
      <p className={`mt-3 text-[12px] leading-relaxed ${V3_TEXT.body}`}>{summary || "No summary returned."}</p>
      {children}
    </article>
  );
}

function ItemLines({ items }) {
  if (!items?.length) return null;
  return <div className="mt-4 space-y-2">{items.slice(0, 6).map((item, index) => <div className={`surface-inset rounded-xl p-3 text-[11px] leading-relaxed ${V3_TEXT.body}`} key={`${item}-${index}`}>{item}</div>)}</div>;
}

function WorkspaceDetails({ health, onError, result }) {
  if (!result) return null;
  const metrics = result.metrics || {};
  const report = result.github_report;
  const trigger = result.github || {};
  const contexts = [result.review_bee, result.trust_gate, result.repo_memory].filter(Boolean).length;

  return (
    <div className="mt-8 space-y-6">
      <section className="surface p-5 sm:p-6">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitPullRequest size={12} /> Pull request evidence</div>
            <h2 className={`mt-2 font-display text-[27px] font-semibold ${V3_TEXT.strong}`}>{result.pr_title || `${result.repo} PR #${result.pr_number}`}</h2>
            <p className={`mt-2 max-w-4xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>{result.summary}</p>
          </div>
          <div className="flex shrink-0 flex-wrap gap-2">
            <Chip tone={stateTone(result.readiness)}>{result.readiness}</Chip>
            <ExternalAction href={result.pr_url}>Open PR</ExternalAction>
          </div>
        </div>
        <div className="mt-5 flex flex-wrap gap-2">
          <Chip tone={mergePolicyTone(result)}>mergeable: {value(result.mergeable)}</Chip>
          <Chip tone={mergePolicyTone(result)}>{result.mergeable === "yes" && result.mergeable_state === "blocked" ? "policy" : "state"}: {value(result.mergeable_state)}</Chip>
          <Chip>base: {value(result.base_ref)}</Chip>
          <Chip>head: {value(result.head_ref)}</Chip>
          <Chip tone={result.approval_required === false ? "neutral" : "hold"}>{result.approval_required === false ? "approval optional" : "approval required"}</Chip>
          {trigger.trigger ? <Chip>trigger: {trigger.trigger}</Chip> : null}
          {trigger.event ? <Chip>{trigger.event}{trigger.action ? ` · ${trigger.action}` : ""}</Chip> : null}
        </div>
      </section>

      <section className="surface p-5 sm:p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Workflow size={12} /> Complete merge metrics</div>
        <div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
          <Fact label="Approvals" value={metrics.approvals} />
          <Fact label="Requested changes" value={metrics.changes_requested} />
          <Fact label="Reviewers" value={metrics.reviewer_count} />
          <Fact label="Open threads" value={metrics.open_review_threads} />
          <Fact label="Actionable threads" value={metrics.actionable_open_threads} />
          <Fact label="Successful checks" value={metrics.successful_checks} />
          <Fact label="Pending checks" value={metrics.pending_checks} />
          <Fact label="Failing checks" value={metrics.failing_checks} />
          <Fact label="Changed files" value={metrics.changed_files} />
          <Fact label="Diff" value={`+${value(metrics.additions, "0")} / -${value(metrics.deletions, "0")}`} />
        </div>
      </section>

      {result.reviewer_states?.length ? (
        <section className="surface p-5 sm:p-6">
          <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Users size={12} /> Latest reviewer states</div>
          <div className="mt-4 flex flex-wrap gap-2">{result.reviewer_states.map((reviewer) => <Chip key={`${reviewer.login}-${reviewer.state}`} tone={stateTone(reviewer.state)}>@{reviewer.login} · {String(reviewer.state || "reviewed").toLowerCase()}</Chip>)}</div>
        </section>
      ) : null}

      <section className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        {result.review_bee ? <ContextCard badge={result.review_bee.status} icon={Users} summary={result.review_bee.summary} title="ReviewBee"><div className="mt-4 flex flex-wrap gap-2"><Chip>{value(result.review_bee.open_items, "0")} open</Chip><Chip>{value(result.review_bee.actionable_threads, "0")} actionable</Chip></div><ItemLines items={result.review_bee.top_items} /></ContextCard> : null}
        {result.trust_gate ? <ContextCard badge={result.trust_gate.recommendation} icon={ShieldCheck} summary={result.trust_gate.summary} title="TrustGate"><div className="mt-4 flex flex-wrap gap-2"><Chip>{value(result.trust_gate.risk_score, "0")} risk</Chip><Chip tone={result.trust_gate.blocked_findings ? "blocked" : "neutral"}>{value(result.trust_gate.blocked_findings, "0")} blocked</Chip><Chip tone={result.trust_gate.warning_findings ? "hold" : "neutral"}>{value(result.trust_gate.warning_findings, "0")} warnings</Chip></div><ItemLines items={result.trust_gate.top_findings} /></ContextCard> : null}
        {result.repo_memory ? <ContextCard badge="context" icon={Database} summary={result.repo_memory.summary} title="RepoMemory"><div className="mt-4 flex flex-wrap gap-2"><Chip>{value(result.repo_memory.policy_entries, "0")} policy</Chip><Chip>{value(result.repo_memory.pinned_entries, "0")} pinned</Chip></div><ItemLines items={result.repo_memory.top_entries?.length ? result.repo_memory.top_entries : result.repo_memory.prompt_lines} /></ContextCard> : null}
      </section>

      {!contexts ? <section className="surface p-5"><div className={`text-[12px] ${V3_TEXT.mute}`}>This assessment used GitHub state only. ReviewBee, TrustGate, and RepoMemory remain optional suite inputs.</div></section> : null}

      {report ? (
        <section className="surface p-5 sm:p-6">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Link2 size={12} /> GitHub artifact</div>
              <h2 className={`mt-2 font-display text-[24px] font-semibold ${V3_TEXT.strong}`}>{report.delivered ? "Report delivered" : report.attempted ? "Report attempted" : "Local-only assessment"}</h2>
              <p className={`mt-2 max-w-3xl text-[12px] leading-relaxed ${V3_TEXT.body}`}>{report.message}</p>
            </div>
            <div className="flex flex-wrap gap-2"><Chip tone={stateTone(report.state)}>{report.state || "local"}</Chip>{report.comment_mode ? <Chip>{report.comment_mode}</Chip> : null}</div>
          </div>
          {report.details?.length ? <ItemLines items={report.details} /> : null}
          <div className="mt-5 flex flex-wrap gap-2">
            <ExternalAction href={report.comment_url}>Open comment</ExternalAction>
            <ExternalAction href={report.check_url || report.status_url}>Open check</ExternalAction>
            <CopyMarkdownButton content={report.report_markdown} label="Copy report Markdown" onError={() => onError("Could not copy the MergeKeeper report to the clipboard.")} />
          </div>
          {report.report_markdown ? <details className="surface-inset mt-5 rounded-xl p-4"><summary className={`cursor-pointer text-[12px] font-semibold ${V3_TEXT.strong}`}>Preview report Markdown</summary><pre className={`mt-4 overflow-x-auto whitespace-pre-wrap text-[11px] leading-relaxed ${V3_TEXT.body}`}>{report.report_markdown}</pre></details> : null}
        </section>
      ) : null}

      {!health.github_ready && !health.github?.token_configured ? <GitHubPermissionGuidance>Configure pull-request and Actions read access before live assessment. Publishing additionally needs permission to maintain the PR comment and check or status artifact.</GitHubPermissionGuidance> : null}
    </div>
  );
}

function ChecksDetails({ health }) {
  const integrations = health.integrations || {};
  const github = health.github || {};
  return (
    <section className="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-2">
      <article className="surface p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitBranch size={12} /> GitHub publish path</div>
        <div className="mt-5 grid grid-cols-2 gap-3">
          <Fact label="Token" value={github.token_configured ? "ready" : "missing"} />
          <Fact label="Webhook" value={github.webhook_secret_configured ? "ready" : "optional"} />
          <Fact label="Public URL" value={github.public_url_configured ? "ready" : "local only"} />
          <Fact label="Report publish" value={github.report_publish_ready ? "ready" : "limited"} />
        </div>
        <GitHubPermissionGuidance>Pull-request and Actions read access powers local decisions. Maintained comments and check/status output require the matching write scopes.</GitHubPermissionGuidance>
      </article>
      <article className="surface p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><CircleCheck size={12} /> Product and integration state</div>
        <dl className="mt-5 space-y-3">
          <div className="surface-inset rounded-xl p-3"><dt className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Database path</dt><dd className={`mt-1 break-all text-[12px] ${V3_TEXT.strong}`}>{health.db_path || "unknown"}</dd></div>
          <div className="flex flex-wrap gap-2"><Chip tone={health.auth_enabled ? "ready" : "hold"}>auth {health.auth_enabled ? "enabled" : "disabled"}</Chip><Chip tone="ready">{value(health.assessment_count, "0")} runs</Chip><Chip>{value(health.repo_count, "0")} repos</Chip></div>
          <div className="flex flex-wrap gap-2"><Chip tone={integrations.review_bee_configured ? "ready" : "neutral"}>ReviewBee {integrations.review_bee_configured ? "linked" : "off"}</Chip><Chip tone={integrations.trust_gate_configured ? "ready" : "neutral"}>TrustGate {integrations.trust_gate_configured ? "linked" : "off"}</Chip><Chip tone={integrations.repo_memory_configured ? "ready" : "neutral"}>RepoMemory {integrations.repo_memory_configured ? "linked" : "off"}</Chip></div>
        </dl>
      </article>
    </section>
  );
}

function SourcesDetails({ health }) {
  return (
    <section className="surface mt-6 p-5 sm:p-6">
      <div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>Assessment safety</div>
      <div className="mt-4 grid gap-3 sm:grid-cols-3">
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Local by default</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Publishing starts off. A normal assessment reads GitHub and saves its decision locally.</p></div>
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Approval policy</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Require an active approval unless the repository intentionally treats clean checks and mergeability as sufficient.</p></div>
        <div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Explicit write-back</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Publishing maintains a PR comment and check/status artifact only when enabled for that run.</p></div>
      </div>
      {!health.github_ready && !health.github?.token_configured ? <GitHubPermissionGuidance>The unified backend is missing a GitHub token. Add the product token before assessing a live pull request.</GitHubPermissionGuidance> : null}
    </section>
  );
}

const config = {
  productKey: "merge-keeper",
  name: "MergeKeeper",
  subtitle: "merge readiness",
  icon: GitPullRequest,
  workspaceLabel: "Readiness",
  eyebrow: "Pull request decision",
  queueLabel: "Readiness evidence",
  description: "Reads pull request state, reviews, threads, checks, and suite context, then returns one clear merge decision.",
  runLabel: "Assess PR",
  runningLabel: "Assessing…",
  actionPath: "/assess/github/pr",
  formTitle: "Choose a pull request.",
  sourceHelp: "The unified backend needs GitHub pull-request and Actions read access. Publishing remains an explicit per-run choice.",
  searchPlaceholder: "Search blocker, warning, evidence…",
  emptyLabel: "No blockers or warnings in this decision.",
  defaultForm: { repo: "", pr_number: "", publish_report: false, require_approval: true },
  fields: [
    { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
    { key: "pr_number", label: "Pull request number", placeholder: "123", type: "number", min: 1 },
    { key: "require_approval", label: "Require active approval", type: "checkbox" },
    { key: "publish_report", label: "Publish readiness report", type: "checkbox" },
  ],
  validate: (form) => {
    if (!/^[^/\s]+\/[^/\s]+$/.test(form.repo?.trim() || "")) return "Enter a repository in owner/name format before assessing.";
    if (!Number.isInteger(Number(form.pr_number)) || Number(form.pr_number) <= 0) return "Enter a pull request number greater than zero.";
    return "";
  },
  serialize: (form) => ({ repo: form.repo.trim(), pr_number: Number(form.pr_number), publish_report: Boolean(form.publish_report), require_approval: form.require_approval !== false }),
  formFromResult: (result) => ({ repo: result.repo, pr_number: result.pr_number ? String(result.pr_number) : "", require_approval: result.approval_required !== false }),
  items: (result) => [...(result?.blockers || []).map((item) => ({ ...item, _tone: "hot" })), ...(result?.warnings || []).map((item) => ({ ...item, _tone: "warn" }))],
  mapItem: (item) => ({ id: item.key || item.label, title: item.label || "Readiness signal", meta: item.severity || "merge evidence", summary: item.detail, evidence: item.evidence || [], score: item._tone === "hot" ? "!" : "?", severity: item.severity || "signal", status: item._tone === "hot" ? "blocker" : "warning", tone: item._tone }),
  dashboard: {
    defaultView: { decision: "all", evidence: "all", sort: "priority" },
    initialCount: 6,
    itemLabel: "signals",
    filters: (_items, view) => [
      { key: "decision", label: "Decision", value: view.decision, options: [{ value: "all", label: "All" }, { value: "blocker", label: "Blockers" }, { value: "warning", label: "Warnings" }] },
      { key: "evidence", label: "Evidence", value: view.evidence, options: [{ value: "all", label: "All" }, { value: "with", label: "With evidence" }, { value: "without", label: "Without evidence" }] },
    ],
    filterItem: (item, view) => {
      if (view.decision !== "all" && item.status !== view.decision) return false;
      if (view.evidence === "with" && !item.evidence?.length) return false;
      if (view.evidence === "without" && item.evidence?.length) return false;
      return true;
    },
    sortItems: (left, right, sort) => {
      if (sort === "title-asc") return left.title.localeCompare(right.title);
      if (sort === "title-desc") return right.title.localeCompare(left.title);
      if (sort === "evidence-desc") return (right.evidence?.length || 0) - (left.evidence?.length || 0) || left.title.localeCompare(right.title);
      const priority = { blocker: 0, warning: 1 };
      return (priority[left.status] ?? 2) - (priority[right.status] ?? 2) || left.title.localeCompare(right.title);
    },
    sortOptions: [
      { value: "priority", label: "Decision priority" },
      { value: "evidence-desc", label: "Most evidence" },
      { value: "title-asc", label: "Title · A to Z" },
      { value: "title-desc", label: "Title · Z to A" },
    ],
  },
  metrics: (result, overview, health) => {
    const metrics = result?.metrics || {};
    const counts = overview?.counts || {};
    return result ? [
      { label: "Approvals", value: metrics.approvals || 0, footerLeft: "review state", footerRight: `${metrics.reviewer_count || 0} reviewers`, tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Open threads", value: metrics.actionable_open_threads || metrics.open_review_threads || 0, footerLeft: "actionable", footerRight: `${metrics.changes_requested || 0} changes requested`, tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Failing checks", value: metrics.failing_checks || 0, footerLeft: `${metrics.successful_checks || 0} passing`, footerRight: `${metrics.pending_checks || 0} pending`, tone: "from-orange-700/70 to-red-900/60" },
      { label: "Changed files", value: metrics.changed_files || 0, footerLeft: `+${metrics.additions || 0}`, footerRight: `-${metrics.deletions || 0}`, tone: "from-slate-500/70 to-slate-800/60" },
    ] : [
      { label: "Runs", value: counts.runs || health.assessment_count || 0, footerLeft: "saved", footerRight: `${counts.repos || 0} repos`, tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Ready", value: counts.ready_runs || 0, footerLeft: "decisions", footerRight: "mergeable", tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Hold", value: counts.hold_runs || 0, footerLeft: "decisions", footerRight: "attention", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Blocked", value: counts.blocked_runs || 0, footerLeft: "decisions", footerRight: "stop", tone: "from-orange-700/70 to-red-900/60" },
    ];
  },
  hero: (result) => result ? { lead: `PR #${result.pr_number}`, middle: "is", highlight: `${result.readiness || "pending"}.` } : { lead: "Merge decisions", middle: "need clear", highlight: "evidence." },
  status: (result, overview) => ({ label: result?.readiness || "—", detail: result?.summary || "Assess a pull request to begin", progress: result ? "100%" : "8%", stats: [["Blocks", result?.blockers?.length || 0], ["Warnings", result?.warnings?.length || 0], ["Runs", overview?.counts?.runs || 0]] }),
  chips: (result, health) => [result?.repo || "No repository selected", result?.pr_number ? `PR #${result.pr_number}` : "No PR selected", result ? mergePostureLabel(result) : health.github_ready ? "GitHub ready" : "Token missing", result ? result.approval_required === false ? "Approval optional" : "Approval required" : "Local by default"],
  targetSubtitle: (result) => result?.pr_number ? `Pull request #${result.pr_number} · ${result.readiness || "pending"}` : "Pull request readiness",
  historyTitle: (entry) => `${entry.repo} · PR #${entry.pr_number}`,
  historySummary: (entry) => entry.summary || entry.pr_title,
  historyMeta: (entry) => `${entry.blockers_count || 0} blockers · ${entry.warnings_count || 0} holds · ${entry.approvals_count || 0} approvals · ${entry.failing_checks_count || 0} failing`,
  historyIdentity: (entry) => `run ${String(entry.id || "unknown").slice(0, 8)}`,
  historyBadges: (entry) => [
    { label: entry.readiness || "saved", tone: entry.readiness === "blocked" ? "hot" : entry.readiness === "hold" ? "warn" : entry.readiness === "ready" ? "ok" : "neutral" },
    { label: `${entry.blockers_count || 0} block`, tone: entry.blockers_count ? "hot" : "neutral" },
    { label: `${entry.warnings_count || 0} hold`, tone: entry.warnings_count ? "warn" : "neutral" },
  ],
  historyDashboard: {
    defaultView: { decision: "all", repo: "all", sort: "newest" },
    initialCount: 6,
    searchPlaceholder: "Search repository, PR, decision…",
    filters: (entries, view) => [
      { key: "decision", label: "Decision", value: view.decision, options: [{ value: "all", label: "All" }, ...[...new Set(entries.map((entry) => entry.readiness).filter(Boolean))].sort().map((decision) => ({ value: decision, label: decision }))] },
      { key: "repo", label: "Repository", value: view.repo, options: [{ value: "all", label: "All" }, ...[...new Set(entries.map((entry) => entry.repo).filter(Boolean))].sort().map((repo) => ({ value: repo, label: repo }))] },
    ],
    filterEntry: (entry, view) => (view.decision === "all" || entry.readiness === view.decision) && (view.repo === "all" || entry.repo === view.repo),
    sortEntries: (left, right, sort) => {
      if (sort === "oldest") return new Date(left.created_at) - new Date(right.created_at);
      if (sort === "repo") return left.repo.localeCompare(right.repo) || Number(left.pr_number) - Number(right.pr_number);
      if (sort === "decision") {
        const priority = { blocked: 0, hold: 1, ready: 2 };
        return (priority[left.readiness] ?? 3) - (priority[right.readiness] ?? 3) || new Date(right.created_at) - new Date(left.created_at);
      }
      return new Date(right.created_at) - new Date(left.created_at);
    },
    sortOptions: [
      { value: "newest", label: "Newest first" },
      { value: "oldest", label: "Oldest first" },
      { value: "decision", label: "Decision priority" },
      { value: "repo", label: "Repository" },
    ],
  },
  WorkspaceDetails,
  ChecksDetails,
  SourcesDetails,
};

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "merge-keeper_api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />;
  return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />;
}

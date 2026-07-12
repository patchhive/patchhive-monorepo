import { useMemo } from "react";
import { ClipboardCheck, Database, ExternalLink, GitBranch, GitPullRequest, Link2, MessageSquareText, ShieldCheck, Users } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  countLabel,
  CopyMarkdownButton,
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

function statusTone(status) {
  const normalized = String(status || "").toLowerCase();
  if (["attention", "open", "failed", "report_failed"].includes(normalized)) return "hot";
  if (["follow-up", "follow_up", "mixed", "quiet", "pending", "warning"].includes(normalized)) return "warn";
  if (["clear", "resolved", "delivered", "created", "updated", "ok"].includes(normalized)) return "ok";
  return "neutral";
}

function statusRank(status) {
  const normalized = String(status || "").toLowerCase();
  if (normalized === "open" || normalized === "attention") return 3;
  if (["mixed", "follow-up", "follow_up", "quiet"].includes(normalized)) return 2;
  return 1;
}

function Chip({ children, tone = "neutral" }) {
  return <span className={`inline-flex items-center justify-center rounded-full border px-2.5 py-1 text-[10px] leading-none uppercase tracking-wider ${CHIP_TONES[tone] || CHIP_TONES.neutral}`}>{children}</span>;
}

function Fact({ label, value: factValue }) {
  return <div className="surface-inset rounded-xl p-3"><div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</div><div className={`mt-1 font-display text-[18px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value(factValue, "0")}</div></div>;
}

function ExternalAction({ href, children }) {
  if (!href) return null;
  return <a className={`surface-inset inline-flex h-9 items-center gap-2 rounded-full px-3 text-[11px] ${V3_TEXT.body}`} href={href} rel="noreferrer" target="_blank">{children}<ExternalLink size={12} /></a>;
}

function evidenceText(entry) {
  return [entry.author_login ? `@${entry.author_login}` : "reviewer", entry.path, entry.excerpt, entry.resolved ? "resolved" : "open", entry.outdated ? "outdated" : ""].filter(Boolean).join(" · ");
}

function buildReviewMarkdown(result) {
  if (result?.github_report?.report_markdown) return result.github_report.report_markdown;
  const metrics = result?.metrics || {};
  const lines = [
    `# ReviewBee checklist for ${result?.repo || "repository"} PR #${result?.pr_number || "?"}`,
    "",
    result?.summary || "ReviewBee pull request review summary.",
    "",
    `- Status: ${result?.status || "unknown"}`,
    `- Reviewers: ${metrics.reviewer_count || 0}`,
    `- Actionable threads: ${metrics.actionable_threads || 0}`,
    `- Open items: ${metrics.open_items || 0}`,
    `- Resolved items: ${metrics.resolved_items || 0}`,
  ];
  if (result?.checklist?.length) {
    lines.push("", "## Checklist", "");
    result.checklist.forEach((item) => lines.push(`- [${item.status === "resolved" ? "x" : " "}] **${item.title}** — ${item.summary || item.prompt_hint || "Review follow-up"}`));
  }
  if (result?.prompt_suggestions?.length) {
    lines.push("", "## Suggested prompts", "", ...result.prompt_suggestions.map((prompt) => `- ${prompt}`));
  }
  lines.push("", "*ReviewBee by [PatchHive](https://github.com/patchhive)*");
  return lines.join("\n");
}

function WorkspaceDetails({ health, onError, result }) {
  if (!result) return null;
  const metrics = result.metrics || {};
  const report = result.github_report;
  const github = result.github || {};
  return (
    <div className="mt-8 space-y-6">
      <section className="surface p-5 sm:p-6">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitPullRequest size={12} /> Pull request review evidence</div><h2 className={`mt-2 font-display text-[27px] font-semibold ${V3_TEXT.strong}`}>{result.pr_title || `${result.repo} PR #${result.pr_number}`}</h2><p className={`mt-2 max-w-4xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>{result.summary}</p></div>
          <div className="flex shrink-0 flex-wrap gap-2"><Chip tone={statusTone(result.status)}>{result.status || "saved"}</Chip><ExternalAction href={result.pr_url}>Open PR</ExternalAction><CopyMarkdownButton content={buildReviewMarkdown(result)} label="Copy review Markdown" onError={() => onError("Could not copy the ReviewBee summary.")} /></div>
        </div>
        <div className="mt-5 flex flex-wrap gap-2"><Chip>base: {value(github.base_ref)}</Chip><Chip>head: {value(github.head_ref)}</Chip>{github.trigger ? <Chip>trigger: {github.trigger}</Chip> : null}{github.event ? <Chip>{github.event}{github.action ? ` · ${github.action}` : ""}</Chip> : null}</div>
      </section>

      <section className="surface p-5 sm:p-6">
        <div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ClipboardCheck size={12} /> Complete review metrics</div>
        <div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5"><Fact label="Reviews" value={metrics.review_count} /><Fact label="Changes requested" value={metrics.requested_changes_reviews} /><Fact label="Approvals" value={metrics.approval_reviews} /><Fact label="Comments" value={metrics.comment_reviews} /><Fact label="Threads" value={metrics.thread_count} /><Fact label="Actionable" value={metrics.actionable_threads} /><Fact label="Open items" value={metrics.open_items} /><Fact label="Resolved items" value={metrics.resolved_items} /><Fact label="Reviewers" value={metrics.reviewer_count} /><Fact label="Prompts" value={result.prompt_suggestions?.length || 0} /></div>
      </section>

      {result.reviewers?.length ? <section className="surface p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Users size={12} /> Review participants</div><div className="mt-4 flex flex-wrap gap-2">{result.reviewers.map((reviewer) => <Chip key={reviewer}>@{reviewer}</Chip>)}</div></section> : null}

      {result.prompt_suggestions?.length ? <section className="surface p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><MessageSquareText size={12} /> Suggested follow-up prompts</div><div className="mt-4 space-y-2">{result.prompt_suggestions.map((prompt, index) => <div className={`surface-inset rounded-xl p-3 text-[11px] leading-relaxed ${V3_TEXT.body}`} key={`${prompt}-${index}`}>{prompt}</div>)}</div></section> : null}

      {report ? <section className="surface p-5 sm:p-6"><div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Link2 size={12} /> GitHub artifact</div><h2 className={`mt-2 font-display text-[24px] font-semibold ${V3_TEXT.strong}`}>{report.delivered ? "Checklist delivered" : report.attempted ? "Publish attempted" : result.status === "quiet" ? "Local-only review" : "Local-only checklist"}</h2><p className={`mt-2 max-w-3xl text-[12px] leading-relaxed ${V3_TEXT.body}`}>{report.message}</p></div><div className="flex flex-wrap gap-2"><Chip tone={statusTone(report.state)}>{report.state || "local"}</Chip>{report.comment_mode ? <Chip>{report.comment_mode}</Chip> : null}</div></div><div className="mt-5 flex flex-wrap gap-2"><ExternalAction href={report.comment_url}>Open comment</ExternalAction><CopyMarkdownButton content={report.report_markdown} label="Copy report Markdown" onError={() => onError("Could not copy the ReviewBee report.")} /></div>{report.report_markdown ? <details className="surface-inset mt-5 rounded-xl p-4"><summary className={`cursor-pointer text-[12px] font-semibold ${V3_TEXT.strong}`}>Preview report Markdown</summary><pre className={`mt-4 overflow-x-auto whitespace-pre-wrap text-[11px] leading-relaxed ${V3_TEXT.body}`}>{report.report_markdown}</pre></details> : null}</section> : null}

      {!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub could not verify the configured token. Check startup evidence before reviewing a live pull request." : "Configure pull-request read access before running ReviewBee."} Maintained comments additionally require Issues write access and explicit per-run opt-in.</GitHubPermissionGuidance> : null}
    </div>
  );
}

function ChecksDetails({ health }) {
  const github = health.github || {};
  return <section className="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-2"><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitBranch size={12} /> GitHub review path</div><div className="mt-5 grid grid-cols-2 gap-3"><Fact label="Token" value={github.token_verified ? "verified" : github.token_configured ? "unverified" : "missing"} /><Fact label="Pull requests" value="read" /><Fact label="Webhook" value={github.webhook_secret_configured ? "ready" : "optional"} /><Fact label="Comment write" value={github.comment_publish_scope_verified ? "verified" : github.comment_publish_configured ? "unverified" : "missing"} /></div><GitHubPermissionGuidance>Metadata and Pull requests read access power analysis. A maintained checklist comment requires Issues write access; only a successful target write proves that permission.</GitHubPermissionGuidance></article><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Database size={12} /> Product state</div><div className="surface-inset mt-5 rounded-xl p-3"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Database path</div><div className={`mt-1 break-all text-[12px] ${V3_TEXT.strong}`}>{health.db_path || "unknown"}</div></div><div className="mt-4 flex flex-wrap gap-2"><Chip tone={health.db_ok ? "ok" : "hot"}>database {health.db_ok ? "ready" : "unavailable"}</Chip><Chip tone={health.auth_enabled ? "ok" : "warn"}>auth {health.auth_enabled ? "enabled" : "disabled"}</Chip><Chip tone="ok">{countLabel(health.review_count, "run")}</Chip><Chip>{countLabel(health.repo_count, "repo")}</Chip><Chip tone={health.open_item_count ? "warn" : "neutral"}>{countLabel(health.open_item_count, "open item")}</Chip></div></article></section>;
}

function SourcesDetails({ health }) {
  return <section className="surface mt-6 p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12} /> Review safety</div><div className="mt-4 grid gap-3 sm:grid-cols-3"><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Read first</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>ReviewBee reads PR metadata, formal reviews, and review threads. It does not edit code, approve, resolve threads, or merge.</p></div><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Explicit publishing</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>A maintained PR checklist comment is attempted only when Publish checklist comment is enabled for that run.</p></div><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Narrow meaning</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Clear means no actionable reviewer feedback was found. It is not a CI, diff-risk, or merge-readiness decision.</p></div></div>{!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub token verification failed. Review startup evidence before loading a PR." : "Add a GitHub token with Metadata and Pull requests read access."}</GitHubPermissionGuidance> : null}</section>;
}

const config = {
  productKey: "review-bee",
  name: "ReviewBee",
  subtitle: "review resolution",
  icon: MessageSquareText,
  workspaceLabel: "Review queue",
  eyebrow: "Pull request feedback",
  queueLabel: "Review checklist",
  description: "Reads GitHub review threads, removes noise, and turns the remaining reviewer asks into one concrete follow-up checklist.",
  runLabel: "Review PR",
  runningLabel: "Reading reviews…",
  actionPath: "/review/github/pr",
  formTitle: "Choose a pull request.",
  sourceHelp: "The unified backend needs GitHub pull-request read access. Publishing a maintained checklist remains an explicit per-run choice.",
  searchPlaceholder: "Search checklist, path, reviewer, evidence…",
  emptyLabel: "No actionable checklist items in this review.",
  defaultForm: { repo: "", pr_number: "", publish_comment: false },
  fields: [
    { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
    { key: "pr_number", label: "Pull request number", placeholder: "123", type: "number", min: 1 },
    { key: "publish_comment", label: "Publish checklist comment", type: "checkbox" },
  ],
  validate: (form) => {
    if (!/^[^/\s]+\/[^/\s]+$/.test(form.repo?.trim() || "")) return "Enter a repository in owner/name format before reviewing.";
    if (!Number.isInteger(Number(form.pr_number)) || Number(form.pr_number) <= 0) return "Enter a pull request number greater than zero.";
    return "";
  },
  serialize: (form) => ({ repo: form.repo.trim(), pr_number: Number(form.pr_number), publish_comment: Boolean(form.publish_comment) }),
  formFromResult: (result) => ({ repo: result.repo || "", pr_number: result.pr_number ? String(result.pr_number) : "" }),
  items: (result) => result?.checklist || [],
  mapItem: (item) => ({
    id: item.key || item.title,
    title: item.title || "Review follow-up",
    meta: [item.category, item.path_hints?.join(", ")].filter(Boolean).join(" · "),
    summary: item.summary || item.prompt_hint,
    evidence: (item.evidence || []).map(evidenceText),
    links: (item.evidence || []).filter((entry) => entry.url).map((entry) => ({ label: entry.path || `@${entry.author_login || "reviewer"}`, url: entry.url })),
    tags: [...(item.path_hints || []), ...(item.commenter_logins || []).map((reviewer) => `@${reviewer}`)],
    facts: [{ label: "Category", value: value(item.category) }, { label: "Status", value: value(item.status) }, { label: "Open threads", value: item.open_threads || 0 }, { label: "Resolved threads", value: item.resolved_threads || 0 }, { label: "Outdated threads", value: item.outdated_threads || 0 }, { label: "Comments", value: item.comment_count || 0 }, { label: "Reviewers", value: item.commenter_logins?.join(", ") || "—" }],
    source: "GitHub review threads",
    score: item.status === "open" ? 100 : item.status === "mixed" ? 65 : 30,
    status: item.status || "follow-up",
    category: item.category || "general",
    reviewer: item.commenter_logins?.[0] || "unknown",
    openThreads: item.open_threads || 0,
    commentCount: item.comment_count || 0,
    tone: statusTone(item.status),
  }),
  dashboard: {
    defaultView: { status: "all", category: "all", reviewer: "all", sort: "pressure" },
    initialCount: 6,
    itemLabel: "items",
    filters: (items, view) => [
      { key: "status", label: "Status", value: view.status, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.status).filter(Boolean))].sort().map((status) => ({ value: status, label: status }))] },
      { key: "category", label: "Category", value: view.category, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.category).filter(Boolean))].sort().map((category) => ({ value: category, label: category }))] },
      { key: "reviewer", label: "Reviewer", value: view.reviewer, options: [{ value: "all", label: "All" }, ...[...new Set(items.map((item) => item.reviewer).filter((reviewer) => reviewer && reviewer !== "unknown"))].sort().map((reviewer) => ({ value: reviewer, label: reviewer }))] },
    ],
    filterItem: (item, view) => (view.status === "all" || item.status === view.status) && (view.category === "all" || item.category === view.category) && (view.reviewer === "all" || item.reviewer === view.reviewer),
    sortItems: (left, right, sort) => {
      if (sort === "threads") return right.openThreads - left.openThreads || right.commentCount - left.commentCount;
      if (sort === "comments") return right.commentCount - left.commentCount || right.openThreads - left.openThreads;
      if (sort === "title") return left.title.localeCompare(right.title);
      return statusRank(right.status) - statusRank(left.status) || right.openThreads - left.openThreads || right.commentCount - left.commentCount;
    },
    sortOptions: [{ value: "pressure", label: "Pressure first" }, { value: "threads", label: "Most open threads" }, { value: "comments", label: "Most comments" }, { value: "title", label: "Title · A to Z" }],
  },
  metrics: (result, overview, health) => {
    const metrics = result?.metrics || {};
    const counts = overview?.counts || {};
    return result ? [
      { label: "Open items", value: metrics.open_items || 0, footerLeft: result.status === "quiet" ? "no signal" : "current queue", footerRight: countLabel(metrics.actionable_threads, "actionable thread"), tone: "from-orange-700/70 to-red-900/60" },
      { label: "Resolved", value: metrics.resolved_items || 0, footerLeft: "prior asks", footerRight: result.status === "quiet" ? "not observed" : "cleared", tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Reviewers", value: metrics.reviewer_count || 0, footerLeft: countLabel(metrics.review_count, "review"), footerRight: countLabel(metrics.approval_reviews, "approval"), tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Threads", value: metrics.thread_count || 0, footerLeft: countLabel(metrics.requested_changes_reviews, "change request"), footerRight: countLabel(metrics.comment_reviews, "comment review"), tone: "from-slate-500/70 to-slate-800/60" },
    ] : [
      { label: "Runs", value: counts.reviews || health.review_count || 0, footerLeft: "saved", footerRight: countLabel(counts.repos || health.repo_count, "repo"), tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Open items", value: counts.open_items || health.open_item_count || 0, footerLeft: "review", footerRight: "pressure", tone: "from-orange-700/70 to-red-900/60" },
      { label: "GitHub", value: health.github_ready ? "on" : "off", footerLeft: "PR reviews", footerRight: "read", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Publishing", value: health.github?.comment_publish_ready ? "on" : "local", footerLeft: "explicit", footerRight: "per run", tone: "from-slate-500/70 to-slate-800/60" },
    ];
  },
  hero: (result) => {
    if (!result) return { lead: "Review threads", middle: "need a", highlight: "checklist." };
    if (result.status === "quiet") return { lead: `PR #${result.pr_number}`, middle: "has", highlight: "no review signal." };
    if (result.status === "clear" || result.status === "resolved") return { lead: `PR #${result.pr_number}`, middle: "needs", highlight: "no follow-up." };
    return { lead: `PR #${result.pr_number}`, middle: "needs", highlight: "review follow-up." };
  },
  status: (result, overview) => ({ label: result?.status || "—", detail: result?.summary || "Review a pull request to begin", progress: result ? "100%" : "8%", stats: [["Open", result?.metrics?.open_items || 0], ["Resolved", result?.metrics?.resolved_items || 0], ["Runs", overview?.counts?.reviews || 0]] }),
  chips: (result, health) => [result?.repo || "No repository selected", result?.pr_number ? `PR #${result.pr_number}` : "No PR selected", result ? countLabel(result.metrics?.reviewer_count, "reviewer") : health.github_ready ? "GitHub verified" : health.github?.token_configured ? "GitHub unverified" : "Token missing", result?.github_report?.attempted ? "Publish attempted" : "Local by default"],
  targetSubtitle: (result) => result?.pr_number ? `Pull request #${result.pr_number} · ${result.status || "saved"}` : "Review resolution",
  historyTitle: (entry) => `${entry.repo} · PR #${entry.pr_number}`,
  historySummary: (entry) => entry.summary || entry.pr_title,
  historyMeta: (entry) => `${countLabel(entry.open_items, "open item")} · ${countLabel(entry.resolved_items, "resolved item")} · ${countLabel(entry.reviewer_count, "reviewer")}`,
  historyIdentity: (entry) => `run ${String(entry.id || "unknown").slice(0, 8)}`,
  historyBadges: (entry) => [{ label: entry.status || "saved", tone: statusTone(entry.status) }, { label: countLabel(entry.open_items, "open item"), tone: entry.open_items ? "hot" : "neutral" }, { label: countLabel(entry.resolved_items, "resolved item"), tone: entry.resolved_items ? "ok" : "neutral" }],
  historyDashboard: {
    defaultView: { status: "all", repo: "all", sort: "newest" },
    initialCount: 6,
    searchPlaceholder: "Search repository, PR, status, summary…",
    filters: (entries, view) => [
      { key: "status", label: "Status", value: view.status, options: [{ value: "all", label: "All" }, ...[...new Set(entries.map((entry) => entry.status).filter(Boolean))].sort().map((status) => ({ value: status, label: status }))] },
      { key: "repo", label: "Repository", value: view.repo, options: [{ value: "all", label: "All" }, ...[...new Set(entries.map((entry) => entry.repo).filter(Boolean))].sort().map((repo) => ({ value: repo, label: repo }))] },
    ],
    filterEntry: (entry, view) => (view.status === "all" || entry.status === view.status) && (view.repo === "all" || entry.repo === view.repo),
    sortEntries: (left, right, sort) => {
      if (sort === "oldest") return new Date(left.created_at) - new Date(right.created_at);
      if (sort === "pressure") return (right.open_items || 0) - (left.open_items || 0) || new Date(right.created_at) - new Date(left.created_at);
      if (sort === "repo") return left.repo.localeCompare(right.repo) || Number(left.pr_number) - Number(right.pr_number);
      return new Date(right.created_at) - new Date(left.created_at);
    },
    sortOptions: [{ value: "newest", label: "Newest first" }, { value: "oldest", label: "Oldest first" }, { value: "pressure", label: "Highest pressure" }, { value: "repo", label: "Repository" }],
  },
  WorkspaceDetails,
  ChecksDetails,
  SourcesDetails,
};

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "review-bee_api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />;
  return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />;
}

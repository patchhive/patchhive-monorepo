import { useMemo } from "react";
import { Database, ExternalLink, FileCode2, FileWarning, GitBranch, GitPullRequest, Link2, Scale, ShieldCheck } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  countLabel,
  CopyMarkdownButton,
  GitHubPermissionGuidance,
  GuidanceNotice,
  IntegratedProductApp,
  ProductLoginScreen,
  ProductShell,
  V3_TEXT,
} from "@patchhivehq/ui-v3";
import { API } from "./config.js";
import PolicyPanel from "./PolicyPanel.jsx";

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
  if (["block", "blocked", "failed", "failure", "error"].includes(normalized)) return "hot";
  if (["warn", "warning", "pending", "action_required"].includes(normalized)) return "warn";
  if (["safe", "success", "delivered", "created", "updated", "ok", "verified"].includes(normalized)) return "ok";
  return "neutral";
}

function decisionLabel(input) {
  if (input === "safe") return "policy-safe";
  if (input === "block") return "policy-blocked";
  return input || "pending";
}

function displaySummary(input) {
  return String(input || "").replace(/\b1 files changed\b/g, "1 file changed");
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

function reportMarkdown(result) {
  if (result?.github_report?.report_markdown) return result.github_report.report_markdown;
  const metrics = result?.metrics || {};
  const lines = [
    `# TrustGate: ${(result?.recommendation || "review").toUpperCase()}`,
    "",
    displaySummary(result?.summary) || "TrustGate diff-policy review.",
    "",
    `- Repository: ${result?.repo || "unknown"}`,
    `- Risk score: ${result?.risk_score || 0}`,
    `- Files changed: ${metrics.files_changed || 0}`,
    `- Additions / deletions: +${metrics.additions || 0} / -${metrics.deletions || 0}`,
    `- Blocking findings: ${metrics.blocked_findings || 0}`,
    `- Warning findings: ${metrics.warning_findings || 0}`,
  ];
  if (result?.findings?.length) lines.push("", "## Findings", "", ...result.findings.map((finding) => `- **${finding.label}** (${finding.severity}): ${finding.detail}`));
  lines.push("", "*TrustGate by [PatchHive](https://github.com/patchhive)*");
  return lines.join("\n");
}

function exportDecision(result) {
  const escape = (input) => String(input ?? "").replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
  const findings = (result.findings || []).map((finding) => `<li><strong>${escape(finding.label)}</strong> (${escape(finding.severity)}): ${escape(finding.detail)}</li>`).join("");
  const files = (result.files || []).map((file) => `<tr><td>${escape(file.path)}</td><td>${escape(file.status)}</td><td>+${file.additions} / -${file.deletions}</td><td>${escape(file.summary)}</td></tr>`).join("");
  const html = `<!doctype html><html><head><meta charset="utf-8"><title>TrustGate ${escape(result.id)}</title><style>body{font:15px system-ui;max-width:1000px;margin:40px auto;padding:0 24px;color:#172033}h1{font-size:36px}code,td{font-size:13px}table{border-collapse:collapse;width:100%}td,th{border:1px solid #ccd3df;padding:9px;text-align:left}.status{text-transform:uppercase;font-weight:700}</style></head><body><p>PatchHive · TrustGate</p><h1>Diff policy decision</h1><p class="status">${escape(decisionLabel(result.recommendation))} · risk ${escape(result.risk_score)}/100</p><h2>${escape(result.repo)}</h2><p>${escape(displaySummary(result.summary))}</p><h2>Findings</h2><ul>${findings || "<li>No active findings.</li>"}</ul><h2>Files</h2><table><thead><tr><th>Path</th><th>Status</th><th>Diff</th><th>Summary</th></tr></thead><tbody>${files}</tbody></table><p><em>TrustGate by PatchHive</em></p></body></html>`;
  const url = URL.createObjectURL(new Blob([html], { type: "text/html" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `trustgate-${result.id}.html`;
  anchor.click();
  URL.revokeObjectURL(url);
}

function WorkspaceDetails({ health, onError, result }) {
  if (!result) return null;
  const metrics = result.metrics || {};
  const report = result.github_report;
  const github = result.github || {};
  return <div className="mt-8 space-y-6">
    <section className="surface p-5 sm:p-6"><div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Scale size={12} /> Diff policy evidence</div><h2 className={`mt-2 font-display text-[27px] font-semibold ${V3_TEXT.strong}`}>{github.pr_title || `${result.repo} safety review`}</h2><p className={`mt-2 max-w-4xl text-[13px] leading-relaxed ${V3_TEXT.body}`}>{displaySummary(result.summary)}</p></div><div className="flex shrink-0 flex-wrap gap-2"><Chip tone={decisionTone(result.recommendation)}>{decisionLabel(result.recommendation)}</Chip><ExternalAction href={github.pr_url}>Open PR</ExternalAction><CopyMarkdownButton content={reportMarkdown(result)} label="Copy decision Markdown" onError={() => onError("Could not copy the TrustGate decision.")} /><button className={`surface-inset h-9 rounded-full px-3 text-[11px] ${V3_TEXT.body}`} onClick={() => exportDecision(result)} type="button">Export decision HTML</button></div></div><div className="mt-5 flex flex-wrap gap-2"><Chip>{result.source_kind || "manual"}</Chip><Chip>AI source: {result.ai_source || "unspecified"}</Chip>{github.base_ref ? <Chip>base: {github.base_ref}</Chip> : null}{github.head_ref ? <Chip>head: {github.head_ref}</Chip> : null}{github.trigger ? <Chip>trigger: {github.trigger}</Chip> : null}</div></section>

    <GuidanceNotice label="Decision scope">A TrustGate recommendation covers diff policy only. It does not include CI results, approvals, review-thread state, mergeability, or release readiness; those remain MergeKeeper, ReviewBee, FlakeSting, and ReleaseSentry evidence.</GuidanceNotice>

    <section className="surface p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><FileCode2 size={12} /> Complete diff metrics</div><div className="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5"><Fact label="Files changed" value={metrics.files_changed} /><Fact label="Additions" value={`+${metrics.additions || 0}`} /><Fact label="Deletions" value={`-${metrics.deletions || 0}`} /><Fact label="Tests changed" value={metrics.tests_changed} /><Fact label="Risky files" value={metrics.risky_files} /><Fact label="Generated files" value={metrics.generated_files} /><Fact label="Source files" value={metrics.source_files_changed} /><Fact label="Blocking findings" value={metrics.blocked_findings} /><Fact label="Warnings" value={metrics.warning_findings} /><Fact label="Risk score" value={`${result.risk_score || 0}/100`} /></div></section>

    {result.files?.length ? <section className="surface p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><FileWarning size={12} /> File assessments</div><div className="mt-4 space-y-2">{result.files.map((file) => <div className="surface-inset rounded-xl p-4" key={file.path}><div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between"><div><div className={`font-mono text-[12px] ${V3_TEXT.strong}`}>{file.path}</div><p className={`mt-1 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>{file.summary}</p></div><div className="flex shrink-0 flex-wrap gap-2"><Chip tone={decisionTone(file.status)}>{decisionLabel(file.status)}</Chip><Chip>+{file.additions} / -{file.deletions}</Chip>{file.generated ? <Chip>generated</Chip> : null}</div></div>{file.matched_rules?.length ? <div className="mt-3 flex flex-wrap gap-2">{file.matched_rules.map((rule) => <Chip key={rule}>{rule}</Chip>)}</div> : null}{file.path_policy ? <p className={`mt-3 text-[11px] leading-relaxed ${V3_TEXT.body}`}>{file.path_policy}</p> : null}</div>)}</div></section> : null}

    {result.repo_memory_context ? <section className="surface p-5 sm:p-6"><div className={`text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}>RepoMemory context</div><pre className={`surface-inset mt-4 overflow-x-auto whitespace-pre-wrap rounded-xl p-4 text-[11px] ${V3_TEXT.body}`}>{JSON.stringify(result.repo_memory_context, null, 2)}</pre></section> : null}

    {report ? <section className="surface p-5 sm:p-6"><div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between"><div><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Link2 size={12} /> GitHub artifact</div><h2 className={`mt-2 font-display text-[24px] font-semibold ${V3_TEXT.strong}`}>{report.delivered ? "Decision delivered" : report.attempted ? "Publish attempted" : "Local-only decision"}</h2><p className={`mt-2 max-w-3xl text-[12px] leading-relaxed ${V3_TEXT.body}`}>{report.message}</p></div><div className="flex flex-wrap gap-2"><Chip tone={decisionTone(report.state)}>{report.state || "local"}</Chip>{report.method && report.method !== "none" ? <Chip>{report.method}</Chip> : null}{report.comment_mode && report.comment_mode !== "none" ? <Chip>{report.comment_mode}</Chip> : null}</div></div>{report.details?.length ? <div className="mt-4 space-y-2">{report.details.map((detail, index) => <div className={`surface-inset rounded-xl p-3 text-[11px] ${V3_TEXT.body}`} key={`${detail}-${index}`}>{detail}</div>)}</div> : null}<div className="mt-5 flex flex-wrap gap-2"><ExternalAction href={report.check_url}>Open check</ExternalAction><ExternalAction href={report.status_url}>Open status</ExternalAction><ExternalAction href={report.comment_url}>Open comment</ExternalAction><CopyMarkdownButton content={report.report_markdown} label="Copy report Markdown" onError={() => onError("Could not copy the TrustGate report.")} /></div>{report.report_markdown ? <details className="surface-inset mt-5 rounded-xl p-4"><summary className={`cursor-pointer text-[12px] font-semibold ${V3_TEXT.strong}`}>Preview report Markdown</summary><pre className={`mt-4 overflow-x-auto whitespace-pre-wrap text-[11px] leading-relaxed ${V3_TEXT.body}`}>{report.report_markdown}</pre></details> : null}</section> : null}

    {!health.github_ready && result.source_kind === "github_pr" ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub could not verify the configured token. Review startup evidence before trusting PR-backed coverage." : "Configure pull-request read access before reviewing a live PR."} Publishing also needs a classic public-repository token or target-specific write access; successful target writes are the proof.</GitHubPermissionGuidance> : null}
  </div>;
}

function ChecksDetails({ health }) {
  const github = health.github || {};
  return <section className="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-2"><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><GitBranch size={12} /> GitHub diff-review path</div><div className="mt-5 grid grid-cols-2 gap-3"><Fact label="Token" value={github.token_verified ? "verified" : github.token_configured ? "unverified" : "missing"} /><Fact label="Pull requests" value="read" /><Fact label="Webhook" value={github.webhook_secret_configured ? "ready" : "optional"} /><Fact label="Publish write" value="target verified" /></div><GitHubPermissionGuidance>Pull-request read access powers live diff review. Publishing uses a commit status for PATs or a check run for GitHub Apps, plus a maintained PR comment; only successful target writes prove those permissions.</GitHubPermissionGuidance></article><article className="surface p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><Database size={12} /> Product state</div><div className="surface-inset mt-5 rounded-xl p-3"><div className={`text-[10px] uppercase tracking-wider ${V3_TEXT.mute}`}>Database path</div><div className={`mt-1 break-all text-[12px] ${V3_TEXT.strong}`}>{health.db_path || "unknown"}</div></div><div className="mt-4 flex flex-wrap gap-2"><Chip tone={health.db_ok ? "ok" : "hot"}>database {health.db_ok ? "ready" : "unavailable"}</Chip><Chip tone={health.auth_enabled ? "ok" : "warn"}>auth {health.auth_enabled ? "enabled" : "disabled"}</Chip><Chip tone="ok">{countLabel(health.review_count, "review")}</Chip><Chip>{countLabel(health.repo_count, "repo")}</Chip><Chip>{countLabel(health.rules_count, "rule set")}</Chip><Chip>{countLabel(health.template_count, "template set")}</Chip></div></article></section>;
}

function SourcesDetails({ health }) {
  return <section className="surface mt-6 p-5 sm:p-6"><div className={`flex items-center gap-2 text-[10px] uppercase tracking-[0.22em] ${V3_TEXT.mute}`}><ShieldCheck size={12} /> Review safety</div><div className="mt-4 grid gap-3 sm:grid-cols-3"><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Policy scope only</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Safe means this diff passed the current repo rules. It is not a CI, approval, merge-readiness, or release-readiness decision.</p></div><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Explicit publishing</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>GitHub status and maintained-comment publishing occurs only for PR-backed runs with publishing enabled.</p></div><div className="surface-inset rounded-xl p-4"><div className={`font-display text-[16px] ${V3_TEXT.strong}`}>Durable policy</div><p className={`mt-2 text-[11px] leading-relaxed ${V3_TEXT.mute}`}>Saved repo rules and templates are reused automatically. Warn and block outcomes may create FailGuard candidates when RepoMemory is configured.</p></div></div>{!health.github_ready ? <GitHubPermissionGuidance>{health.github?.token_configured ? "GitHub token verification failed. Pasted-diff review remains available." : "Add a GitHub token for PR-backed diff review; pasted-diff review works without it."}</GitHubPermissionGuidance> : null}</section>;
}

function PolicyTab(props) {
  return <PolicyPanel {...props} />;
}

const config = {
  productKey: "trust-gate",
  name: "TrustGate",
  subtitle: "diff policy",
  icon: ShieldCheck,
  workspaceLabel: "Risk review",
  eyebrow: "Diff policy decision",
  queueLabel: "Policy findings",
  description: "Reviews pasted or pull-request diffs against durable repo rules, then returns one clear safe, warn, or block recommendation.",
  runLabel: "Review diff",
  runningLabel: "Reviewing…",
  actionPath: (form) => form.source_mode === "manual" ? "/review" : "/review/github/pr",
  formTitle: "Choose a diff source.",
  sourceHelp: "Pasted diffs work locally. Live PR review needs GitHub pull-request read access; publishing remains an explicit per-run choice.",
  searchPlaceholder: "Search finding, severity, rule, evidence…",
  emptyLabel: "No active policy findings in this decision.",
  defaultForm: { source_mode: "github", repo: "", pr_number: "", ai_source: "Codex", publish_status: false, diff: "" },
  fields: (_health, form) => [
    { key: "source_mode", label: "Diff source", type: "select", options: [{ value: "github", label: "GitHub pull request" }, { value: "manual", label: "Pasted unified diff" }] },
    { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
    ...(form.source_mode === "manual" ? [
      { key: "ai_source", label: "AI source", placeholder: "Codex, Copilot, human…" },
      { key: "diff", label: "Unified diff", placeholder: "diff --git a/src/file.rs b/src/file.rs…", type: "textarea", rows: 14, fullWidth: true },
    ] : [
      { key: "pr_number", label: "Pull request number", placeholder: "123", type: "number", min: 1 },
      { key: "ai_source", label: "AI source", placeholder: "Codex, Copilot, human…" },
      { key: "publish_status", label: "Publish TrustGate report", help: "Publishes a GitHub status signal and maintained PR comment for this run.", type: "checkbox" },
    ]),
  ],
  validate: (form) => {
    if (!/^[^/\s]+\/[^/\s]+$/.test(form.repo?.trim() || "")) return "Enter a repository in owner/name format before reviewing.";
    if (form.source_mode === "manual" && !form.diff?.trim()) return "Paste a unified diff before running TrustGate.";
    if (form.source_mode !== "manual" && (!Number.isInteger(Number(form.pr_number)) || Number(form.pr_number) <= 0)) return "Enter a pull request number greater than zero.";
    return "";
  },
  serialize: (form) => form.source_mode === "manual"
    ? { repo: form.repo.trim(), diff: form.diff, ai_source: form.ai_source.trim() }
    : { repo: form.repo.trim(), pr_number: Number(form.pr_number), ai_source: form.ai_source.trim(), publish_status: Boolean(form.publish_status) },
  formFromResult: (result) => ({ source_mode: result.source_kind === "github_pr" ? "github" : "manual", repo: result.repo || "", pr_number: result.github?.pr_number ? String(result.github.pr_number) : "", ai_source: result.ai_source || "", diff: result.source_kind === "manual" ? result.diff || "" : "" }),
  historyItems: (payload) => payload?.reviews || [],
  items: (result) => result?.findings || [],
  mapItem: (item) => ({ id: item.key || item.label, title: item.label || "Policy finding", meta: item.severity || "finding", summary: item.detail, evidence: item.evidence || [], facts: [{ label: "Severity", value: item.severity }, { label: "Evidence", value: item.evidence?.length || 0 }], source: "TrustGate policy", score: item.severity === "block" ? 100 : item.severity === "warn" ? 65 : 25, status: item.severity || "finding", severity: item.severity || "unknown", tone: decisionTone(item.severity) }),
  dashboard: {
    defaultView: { severity: "all", evidence: "all", sort: "severity" }, initialCount: 6, itemLabel: "findings",
    filters: (_items, view) => [{ key: "severity", label: "Severity", value: view.severity, options: [{ value: "all", label: "All" }, { value: "block", label: "Blocked" }, { value: "warn", label: "Warnings" }, { value: "info", label: "Info" }] }, { key: "evidence", label: "Evidence", value: view.evidence, options: [{ value: "all", label: "All" }, { value: "with", label: "With evidence" }, { value: "without", label: "Without evidence" }] }],
    filterItem: (item, view) => (view.severity === "all" || item.severity === view.severity) && (view.evidence === "all" || (view.evidence === "with" ? item.evidence?.length : !item.evidence?.length)),
    sortItems: (left, right, sort) => { if (sort === "title") return left.title.localeCompare(right.title); if (sort === "evidence") return (right.evidence?.length || 0) - (left.evidence?.length || 0); const rank = { block: 0, warn: 1, info: 2 }; return (rank[left.severity] ?? 3) - (rank[right.severity] ?? 3) || left.title.localeCompare(right.title); },
    sortOptions: [{ value: "severity", label: "Severity first" }, { value: "evidence", label: "Most evidence" }, { value: "title", label: "Title · A to Z" }],
  },
  metrics: (result, overview, health) => {
    const metrics = result?.metrics || {}; const counts = overview?.counts || {};
    return result ? [
      { label: "Blocked", value: metrics.blocked_findings || 0, footerLeft: "policy", footerRight: countLabel(metrics.risky_files, "risky file"), tone: "from-orange-700/70 to-red-900/60" },
      { label: "Warnings", value: metrics.warning_findings || 0, footerLeft: "review", footerRight: countLabel(result.findings?.length, "finding"), tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Scope", value: metrics.files_changed || 0, footerLeft: `+${metrics.additions || 0}`, footerRight: `-${metrics.deletions || 0}`, tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Tests", value: metrics.tests_changed || 0, footerLeft: countLabel(metrics.source_files_changed, "source file"), footerRight: countLabel(metrics.generated_files, "generated file"), tone: "from-emerald-700/70 to-teal-900/60" },
    ] : [
      { label: "Reviews", value: counts.reviews || health.review_count || 0, footerLeft: "saved", footerRight: countLabel(counts.repos || health.repo_count, "repo"), tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Safe", value: counts.safe || 0, footerLeft: "policy", footerRight: "clear", tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Warn", value: counts.warn || 0, footerLeft: "human", footerRight: "review", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Block", value: counts.block || 0, footerLeft: "policy", footerRight: "stop", tone: "from-orange-700/70 to-red-900/60" },
    ];
  },
  hero: (result) => result ? { lead: result.github?.pr_number ? `PR #${result.github.pr_number}` : "This diff", middle: "is", highlight: result.recommendation === "safe" ? "policy-safe." : result.recommendation === "block" ? "blocked by policy." : `${result.recommendation || "pending"}.` } : { lead: "Every diff", middle: "needs a", highlight: "trust call." },
  status: (result) => ({ label: result ? decisionLabel(result.recommendation) : "—", detail: displaySummary(result?.summary) || "Review a pasted diff or live pull request to begin", progress: result ? `${Math.max(4, result.risk_score || 0)}%` : "8%" }),
  chips: (result, health) => [result?.repo || "No repository selected", result?.source_kind === "github_pr" ? `PR #${result.github?.pr_number || "—"}` : result?.source_kind || "Pasted or GitHub diff", result?.ai_source || "AI source optional", health.github_ready ? "GitHub verified" : "Manual review ready"],
  targetSubtitle: (result) => result?.github?.pr_number ? `Pull request #${result.github.pr_number} · ${decisionLabel(result.recommendation)}` : result ? `${result.source_kind} · ${decisionLabel(result.recommendation)}` : "Diff policy review",
  historyTitle: (entry) => `${entry.repo}${entry.pr_number ? ` · PR #${entry.pr_number}` : " · pasted diff"}`,
  historySummary: (entry) => displaySummary(entry.summary),
  historyMeta: (entry) => `${entry.source_kind || "manual"} · ${entry.risk_score || 0}/100 · ${countLabel(entry.files_changed, "file")}`,
  historyIdentity: (entry) => `review ${String(entry.id || "unknown").slice(0, 8)}`,
  historyBadges: (entry) => [{ label: decisionLabel(entry.recommendation), tone: decisionTone(entry.recommendation) }, { label: `${entry.risk_score || 0} risk`, tone: entry.risk_score >= 70 ? "hot" : entry.risk_score >= 35 ? "warn" : "neutral" }],
  historySearchText: (entry) => `${entry.ai_source || ""} ${entry.source_kind || ""} ${entry.files_changed || 0} files`,
  historyDashboard: {
    defaultView: { recommendation: "all", source: "all", repo: "all", sort: "newest" }, initialCount: 6, searchPlaceholder: "Search repository, PR, decision, source…",
    filters: (entries, view) => [{ key: "recommendation", label: "Decision", value: view.recommendation, options: [{ value: "all", label: "All" }, ...[...new Set(entries.map((entry) => entry.recommendation).filter(Boolean))].sort().map((item) => ({ value: item, label: decisionLabel(item) }))] }, { key: "source", label: "Source", value: view.source, options: [{ value: "all", label: "All" }, { value: "manual", label: "Pasted diff" }, { value: "github_pr", label: "GitHub PR" }] }, { key: "repo", label: "Repository", value: view.repo, options: [{ value: "all", label: "All" }, ...[...new Set(entries.map((entry) => entry.repo).filter(Boolean))].sort().map((repo) => ({ value: repo, label: repo }))] }],
    filterEntry: (entry, view) => (view.recommendation === "all" || entry.recommendation === view.recommendation) && (view.source === "all" || entry.source_kind === view.source) && (view.repo === "all" || entry.repo === view.repo),
    sortEntries: (left, right, sort) => { if (sort === "oldest") return new Date(left.created_at) - new Date(right.created_at); if (sort === "risk") return (right.risk_score || 0) - (left.risk_score || 0); if (sort === "repo") return left.repo.localeCompare(right.repo); if (sort === "decision") { const rank = { block: 0, warn: 1, safe: 2 }; return (rank[left.recommendation] ?? 3) - (rank[right.recommendation] ?? 3); } return new Date(right.created_at) - new Date(left.created_at); },
    sortOptions: [{ value: "newest", label: "Newest first" }, { value: "oldest", label: "Oldest first" }, { value: "decision", label: "Decision priority" }, { value: "risk", label: "Highest risk" }, { value: "repo", label: "Repository" }],
  },
  extraTabs: [{ id: "policy", label: "Policy", render: PolicyTab }],
  WorkspaceDetails,
  ChecksDetails,
  SourcesDetails,
};

function normalizeLegacyRoute() {
  const match = window.location.pathname.match(/^\/history\/([^/]+)(?:\/print)?\/?$/);
  if (!match) return;
  const url = new URL(window.location.href);
  url.pathname = "/";
  url.searchParams.set("run", decodeURIComponent(match[1]));
  window.history.replaceState({}, "", url.toString());
}

export default function App() {
  normalizeLegacyRoute();
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "trust-gate_api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />;
  return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />;
}

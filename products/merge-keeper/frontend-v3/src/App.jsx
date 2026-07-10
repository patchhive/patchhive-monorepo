import { useMemo } from "react";
import { GitPullRequest } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import { IntegratedProductApp, ProductLoginScreen, ProductShell, V3_TEXT } from "@patchhivehq/ui-v3";
import { API } from "./config.js";

const config = {
  productKey: "merge-keeper", name: "MergeKeeper", subtitle: "merge readiness", icon: GitPullRequest,
  workspaceLabel: "Readiness", eyebrow: "Pull request decision", queueLabel: "Readiness evidence",
  description: "Reads pull request state, reviews, threads, checks, and suite context, then returns one clear merge decision.",
  runLabel: "Assess PR", runningLabel: "Assessing…", actionPath: "/assess/github/pr", formTitle: "Choose a pull request.",
  sourceHelp: "The unified backend needs GitHub pull-request and Actions read access. Publishing remains an explicit per-run choice.",
  searchPlaceholder: "Search blocker, warning, evidence…", emptyLabel: "No blockers or warnings in this decision.",
  defaultForm: { repo: "", pr_number: "", publish_report: false, require_approval: true },
  fields: [
    { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
    { key: "pr_number", label: "Pull request number", placeholder: "123", type: "number", min: 1 },
    { key: "require_approval", label: "Require approval", type: "checkbox" },
    { key: "publish_report", label: "Publish GitHub report", type: "checkbox" },
  ],
  serialize: (form) => ({ repo: form.repo, pr_number: Number(form.pr_number) || 0, publish_report: Boolean(form.publish_report), require_approval: form.require_approval !== false }),
  formFromResult: (result) => ({ repo: result.repo, pr_number: result.pr_number ? String(result.pr_number) : "", require_approval: result.approval_required !== false }),
  items: (result) => [...(result?.blockers || []).map((item) => ({ ...item, _tone: "hot" })), ...(result?.warnings || []).map((item) => ({ ...item, _tone: "warn" }))],
  mapItem: (item) => ({ id: item.key || item.label, title: item.label || "Readiness signal", meta: item.severity || "merge evidence", summary: item.detail, evidence: item.evidence || [], score: item._tone === "hot" ? "!" : "?", status: item._tone === "hot" ? "blocker" : "warning", tone: item._tone }),
  metrics: (result, overview, health) => {
    const m = result?.metrics || {};
    const c = overview?.counts || {};
    return result ? [
      { label: "Approvals", value: m.approvals || 0, footerLeft: "review state", footerRight: `${m.reviewer_count || 0} reviewers`, tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Open threads", value: m.actionable_open_threads || m.open_review_threads || 0, footerLeft: "actionable", footerRight: "review pressure", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Failing checks", value: m.failing_checks || 0, footerLeft: "CI", footerRight: `${m.pending_checks || 0} pending`, tone: "from-orange-700/70 to-red-900/60" },
      { label: "Changed files", value: m.changed_files || 0, footerLeft: "scope", footerRight: `${m.additions || 0}+`, tone: "from-slate-500/70 to-slate-800/60" },
    ] : [
      { label: "Runs", value: c.runs || health.run_count || 0, footerLeft: "saved", footerRight: `${c.repos || 0} repos`, tone: "from-slate-500/70 to-slate-800/60" },
      { label: "Ready", value: c.ready_runs || 0, footerLeft: "decisions", footerRight: "mergeable", tone: "from-emerald-700/70 to-teal-900/60" },
      { label: "Hold", value: c.hold_runs || 0, footerLeft: "decisions", footerRight: "attention", tone: "from-amber-600/70 to-yellow-800/50" },
      { label: "Blocked", value: c.blocked_runs || 0, footerLeft: "decisions", footerRight: "stop", tone: "from-orange-700/70 to-red-900/60" },
    ];
  },
  hero: (result) => result ? { lead: `PR #${result.pr_number}`, middle: "is", highlight: `${result.readiness || "pending"}.` } : { lead: "Merge decisions", middle: "need clear", highlight: "evidence." },
  status: (result, overview) => ({ label: result?.readiness || "—", detail: result?.summary || "Assess a pull request to begin", progress: result ? "100%" : "8%", stats: [["Blocks", result?.blockers?.length || 0], ["Warnings", result?.warnings?.length || 0], ["Runs", overview?.counts?.runs || 0]] }),
  chips: (result, health) => [result?.repo || "No repository selected", result?.pr_number ? `PR #${result.pr_number}` : "No PR selected", health.github_ready ? "GitHub ready" : "Token missing"],
  targetSubtitle: (result) => result?.pr_number ? `Pull request #${result.pr_number}` : "Pull request readiness",
  historyTitle: (entry) => `${entry.repo} #${entry.pr_number}`,
};

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "merge-keeper_api_key" });
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  if (!auth.checked) return <ProductShell productKey={config.productKey} footerLeft="PatchHive · MergeKeeper"><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />;
  return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />;
}

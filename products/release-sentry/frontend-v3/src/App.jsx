import { useMemo } from "react";
import { Rocket } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import { IntegratedProductApp, ProductLoginScreen, ProductShell, V3_TEXT } from "@patchhivehq/ui-v3";
import { API } from "./config.js";

const decisionTone = (value) => value === "hold" || value === "block" ? "hot" : value === "watch" || value === "warn" ? "warn" : value === "ready" || value === "pass" ? "ok" : "neutral";
const config = {
  productKey: "release-sentry", name: "ReleaseSentry", subtitle: "release gate", icon: Rocket,
  workspaceLabel: "Release gate", eyebrow: "Ship decision", queueLabel: "Release evidence",
  description: "Combines tags, changelog state, workflow health, blockers, and release pressure into one ship or hold decision.",
  runLabel: "Check release", runningLabel: "Checking…", actionPath: "/check/github/release", formTitle: "Choose a release candidate.",
  sourceHelp: "The unified backend needs repository contents, releases, pull-request, and Actions read access.",
  searchPlaceholder: "Search check, blocker, evidence…", emptyLabel: "No release evidence has been returned yet.",
  defaultForm: { repo: "", branch: "", target_version: "", target_tag: "", changelog_path: "CHANGELOG.md", workflow_run_limit: "20", blocker_labels: "release-blocker, blocker, critical, regression" },
  fields: [
    { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
    { key: "branch", label: "Branch", placeholder: "main" },
    { key: "target_version", label: "Target version", placeholder: "1.4.0" },
    { key: "target_tag", label: "Target tag", placeholder: "v1.4.0" },
    { key: "changelog_path", label: "Changelog path", placeholder: "CHANGELOG.md" },
    { key: "workflow_run_limit", label: "Workflow runs", type: "number", min: 1 },
  ],
  serialize: (form) => ({ ...form, workflow_run_limit: Number(form.workflow_run_limit) || 20, blocker_labels: String(form.blocker_labels || "").split(",").map((value) => value.trim()).filter(Boolean) }),
  formFromResult: (result) => ({ repo: result.repo, branch: result.branch, target_version: result.target_version, target_tag: result.target_tag }),
  items: (result) => result?.checks || [],
  mapItem: (item) => { const t = decisionTone(item.status); return { id: item.key || item.label, title: item.label || "Release check", meta: item.status || "evidence", summary: item.detail, evidence: item.evidence || [], link: item.links?.[0]?.url, score: t === "ok" ? "✓" : t === "hot" ? "!" : "?", status: item.status || "check", tone: t }; },
  metrics: (result, overview, health) => { const m = result?.metrics || {}; const c = overview?.counts || {}; return result ? [
    { label: "Passed", value: m.passed || 0, footerLeft: "checks", footerRight: `${m.checks || 0} total`, tone: "from-emerald-700/70 to-teal-900/60" },
    { label: "Warnings", value: m.warned || 0, footerLeft: "watch", footerRight: "review", tone: "from-amber-600/70 to-yellow-800/50" },
    { label: "Blocked", value: m.blocked || 0, footerLeft: "gate", footerRight: "hold", tone: "from-orange-700/70 to-red-900/60" },
    { label: "CI failures", value: m.workflow_failures || 0, footerLeft: "workflows", footerRight: `${m.workflow_runs || 0} runs`, tone: "from-slate-500/70 to-slate-800/60" },
  ] : [
    { label: "Runs", value: c.runs || health.run_count || 0, footerLeft: "saved", footerRight: `${c.repos || 0} repos`, tone: "from-slate-500/70 to-slate-800/60" },
    { label: "Ready", value: c.ready || 0, footerLeft: "release", footerRight: "ship", tone: "from-emerald-700/70 to-teal-900/60" },
    { label: "Watch", value: c.watch || 0, footerLeft: "release", footerRight: "review", tone: "from-amber-600/70 to-yellow-800/50" },
    { label: "Hold", value: c.hold || 0, footerLeft: "release", footerRight: "stop", tone: "from-orange-700/70 to-red-900/60" },
  ]; },
  hero: (result) => result ? { lead: result.target_tag || result.target_version || "Release", middle: "is", highlight: `${result.decision || "pending"}.` } : { lead: "Release candidates", middle: "need a", highlight: "ship call." },
  status: (result, overview) => ({ label: result?.decision || "—", detail: result?.summary || "Check a release candidate to begin", progress: result ? `${result.score || 0}%` : "8%", stats: [["Score", result?.score || 0], ["Checks", result?.metrics?.checks || 0], ["Runs", overview?.counts?.runs || 0]] }),
  chips: (result, health) => [result?.repo || "No repository selected", result?.branch || "Default branch", health.github_ready ? "GitHub verified" : health.github?.token_configured ? "GitHub unverified" : "Token missing"],
  targetSubtitle: (result) => result?.target_tag || result?.target_version || result?.branch || "Release readiness",
  historyTitle: (entry) => `${entry.repo} · ${entry.target_tag || entry.target_version || entry.branch || "release"}`,
};

export default function App() { const auth = useApiKeyAuth({ apiBase: API, storageKey: "release-sentry_api_key" }); const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]); if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>; if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />; return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />; }

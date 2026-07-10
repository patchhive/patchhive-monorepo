import { useMemo } from "react";
import { Bug } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import { IntegratedProductApp, ProductLoginScreen, ProductShell, V3_TEXT } from "@patchhivehq/ui-v3";
import { API } from "./config.js";

const signalTone = (item) => item.status === "quarantine" || Number(item.score) >= 80 ? "hot" : Number(item.score) >= 50 || item.status === "watch" ? "warn" : "ok";
const config = {
  productKey: "flake-sting", name: "FlakeSting", subtitle: "CI trust", icon: Bug,
  workspaceLabel: "Instability", eyebrow: "Workflow instability", queueLabel: "Flaky signal queue",
  description: "Reads GitHub Actions history and explains fail/pass swings, rerun pressure, and likely quarantine candidates.",
  runLabel: "Run scan", runningLabel: "Scanning…", actionPath: "/scan/github/actions", formTitle: "Choose a workflow scope.",
  sourceHelp: "The unified backend needs GitHub Actions read access. FlakeSting remains read-only and evidence-first.",
  searchPlaceholder: "Search workflow, job, step…", emptyLabel: "No flaky signals match this view.",
  defaultForm: { repo: "", branch: "", workflow_name: "", lookback_runs: "25" },
  fields: [
    { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
    { key: "branch", label: "Branch", placeholder: "main or blank for all" },
    { key: "workflow_name", label: "Workflow", placeholder: "CI or blank for all" },
    { key: "lookback_runs", label: "Lookback runs", type: "number", min: 5 },
  ],
  serialize: (form) => ({ repo: form.repo, branch: form.branch, workflow_name: form.workflow_name, lookback_runs: Number(form.lookback_runs) || 25 }),
  formFromResult: (result) => ({ repo: result.repo, branch: result.branch, workflow_name: result.workflow_name }),
  items: (result) => result?.signals || [],
  mapItem: (item) => ({ id: item.key || `${item.workflow_name}-${item.job_name}-${item.step_name}`, title: item.step_name || item.job_name || item.workflow_name || "Workflow signal", meta: [item.workflow_name, item.job_name, item.kind].filter(Boolean).join(" · "), summary: item.summary, evidence: item.evidence || [], link: (item.evidence || []).map((value) => String(value).match(/https?:\/\/\S+/)?.[0]).find(Boolean), score: item.score || 0, status: item.status || item.kind || "signal", tone: signalTone(item) }),
  metrics: (result, overview, health) => { const m = result?.metrics || {}; const c = overview?.counts || {}; return result ? [
    { label: "Flaky signals", value: m.flaky_signals || 0, footerLeft: "current", footerRight: result.trend?.status || "baseline", tone: "from-orange-700/70 to-red-900/60" },
    { label: "Failed runs", value: m.failed_runs || 0, footerLeft: "workflows", footerRight: `${m.workflow_runs || 0} runs`, tone: "from-amber-600/70 to-yellow-800/50" },
    { label: "Reruns", value: m.rerun_like_runs || 0, footerLeft: "retry", footerRight: "pressure", tone: "from-slate-500/70 to-slate-800/60" },
    { label: "Quarantine", value: m.quarantine_candidates || 0, footerLeft: "candidates", footerRight: "review", tone: "from-orange-700/70 to-red-900/60" },
  ] : [
    { label: "Scans", value: c.scans || health.scan_count || 0, footerLeft: "saved", footerRight: `${c.repos || 0} repos`, tone: "from-slate-500/70 to-slate-800/60" },
    { label: "Flaky signals", value: c.flaky_signals || 0, footerLeft: "saved", footerRight: "CI pressure", tone: "from-orange-700/70 to-red-900/60" },
    { label: "Quarantine", value: c.quarantine_candidates || 0, footerLeft: "saved", footerRight: "candidates", tone: "from-amber-600/70 to-yellow-800/50" },
    { label: "GitHub", value: health.github_ready ? "on" : "off", footerLeft: "Actions", footerRight: "read", tone: "from-slate-500/70 to-slate-800/60" },
  ]; },
  hero: (result) => result ? { lead: `${result.metrics?.flaky_signals || 0} signals`, middle: "need", highlight: "attention." } : { lead: "Unstable checks", middle: "need", highlight: "evidence." },
  status: (result, overview) => ({ label: result?.trend?.status || (result ? "scanned" : "—"), detail: result?.summary || "Scan workflow history to begin", progress: result ? "100%" : "8%", stats: [["Runs", result?.metrics?.workflow_runs || 0], ["Reruns", result?.metrics?.rerun_like_runs || 0], ["Scans", overview?.counts?.scans || 0]] }),
  chips: (result, health) => [result?.repo || "No repository selected", result?.workflow_name || "All workflows", health.github_ready ? "GitHub ready" : "Token missing"],
  targetSubtitle: (result) => result ? [result.branch || "all branches", result.workflow_name || "all workflows"].join(" · ") : "Actions history",
  historyTitle: (entry) => `${entry.repo} · ${entry.workflow_name || entry.branch || "Actions"}`,
};

export default function App() { const auth = useApiKeyAuth({ apiBase: API, storageKey: "flake-sting_api_key" }); const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]); if (!auth.checked) return <ProductShell productKey={config.productKey} footerLeft="PatchHive · FlakeSting"><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>; if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />; return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />; }

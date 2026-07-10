import { useMemo } from "react";
import { PackageSearch } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import { IntegratedProductApp, ProductLoginScreen, ProductShell, V3_TEXT } from "@patchhivehq/ui-v3";
import { API } from "./config.js";

const itemTone = (value) => String(value || "").includes("now") || String(value || "").includes("fix") ? "hot" : String(value || "").includes("watch") ? "warn" : "ok";
const config = {
  productKey: "dep-triage", name: "DepTriage", subtitle: "dependency queue", icon: PackageSearch,
  workspaceLabel: "Triage", eyebrow: "Dependency decisions", queueLabel: "Update queue",
  description: "Turns dependency pull requests and Dependabot alerts into update-now, watch, and safe-defer decisions.",
  runLabel: "Run scan", runningLabel: "Scanning…", actionPath: "/scan/github/dependencies", formTitle: "Choose a dependency scope.",
  sourceHelp: "The unified backend reads dependency pull requests and optionally Dependabot alerts without changing repository code.",
  searchPlaceholder: "Search package, manifest, reason…", emptyLabel: "No dependency items match this view.",
  defaultForm: { repo: "", pr_limit: "25", include_alerts: false },
  fields: [
    { key: "repo", label: "Repository", placeholder: "owner/repository", icon: "github", primary: true },
    { key: "pr_limit", label: "Pull request limit", type: "number", min: 5 },
    { key: "include_alerts", label: "Include Dependabot alerts", type: "checkbox" },
  ],
  serialize: (form, health) => ({ repo: form.repo, pr_limit: Number(form.pr_limit) || 25, include_alerts: Boolean(form.include_alerts) && Boolean(health.github_ready || health.github?.token_configured) }),
  formFromResult: (result) => ({ repo: result.repo }),
  items: (result) => result?.items || [],
  mapItem: (item) => { const t = itemTone(item.recommendation); return { id: item.key || item.package_name, title: item.package_name || item.key || "Dependency", meta: [item.ecosystem, item.update_kind, item.manifests?.[0]].filter(Boolean).join(" · "), summary: item.summary || item.reasons?.[0], evidence: [...(item.reasons || []), ...(item.evidence || [])], link: item.pull_requests?.[0]?.html_url || item.alerts?.[0]?.html_url, score: item.score || 0, status: String(item.recommendation || "watch").replaceAll("_", " "), tone: t }; },
  metrics: (result, overview, health) => { const m = result?.metrics || {}; const c = overview?.counts || {}; return result ? [
    { label: "Update now", value: m.update_now || 0, footerLeft: "urgent", footerRight: "act first", tone: "from-orange-700/70 to-red-900/60" },
    { label: "Watch", value: m.watch || 0, footerLeft: "monitor", footerRight: "batch later", tone: "from-amber-600/70 to-yellow-800/50" },
    { label: "Safe defers", value: m.ignore_for_now || 0, footerLeft: "ignore", footerRight: "low churn", tone: "from-emerald-700/70 to-teal-900/60" },
    { label: "Runtime", value: m.runtime_updates || 0, footerLeft: "updates", footerRight: `${m.open_alerts || 0} alerts`, tone: "from-slate-500/70 to-slate-800/60" },
  ] : [
    { label: "Scans", value: c.scans || health.scan_count || 0, footerLeft: "saved", footerRight: `${c.repos || 0} repos`, tone: "from-slate-500/70 to-slate-800/60" },
    { label: "Update now", value: c.update_now || 0, footerLeft: "saved", footerRight: "urgent", tone: "from-orange-700/70 to-red-900/60" },
    { label: "Watch", value: c.watch || 0, footerLeft: "saved", footerRight: "monitor", tone: "from-amber-600/70 to-yellow-800/50" },
    { label: "Safe defers", value: c.ignore_for_now || 0, footerLeft: "saved", footerRight: "ignore", tone: "from-emerald-700/70 to-teal-900/60" },
  ]; },
  hero: (result) => result ? { lead: `${result.metrics?.tracked_items || 0} dependencies`, middle: "need a", highlight: "decision." } : { lead: "Dependency noise", middle: "needs a", highlight: "priority." },
  status: (result, overview) => ({ label: result?.metrics?.update_now ? "act" : result ? "watch" : "—", detail: result?.summary || "Scan a repository to begin", progress: result ? "100%" : "8%", stats: [["Tracked", result?.metrics?.tracked_items || 0], ["Alerts", result?.metrics?.open_alerts || 0], ["Scans", overview?.counts?.scans || 0]] }),
  chips: (result, health) => [result?.repo || "No repository selected", `${result?.metrics?.dependency_pull_requests || 0} dependency PRs`, health.github_ready ? "GitHub ready" : "Token missing"],
  targetSubtitle: (result) => result ? `${result.metrics?.tracked_items || 0} tracked dependency items` : "Dependency intake",
  historyTitle: (entry) => entry.repo,
};

export default function App() { const auth = useApiKeyAuth({ apiBase: API, storageKey: "dep-triage_api_key" }); const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]); if (!auth.checked) return <ProductShell productKey={config.productKey}><div className={`min-h-screen grid place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>; if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={config} />; return <IntegratedProductApp apiBase={API} auth={auth} config={config} fetcher={fetcher} />; }

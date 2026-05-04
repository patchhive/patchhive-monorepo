import { useCallback, useEffect, useState } from "react";
import { applyTheme } from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  ProductSetupWizard,
  useApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import { API } from "./config.js";
import ScanPanel from "./panels/ScanPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";
import ControlsPanel from "./panels/ControlsPanel.jsx";

const TABS = [
  { id: "scan", label: "📡 Scan" },
  { id: "setup", label: "Setup" },
  { id: "history", label: "◎ History" },
  { id: "controls", label: "Controls" },
  { id: "checks", label: "Checks" },
];

const SETUP_STEPS = [
  {
    title: "Connect GitHub for issue and marker reads",
    detail: "Set BOT_GITHUB_TOKEN so SignalHive can read repos, issues, and optional TODO/FIXME code-search markers without partial results.",
    tab: "checks",
    actionLabel: "Review Checks",
  },
  {
    title: "Define a safe scan scope",
    detail: "Use allowlists, denylists, schedules, and saved presets before widening discovery across more repositories.",
    tab: "controls",
    actionLabel: "Open Controls",
  },
  {
    title: "Run a small first scan",
    detail: "Start with a narrow topic or low repo count so you can judge signal quality before letting the queue get broader.",
    tab: "scan",
    actionLabel: "Open Scan",
  },
];

const DEFAULT_PARAMS = {
  search_query: "",
  topics: "",
  languages: "rust,typescript,python",
  min_stars: "25",
  max_repos: "8",
  issues_per_repo: "30",
  stale_days: "45",
};

function toList(value) {
  return value
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
}

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "signal_api_key",
  });
  const [tab, setTab] = useState("scan");
  const [running, setRunning] = useState(false);
  const [params, setParams] = useState(DEFAULT_PARAMS);
  const [scan, setScan] = useState(null);
  const [error, setError] = useState("");

  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("signal-hive");
  }, []);

  const runScan = useCallback(async () => {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/scan`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          search_query: params.search_query,
          topics: toList(params.topics),
          languages: toList(params.languages),
          min_stars: Number(params.min_stars) || 25,
          max_repos: Number(params.max_repos) || 8,
          issues_per_repo: Number(params.issues_per_repo) || 30,
          stale_days: Number(params.stale_days) || 45,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "Signal scan failed");
      }
      setScan(data);
      setTab("scan");
    } catch (err) {
      setError(err.message || "Signal scan failed");
    } finally {
      setRunning(false);
    }
  }, [fetch_, params]);

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="📡"
      title="SignalHive"
      storageKey="signal_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
      loadingColor="#2a6aaa"
    >
      <ProductAppFrame
        icon="📡"
        title="SignalHive"
        product="SignalHive"
        running={running}
        headerChildren={
          <>
            <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
              Read-only maintenance reconnaissance
            </div>
            {scan?.summary?.total_signals > 0 && (
              <div style={{ fontSize: 10, color: "var(--accent)" }}>
                {scan.summary.total_signals} signals
              </div>
            )}
            {scan?.warnings?.length > 0 && (
              <div style={{ fontSize: 10, color: "var(--gold)" }}>
                {scan.warnings.length} warning{scan.warnings.length === 1 ? "" : "s"}
              </div>
            )}
          </>
        }
        tabs={TABS}
        activeTab={tab}
        onTabChange={setTab}
        error={error}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
        {tab === "setup" && (
          <ProductSetupWizard
            apiBase={API}
            fetch_={fetch_}
            product="SignalHive"
            icon="📡"
            description="Use the same setup flow everywhere outside HiveCore: confirm local operator access, clear startup checks, then tune a narrow first scan before widening the queue."
            steps={SETUP_STEPS}
            onOpenTab={setTab}
          />
        )}
        {tab === "scan" && (
          <ScanPanel
            apiKey={apiKey}
            params={params}
            setParams={setParams}
            running={running}
            onRun={runScan}
            scan={scan}
            setScan={setScan}
          />
        )}
        {tab === "history" && <HistoryPanel apiKey={apiKey} />}
        {tab === "controls" && <ControlsPanel apiKey={apiKey} />}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </ProductAppFrame>
    </ProductSessionGate>
  );
}

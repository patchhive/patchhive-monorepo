import { useCallback, useEffect, useState } from "react";
import { applyTheme } from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  ProductSetupWizard,
  useApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import "./App.css";
import { API } from "./config.js";
import { sortRepos, SORT_OPTIONS } from "./sort.js";
import { buildDashboardSummary, downloadTextFile, summarizeScanHighlights, timeAgo } from "./report.js";
import SignalCard from "./components/SignalCard.jsx";
import ScanForm from "./components/ScanForm.jsx";
import HistoryPanel from "./components/HistoryPanel.jsx";
import ControlsPanel from "./components/ControlsPanel.jsx";
import ChecksPanel from "./components/ChecksPanel.jsx";

const TABS = [
  { id: "scan", label: "🔍 Scan", icon: "🔍" },
  { id: "history", label: "◎ History", icon: "◎" },
  { id: "controls", label: "⚙️ Controls", icon: "⚙️" },
  { id: "checks", label: "✓ Checks", icon: "✓" },
];

const SETUP_STEPS = [
  {
    title: "Connect GitHub for issue and marker reads",
    detail: "Set BOT_GITHUB_TOKEN so SignalHive can read repos, issues, and TODO/FIXME code-search markers without partial results.",
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

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "signal_api_key",
  });
  const [tab, setTab] = useState("scan");
  const [running, setRunning] = useState(false);
  const [params, setParams] = useState(DEFAULT_PARAMS);
  const [scan, setScan] = useState(null);
  const [scanHistory, setScanHistory] = useState([]);
  const [error, setError] = useState("");
  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => { applyTheme("signal-hive"); }, []);

  useEffect(() => {
    if (!apiKey) return;
    fetch_(`${API}/history`)
      .then(r => r.json())
      .then(data => setScanHistory(data.scans || []))
      .catch(() => {});
  }, [apiKey, scan?.id]);

  const runScan = useCallback(async (scanParams) => {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/scan`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(scanParams),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Signal scan failed");
      setScan(data);
    } catch (err) {
      setError(err.message || "Scan failed. Check your connection and API key.");
    } finally {
      setRunning(false);
    }
  }, [fetch_]);

  const loadScan = useCallback(async (scanId) => {
    try {
      const res = await fetch_(`${API}/history/${scanId}`);
      const data = await res.json();
      if (res.ok) setScan(data);
    } catch {}
  }, [fetch_]);

  const downloadReport = useCallback(async (scanId) => {
    try {
      const res = await fetch_(`${API}/history/${scanId}/report`);
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Export failed");
      downloadTextFile(data.filename || `signalhive-report-${scanId}.md`, data.markdown, "text/markdown;charset=utf-8");
    } catch (err) {
      setError(err.message || "Could not export report.");
    }
  }, [fetch_]);

  const content = () => {
    if (!apiKey) return null;

    return (
      <div className="app-shell">
        {/* Sidebar */}
        <aside className="sidebar">
          <div className="sidebar-brand">
            <span className="sidebar-brand-icon">🔍</span>
            SignalHive
          </div>
          <nav className="sidebar-nav">
            {TABS.map(t => (
              <button key={t.id} className={`nav-item ${tab === t.id ? "active" : ""}`} onClick={() => setTab(t.id)}>
                <span>{t.icon}</span>
                <span>{t.label.replace(/^[^\s]+\s/, "")}</span>
                {t.id === "scan" && scan?.repos?.length > 0 && <span className="nav-count">{scan.repos.length}</span>}
              </button>
            ))}
          </nav>
          <div className="sidebar-footer">
            <span>SignalHive by PatchHive</span>
            <a href="/">← Back to PatchHive</a>
            <button className="btn-ghost" onClick={logout} style={{ padding: "4px 0", textAlign: "left", fontSize: 11 }}>Sign out</button>
          </div>
        </aside>

        {/* Main Content */}
        <main className="main-area">
          <div className="main-header">
            <h1>{TABS.find(t => t.id === tab)?.label || "SignalHive"}</h1>
            <p>
              {tab === "scan" && "Discover maintenance signals across repositories — stale issues, duplicates, TODO hotspots, and recurring bugs."}
              {tab === "history" && "Past scan results and exported reports."}
              {tab === "controls" && "Manage presets and scheduled scans."}
              {tab === "checks" && "Verify SignalHive is configured and ready."}
            </p>
          </div>

          {error && (
            <div style={{ padding: "12px 16px", marginBottom: 16, borderRadius: "var(--radius-sm)", background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.2)", color: "#f87171", fontSize: 13 }}>
              {error}
              <button className="btn-ghost" style={{ marginLeft: 12, fontSize: 12 }} onClick={() => setError("")}>Dismiss</button>
            </div>
          )}

          <div className="animate-in" key={tab}>
            {tab === "scan" && <ScanForm apiKey={apiKey} params={params} setParams={setParams} running={running} onRun={runScan} scan={scan} setScan={setScan} loadScan={loadScan} downloadReport={downloadReport} />}
            {tab === "history" && <HistoryPanel apiKey={apiKey} scans={scanHistory} loadScan={loadScan} downloadReport={downloadReport} />}
            {tab === "controls" && <ControlsPanel apiKey={apiKey} params={params} setParams={setParams} fetch_={fetch_} />}
            {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
          </div>
        </main>
      </div>
    );
  };

  return (
    <ProductAppFrame>
      <ProductSessionGate
        needsAuth={needsAuth}
        checked={checked}
        login={login}
        authError={authError}
      >
        <ProductSetupWizard
          bootstrapRequired={bootstrapRequired}
          generateKey={generateKey}
          steps={SETUP_STEPS}
          onNavigate={(tabId) => setTab(tabId)}
          tab={tab}
        >
          {content()}
        </ProductSetupWizard>
      </ProductSessionGate>
    </ProductAppFrame>
  );
}

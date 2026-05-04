import { useEffect, useState } from "react";
import { applyTheme } from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  ProductSetupWizard,
  useApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import { API } from "./config.js";
import ScoutPanel from "./panels/ScoutPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "scout", label: "🧭 Scout" },
  { id: "setup", label: "Setup" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

const SETUP_STEPS = [
  {
    title: "Keep the scan on local allowlisted repo paths",
    detail: "RefactorScout should stay conservative. Confirm the backend can see only the repo paths you actually intend to inspect.",
    tab: "checks",
    actionLabel: "Review Checks",
  },
  {
    title: "Start with one repo path and a modest file cap",
    detail: "Use one local repository and a smaller max file count first so the refactor leads stay explainable before you widen the analysis.",
    tab: "scout",
    actionLabel: "Open Scout",
  },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } =
    useApiKeyAuth({
      apiBase: API,
      storageKey: "refactor-scout_api_key",
    });
  const [tab, setTab] = useState("scout");
  const [form, setForm] = useState({
    repo_path: "",
    max_files: "250",
  });
  const [scan, setScan] = useState(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("refactor-scout");
  }, []);

  async function runScan() {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/scan/local`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo_path: form.repo_path.trim(),
          max_files: Number(form.max_files) || 250,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "RefactorScout could not scan that repo path.");
      }
      setScan(data);
      setForm((prev) => ({
        ...prev,
        repo_path: data.repo_path || prev.repo_path,
      }));
      setTab("scout");
    } catch (err) {
      setError(err.message || "RefactorScout could not scan that repo path.");
    } finally {
      setRunning(false);
    }
  }

  async function loadHistoryScan(id) {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/history/${id}`);
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "RefactorScout could not load that scan.");
      }
      setScan(data);
      setForm((prev) => ({
        ...prev,
        repo_path: data.repo_path || prev.repo_path,
      }));
      setTab("scout");
    } catch (err) {
      setError(err.message || "RefactorScout could not load that scan.");
    } finally {
      setRunning(false);
    }
  }

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="🧭"
      title="RefactorScout"
      storageKey="refactor-scout_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
      loadingColor="#2a8a4a"
    >
      <ProductAppFrame
        icon="🧭"
        title="RefactorScout"
        product="RefactorScout"
        running={running}
        headerChildren={
          <>
            <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
              Surface safe, high-value refactors before code quality drift turns expensive.
            </div>
            {scan?.metrics?.high_safety > 0 && (
              <div style={{ fontSize: 10, color: "var(--green)", fontWeight: 700 }}>
                {scan.metrics.high_safety} high-safety leads
              </div>
            )}
            {scan?.metrics?.medium_safety > 0 && (
              <div style={{ fontSize: 10, color: "var(--accent)" }}>
                {scan.metrics.medium_safety} medium-safety leads
              </div>
            )}
          </>
        }
        tabs={TABS}
        activeTab={tab}
        onTabChange={setTab}
        error={error}
        maxWidth={1240}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
        {tab === "setup" && (
          <ProductSetupWizard
            apiBase={API}
            fetch_={fetch_}
            product="RefactorScout"
            icon="🧭"
            description="RefactorScout should stay narrow and credible. This setup path keeps the local-scan rules obvious before you trust the refactor queue."
            steps={SETUP_STEPS}
            onOpenTab={setTab}
          />
        )}
        {tab === "scout" && (
          <ScoutPanel
            apiKey={apiKey}
            form={form}
            setForm={setForm}
            running={running}
            onRun={runScan}
            scan={scan}
          />
        )}
        {tab === "history" && (
          <HistoryPanel
            apiKey={apiKey}
            onLoadScan={loadHistoryScan}
            activeScanId={scan?.id || ""}
          />
        )}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </ProductAppFrame>
    </ProductSessionGate>
  );
}

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
import TriagePanel from "./panels/TriagePanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "triage", label: "📦 Triage" },
  { id: "setup", label: "Setup" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

const SETUP_STEPS = [
  {
    title: "Connect GitHub for dependency and alert reads",
    detail: "DepTriage needs repository reads first, and it gets better once dependency alert access is available for urgency scoring.",
    tab: "checks",
    actionLabel: "Review Checks",
  },
  {
    title: "Start with one repository",
    detail: "Run the queue against a single repo first so you can judge what lands in update now, watch, and ignore before scaling out.",
    tab: "triage",
    actionLabel: "Open Triage",
  },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "dep-triage_api_key",
  });
  const [tab, setTab] = useState("triage");
  const [form, setForm] = useState({
    repo: "",
    pr_limit: "25",
    include_alerts: "yes",
  });
  const [scan, setScan] = useState(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("dep-triage");
  }, []);

  async function runScan() {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/scan/github/dependencies`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo.trim(),
          pr_limit: Number(form.pr_limit) || 25,
          include_alerts: form.include_alerts === "yes",
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "DepTriage could not scan that repository.");
      }
      setScan(data);
      setForm((prev) => ({ ...prev, repo: data.repo || prev.repo }));
      setTab("triage");
    } catch (err) {
      setError(err.message || "DepTriage could not scan that repository.");
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
        throw new Error(data.error || "DepTriage could not load that scan.");
      }
      setScan(data);
      setForm((prev) => ({ ...prev, repo: data.repo || prev.repo }));
      setTab("triage");
    } catch (err) {
      setError(err.message || "DepTriage could not load that scan.");
    } finally {
      setRunning(false);
    }
  }

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="📦"
      title="DepTriage"
      storageKey="dep-triage_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
    >
      <ProductAppFrame
        icon="📦"
        title="DepTriage"
        product="DepTriage"
        running={running}
        headerChildren={
          <>
            <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
              Tell teams which dependency updates matter now and which ones can wait.
            </div>
            {scan?.metrics?.update_now > 0 && (
              <div style={{ fontSize: 10, color: "var(--accent)", fontWeight: 700 }}>
                {scan.metrics.update_now} UPDATE NOW
              </div>
            )}
          </>
        }
        tabs={TABS}
        activeTab={tab}
        onTabChange={setTab}
        error={error}
        maxWidth={1200}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
        {tab === "setup" && (
          <ProductSetupWizard
            apiBase={API}
            fetch_={fetch_}
            product="DepTriage"
            icon="📦"
            description="DepTriage should feel like a focused queue, not more dependency noise. Clear backend readiness, then validate the ranking flow on one repository first."
            steps={SETUP_STEPS}
            onOpenTab={setTab}
          />
        )}
        {tab === "triage" && (
          <TriagePanel
            apiKey={apiKey}
            form={form}
            setForm={setForm}
            running={running}
            onRun={runScan}
            scan={scan}
            onLoadScan={loadHistoryScan}
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

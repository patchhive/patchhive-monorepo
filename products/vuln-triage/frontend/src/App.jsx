import { useEffect, useState } from "react";
import { applyTheme } from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  useApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import { API } from "./config.js";
import TriagePanel from "./panels/TriagePanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "triage", label: "🛡 Triage" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "vuln-triage_api_key",
  });
  const [tab, setTab] = useState("triage");
  const [form, setForm] = useState({
    repo: "",
    include_code_scanning: "yes",
    include_dependency_alerts: "yes",
  });
  const [scan, setScan] = useState(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("vuln-triage");
  }, []);

  async function runScan() {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/scan/github/findings`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo.trim(),
          include_code_scanning: form.include_code_scanning === "yes",
          include_dependency_alerts: form.include_dependency_alerts === "yes",
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "VulnTriage could not scan that repository.");
      }
      setScan(data);
      setForm((prev) => ({ ...prev, repo: data.repo || prev.repo }));
      setTab("triage");
    } catch (err) {
      setError(err.message || "VulnTriage could not scan that repository.");
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
        throw new Error(data.error || "VulnTriage could not load that scan.");
      }
      setScan(data);
      setForm((prev) => ({ ...prev, repo: data.repo || prev.repo }));
      setTab("triage");
    } catch (err) {
      setError(err.message || "VulnTriage could not load that scan.");
    } finally {
      setRunning(false);
    }
  }

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="🛡"
      title="VulnTriage"
      storageKey="vuln-triage_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
    >
      <ProductAppFrame
        icon="🛡"
        title="VulnTriage"
        product="VulnTriage"
        running={running}
        headerChildren={
          <>
            <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
              Turn security findings into ranked, actionable engineering work instead of a wall of alerts.
            </div>
            {scan?.metrics?.fix_now > 0 && (
              <div style={{ fontSize: 10, color: "var(--accent)", fontWeight: 700 }}>
                {scan.metrics.fix_now} FIX NOW
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
        {tab === "triage" && (
          <TriagePanel
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

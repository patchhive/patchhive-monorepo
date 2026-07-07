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
  { id: "triage", label: "🛡 Triage" },
  { id: "setup", label: "Setup" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

const SETUP_STEPS = [
  {
    title: "Connect GitHub security reads first",
    detail: "VulnTriage becomes trustworthy once code scanning and dependency alert access are available for the repositories you plan to triage.",
    tab: "checks",
    actionLabel: "Review Checks",
  },
  {
    title: "Validate the ranking on one repo",
    detail: "Start with a single repository and inspect what lands in fix now, plan next, and watch before acting on the queue more broadly.",
    tab: "triage",
    actionLabel: "Open Triage",
  },
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
        {tab === "setup" && (
          <ProductSetupWizard
            apiBase={API}
            fetch_={fetch_}
            product="VulnTriage"
            icon="🛡"
            description="VulnTriage should turn noisy findings into a practical engineering queue. The shared setup wizard keeps the boot path clear before you trust that ranking."
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

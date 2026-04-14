import { useCallback, useEffect, useState } from "react";
import { applyTheme } from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  useApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import { API } from "./config.js";
import OverviewPanel from "./panels/OverviewPanel.jsx";
import IngestPanel from "./panels/IngestPanel.jsx";
import MemoryPanel from "./panels/MemoryPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "overview", label: "🧠 Overview" },
  { id: "ingest", label: "◎ Ingest" },
  { id: "memory", label: "Memory" },
  { id: "history", label: "History" },
  { id: "checks", label: "Checks" },
];

const DEFAULT_FORM = {
  repo: "",
  merged_pr_limit: "18",
  issue_limit: "24",
  since_days: "180",
};

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "repo-memory_api_key",
  });
  const [tab, setTab] = useState("overview");
  const [running, setRunning] = useState(false);
  const [form, setForm] = useState(DEFAULT_FORM);
  const [run, setRun] = useState(null);
  const [activeRepo, setActiveRepo] = useState("");
  const [error, setError] = useState("");
  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("repo-memory");
  }, []);

  const runIngest = useCallback(async () => {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/ingest`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo.trim(),
          merged_pr_limit: Number(form.merged_pr_limit) || 18,
          issue_limit: Number(form.issue_limit) || 24,
          since_days: Number(form.since_days) || 180,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "RepoMemory could not ingest that repo.");
      }
      setRun(data);
      setActiveRepo(data.repo);
      setForm((prev) => ({ ...prev, repo: data.repo }));
      setTab("memory");
    } catch (err) {
      setError(err.message || "RepoMemory could not ingest that repo.");
    } finally {
      setRunning(false);
    }
  }, [fetch_, form]);

  const loadRun = useCallback(
    async (id) => {
      setRunning(true);
      setError("");
      try {
        const res = await fetch_(`${API}/history/${id}`);
        const data = await res.json();
        if (!res.ok) {
          throw new Error(data.error || "RepoMemory could not load that run.");
        }
        setRun(data);
        setActiveRepo(data.repo);
        setForm((prev) => ({ ...prev, repo: data.repo }));
        setTab("memory");
      } catch (err) {
        setError(err.message || "RepoMemory could not load that run.");
      } finally {
        setRunning(false);
      }
    },
    [fetch_]
  );

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="🧠"
      title="RepoMemory"
      storageKey="repo-memory_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
      loadingColor="#2a8a4a"
    >
      <ProductAppFrame
        icon="🧠"
        title="RepoMemory"
        product="RepoMemory"
        running={running}
        headerChildren={
          <>
            <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
              Durable repo memory from merged history and review pain
            </div>
            {activeRepo && (
              <div style={{ fontSize: 10, color: "var(--accent)" }}>
                {activeRepo}
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
        {tab === "overview" && (
          <OverviewPanel apiKey={apiKey} activeRepo={activeRepo} setActiveRepo={setActiveRepo} onOpenIngest={() => setTab("ingest")} />
        )}
        {tab === "ingest" && (
          <IngestPanel
            form={form}
            setForm={setForm}
            running={running}
            onRun={runIngest}
            run={run}
            onShowMemory={() => setTab("memory")}
          />
        )}
        {tab === "memory" && (
          <MemoryPanel
            apiKey={apiKey}
            activeRepo={activeRepo}
            setActiveRepo={setActiveRepo}
            currentRun={run}
          />
        )}
        {tab === "history" && (
          <HistoryPanel
            apiKey={apiKey}
            activeRepo={activeRepo}
            onLoadRun={loadRun}
          />
        )}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </ProductAppFrame>
    </ProductSessionGate>
  );
}

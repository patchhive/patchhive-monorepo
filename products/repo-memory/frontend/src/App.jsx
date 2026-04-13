import { useCallback, useEffect, useState } from "react";
import {
  applyTheme,
  Btn,
  LoginPage,
  PatchHiveFooter,
  PatchHiveHeader,
  TabBar,
} from "@patchhivehq/ui";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell";
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
  const fetch_ = createApiFetcher(apiKey);

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

  if (!checked) {
    return (
      <div style={{ minHeight: "100vh", background: "#080810", display: "flex", alignItems: "center", justifyContent: "center", color: "#2a8a4a", fontSize: 26 }}>
        🧠
      </div>
    );
  }

  if (needsAuth) {
    return (
      <LoginPage
        onLogin={login}
        icon="🧠"
        title="RepoMemory"
        subtitle="by PatchHive"
        storageKey="repo-memory_api_key"
        apiBase={API}
        authError={authError}
        bootstrapRequired={bootstrapRequired}
        onGenerateKey={generateKey}
      />
    );
  }

  return (
    <div style={{ minHeight: "100vh", background: "var(--bg)", color: "var(--text)", fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: 12 }}>
      <PatchHiveHeader icon="🧠" title="RepoMemory" version="v0.1.0" running={running}>
        <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
          Durable repo memory from merged history and review pain
        </div>
        {activeRepo && (
          <div style={{ fontSize: 10, color: "var(--accent)" }}>
            {activeRepo}
          </div>
        )}
        {apiKey && (
          <Btn onClick={logout} style={{ padding: "4px 10px" }}>
            Sign out
          </Btn>
        )}
      </PatchHiveHeader>

      <TabBar tabs={TABS} active={tab} onChange={setTab} />

      <div style={{ padding: 24, maxWidth: 1320, margin: "0 auto", display: "grid", gap: 16 }}>
        {error && (
          <div style={{ border: "1px solid var(--accent)44", background: "var(--accent)10", color: "var(--accent)", borderRadius: 8, padding: "12px 14px" }}>
            {error}
          </div>
        )}

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
      </div>

      <PatchHiveFooter product="RepoMemory" />
    </div>
  );
}

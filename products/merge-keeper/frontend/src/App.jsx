import { useEffect, useState } from "react";
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
import KeeperPanel from "./panels/KeeperPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "keeper", label: "🪢 Keeper" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout } = useApiKeyAuth({
    apiBase: API,
    storageKey: "merge-keeper_api_key",
  });
  const [tab, setTab] = useState("keeper");
  const [form, setForm] = useState({
    repo: "",
    pr_number: "",
  });
  const [assessment, setAssessment] = useState(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("merge-keeper");
  }, []);

  async function runAssessment() {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/assess/github/pr`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo,
          pr_number: Number(form.pr_number) || 0,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "MergeKeeper could not assess that pull request.");
      }
      setAssessment(data);
      setTab("keeper");
    } catch (err) {
      setError(err.message || "MergeKeeper could not assess that pull request.");
    } finally {
      setRunning(false);
    }
  }

  async function loadHistoryAssessment(id) {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/history/${id}`);
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "MergeKeeper could not load that run.");
      }
      setAssessment(data);
      setForm({
        repo: data.repo || "",
        pr_number: data.pr_number ? String(data.pr_number) : "",
      });
      setTab("keeper");
    } catch (err) {
      setError(err.message || "MergeKeeper could not load that run.");
    } finally {
      setRunning(false);
    }
  }

  if (!checked) {
    return (
      <div style={{ minHeight: "100vh", background: "#080810", display: "flex", alignItems: "center", justifyContent: "center", color: "var(--accent)", fontSize: 26 }}>
        🪢
      </div>
    );
  }

  if (needsAuth) {
    return (
      <LoginPage
        onLogin={login}
        icon="🪢"
        title="MergeKeeper"
        subtitle="by PatchHive"
        storageKey="merge-keeper_api_key"
        apiBase={API}
      />
    );
  }

  return (
    <div style={{ minHeight: "100vh", background: "var(--bg)", color: "var(--text)", fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: 12 }}>
      <PatchHiveHeader icon="🪢" title="MergeKeeper" version="v0.1.0" running={running}>
        <div style={{ fontSize: 10, color: "var(--text-dim)" }}>Keep pull requests mergeable by turning GitHub merge pressure into a clear readiness call.</div>
        {assessment?.readiness && (
          <div
            style={{
              fontSize: 10,
              color:
                assessment.readiness === "ready"
                  ? "var(--green)"
                  : assessment.readiness === "blocked"
                    ? "var(--accent)"
                    : "var(--gold)",
              fontWeight: 700,
            }}
          >
            {assessment.readiness.toUpperCase()}
          </div>
        )}
        {apiKey && (
          <Btn onClick={logout} style={{ padding: "4px 10px" }}>
            Sign out
          </Btn>
        )}
      </PatchHiveHeader>

      <TabBar tabs={TABS} active={tab} onChange={setTab} />

      <div style={{ padding: 24, maxWidth: 1200, margin: "0 auto", display: "grid", gap: 16 }}>
        {error && (
          <div style={{ border: "1px solid var(--accent)44", background: "var(--accent)10", color: "var(--accent)", borderRadius: 8, padding: "12px 14px" }}>
            {error}
          </div>
        )}
        {tab === "keeper" && (
          <KeeperPanel
            apiKey={apiKey}
            form={form}
            setForm={setForm}
            running={running}
            onRun={runAssessment}
            assessment={assessment}
            onLoadAssessment={loadHistoryAssessment}
          />
        )}
        {tab === "history" && (
          <HistoryPanel
            apiKey={apiKey}
            onLoadAssessment={loadHistoryAssessment}
            activeAssessmentId={assessment?.id || ""}
          />
        )}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </div>

      <PatchHiveFooter product="MergeKeeper" />
    </div>
  );
}

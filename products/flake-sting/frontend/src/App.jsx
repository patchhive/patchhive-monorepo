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
import ScanPanel from "./panels/ScanPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";

const TABS = [
  { id: "scan", label: "🦂 Scan" },
  { id: "history", label: "◎ History" },
  { id: "checks", label: "Checks" },
];

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "flake-sting_api_key",
  });
  const [tab, setTab] = useState("scan");
  const [form, setForm] = useState({
    repo: "",
    branch: "",
    workflow_name: "",
    lookback_runs: "25",
  });
  const [scan, setScan] = useState(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    applyTheme("flake-sting");
  }, []);

  async function runScan() {
    setRunning(true);
    setError("");
    try {
      const res = await fetch_(`${API}/scan/github/actions`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          repo: form.repo,
          branch: form.branch,
          workflow_name: form.workflow_name,
          lookback_runs: Number(form.lookback_runs) || 25,
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || "FlakeSting could not scan that repository.");
      }
      setScan(data);
      setTab("scan");
    } catch (err) {
      setError(err.message || "FlakeSting could not scan that repository.");
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
        throw new Error(data.error || "FlakeSting could not load that scan.");
      }
      setScan(data);
      setForm((prev) => ({
        ...prev,
        repo: data.repo || "",
        branch: data.branch || "",
        workflow_name: data.workflow_name || "",
      }));
      setTab("scan");
    } catch (err) {
      setError(err.message || "FlakeSting could not load that scan.");
    } finally {
      setRunning(false);
    }
  }

  if (!checked) {
    return (
      <div style={{ minHeight: "100vh", background: "#080810", display: "flex", alignItems: "center", justifyContent: "center", color: "var(--accent)", fontSize: 26 }}>
        🦂
      </div>
    );
  }

  if (needsAuth) {
    return (
      <LoginPage
        onLogin={login}
        icon="🦂"
        title="FlakeSting"
        subtitle="by PatchHive"
        storageKey="flake-sting_api_key"
        apiBase={API}
        authError={authError}
        bootstrapRequired={bootstrapRequired}
        onGenerateKey={generateKey}
      />
    );
  }

  return (
    <div style={{ minHeight: "100vh", background: "var(--bg)", color: "var(--text)", fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: 12 }}>
      <PatchHiveHeader icon="🦂" title="FlakeSting" version="v0.1.0" running={running}>
        <div style={{ fontSize: 10, color: "var(--text-dim)" }}>Spot flaky CI patterns before unreliable checks erode team trust.</div>
        {scan?.metrics?.quarantine_candidates > 0 && (
          <div style={{ fontSize: 10, color: "var(--accent)", fontWeight: 700 }}>
            {scan.metrics.quarantine_candidates} QUARANTINE CANDIDATE{scan.metrics.quarantine_candidates === 1 ? "" : "S"}
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
        {tab === "scan" && (
          <ScanPanel
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
      </div>

      <PatchHiveFooter product="FlakeSting" />
    </div>
  );
}

import { useCallback, useEffect, useState } from "react";
import {
  applyTheme,
  Btn,
  LoginPage,
  PatchHiveFooter,
  PatchHiveHeader,
  TabBar,
} from "@patchhivehq/ui";
import { API } from "./config.js";
import ScanPanel from "./panels/ScanPanel.jsx";
import HistoryPanel from "./panels/HistoryPanel.jsx";
import ChecksPanel from "./panels/ChecksPanel.jsx";
import ControlsPanel from "./panels/ControlsPanel.jsx";

const TABS = [
  { id: "scan", label: "📡 Scan" },
  { id: "history", label: "◎ History" },
  { id: "controls", label: "Controls" },
  { id: "checks", label: "Checks" },
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

function useAuth() {
  const [apiKey, setApiKey] = useState(() => localStorage.getItem("signal_api_key") || "");
  const [checked, setChecked] = useState(false);
  const [needsAuth, setNeedsAuth] = useState(false);

  useEffect(() => {
    fetch(`${API}/auth/status`)
      .then((res) => res.json())
      .then((data) => {
        if (!data.auth_enabled) {
          setChecked(true);
          return;
        }
        const stored = localStorage.getItem("signal_api_key");
        if (!stored) {
          setNeedsAuth(true);
          setChecked(true);
          return;
        }
        fetch(`${API}/auth/login`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ api_key: stored }),
        }).then((res) => {
          if (res.ok) {
            setApiKey(stored);
          } else {
            setNeedsAuth(true);
          }
          setChecked(true);
        });
      })
      .catch(() => setChecked(true));
  }, []);

  const login = (key) => {
    localStorage.setItem("signal_api_key", key);
    setApiKey(key);
    setNeedsAuth(false);
  };

  const logout = () => {
    localStorage.removeItem("signal_api_key");
    setApiKey("");
    setNeedsAuth(true);
  };

  return { apiKey, checked, needsAuth, login, logout };
}

const af = (key) => (url, opts = {}) =>
  fetch(url, {
    ...opts,
    headers: { ...(opts.headers || {}), ...(key ? { "X-API-Key": key } : {}) },
  });

export default function App() {
  const { apiKey, checked, needsAuth, login, logout } = useAuth();
  const [tab, setTab] = useState("scan");
  const [running, setRunning] = useState(false);
  const [params, setParams] = useState(DEFAULT_PARAMS);
  const [scan, setScan] = useState(null);
  const [error, setError] = useState("");

  const fetch_ = af(apiKey);

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
  }, [apiKey, params]);

  if (!checked) {
    return (
      <div style={{ minHeight: "100vh", background: "#080810", display: "flex", alignItems: "center", justifyContent: "center", color: "#2a6aaa", fontSize: 26 }}>
        📡
      </div>
    );
  }

  if (needsAuth) {
    return (
      <LoginPage
        onLogin={login}
        icon="📡"
        title="SignalHive"
        subtitle="by PatchHive"
        storageKey="signal_api_key"
        apiBase={API}
      />
    );
  }

  return (
    <div style={{ minHeight: "100vh", background: "var(--bg)", color: "var(--text)", fontFamily: "'SF Mono','Fira Mono',monospace", fontSize: 12 }}>
      <PatchHiveHeader icon="📡" title="SignalHive" version="v0.1.0" running={running}>
        <div style={{ fontSize: 10, color: "var(--text-dim)" }}>
          Read-only maintenance reconnaissance
        </div>
        {scan?.summary?.total_signals > 0 && (
          <div style={{ fontSize: 10, color: "var(--accent)" }}>
            {scan.summary.total_signals} signals
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

        {tab === "scan" && (
          <ScanPanel
            apiKey={apiKey}
            params={params}
            setParams={setParams}
            running={running}
            onRun={runScan}
            scan={scan}
          />
        )}
        {tab === "history" && <HistoryPanel apiKey={apiKey} />}
        {tab === "controls" && <ControlsPanel apiKey={apiKey} />}
        {tab === "checks" && <ChecksPanel apiKey={apiKey} />}
      </div>

      <PatchHiveFooter product="SignalHive" />
    </div>
  );
}

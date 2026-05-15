import { useState, useEffect } from "react";
import { API } from "../config.js";

function createApiFetcher(key) {
  return (url, opts = {}) => fetch(url, {
    ...opts,
    headers: { ...opts.headers, "X-API-Key": key },
  });
}

export default function ChecksPanel({ apiKey }) {
  const [checks, setChecks] = useState([]);
  const [running, setRunning] = useState(false);
  const [smokeResult, setSmokeResult] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  const runSmoke = async () => {
    setRunning(true);
    try {
      const res = await fetch_(`${API}/smoke`, { method: "POST" });
      const data = await res.json();
      setSmokeResult({ ok: res.ok, data });
    } catch {
      setSmokeResult({ ok: false, data: { error: "Connection failed" } });
    } finally {
      setRunning(false);
    }
  };

  useEffect(() => {
    if (!apiKey) return;
    const items = [
      { label: "API Connection", check: () => fetch_(`${API}/capabilities`).then(r => r.ok), detail: `${API}/capabilities` },
      { label: "History Endpoint", check: () => fetch_(`${API}/history`).then(r => r.ok), detail: `${API}/history` },
      { label: "Presets Endpoint", check: () => fetch_(`${API}/presets`).then(r => r.ok), detail: `${API}/presets` },
      { label: "Schedules Endpoint", check: () => fetch_(`${API}/schedules`).then(r => r.ok), detail: `${API}/schedules` },
    ];
    Promise.all(items.map(async (item) => {
      try {
        const ok = await item.check();
        return { ...item, status: ok ? "ok" : "fail" };
      } catch {
        return { ...item, status: "fail" };
      }
    })).then(setChecks);
  }, [apiKey]);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      {/* Smoke Test */}
      <div className="card">
        <div className="card-header">
          <h3>Smoke Test</h3>
          <button className="btn btn-primary btn-sm" onClick={runSmoke} disabled={running}>
            {running ? "Running…" : "Run Smoke Test"}
          </button>
        </div>
        <div className="card-body">
          {smokeResult ? (
            <div style={{ display: "grid", gap: 8 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span className={`check-status ${smokeResult.ok ? "check-ok" : "check-fail"}`} />
                <span style={{ fontSize: 13, fontWeight: 600 }}>{smokeResult.ok ? "All systems operational" : "Smoke test failed"}</span>
              </div>
              {smokeResult.data && (
                <pre style={{ fontSize: 11, color: "var(--text-muted)", background: "var(--surface)", padding: 12, borderRadius: "var(--radius-sm)", overflow: "auto", maxHeight: 200 }}>
                  {JSON.stringify(smokeResult.data, null, 2)}
                </pre>
              )}
            </div>
          ) : (
            <div style={{ fontSize: 13, color: "var(--text-muted)" }}>Run a smoke test to verify SignalHive is ready for dispatch.</div>
          )}
        </div>
      </div>

      {/* Endpoint Checks */}
      <div className="card">
        <div className="card-header">
          <h3>Endpoint Status</h3>
          <span className={`tag ${checks.every(c => c.status === "ok") ? "tag-green" : "tag-red"}`}>
            {checks.filter(c => c.status === "ok").length}/{checks.length} healthy
          </span>
        </div>
        <div className="card-body" style={{ padding: 0 }}>
          <div className="checks-grid">
            {checks.map((c, i) => (
              <div key={i} className="check-item" style={{ borderRadius: 0, borderLeft: "none", borderRight: "none", borderTop: i === 0 ? "none" : undefined, borderBottom: i === checks.length - 1 ? "none" : undefined }}>
                <span className={`check-status check-${c.status}`} />
                <div className="check-label">{c.label}</div>
                <div className="check-detail">{c.detail}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

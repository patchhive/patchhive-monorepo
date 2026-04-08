import { useEffect, useState } from "react";
import { API } from "../config.js";
import { Btn, EmptyState, S, timeAgo } from "@patchhivehq/ui";
import SignalCard from "../components/SignalCard.jsx";

const af = (key) => (url, opts = {}) =>
  fetch(url, {
    ...opts,
    headers: { ...(opts.headers || {}), ...(key ? { "X-API-Key": key } : {}) },
  });

export default function HistoryPanel({ apiKey }) {
  const [history, setHistory] = useState([]);
  const [selected, setSelected] = useState(null);
  const [loading, setLoading] = useState(false);
  const fetch_ = af(apiKey);

  const loadHistory = () =>
    fetch_(`${API}/history`)
      .then((res) => res.json())
      .then((data) => setHistory(data.scans || []))
      .catch(() => setHistory([]));

  const loadScan = (id) => {
    setLoading(true);
    fetch_(`${API}/history/${id}`)
      .then((res) => res.json())
      .then(setSelected)
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadHistory();
  }, [apiKey]);

  return (
    <div style={{ display: "grid", gridTemplateColumns: "340px 1fr", gap: 18 }}>
      <div style={{ ...S.panel, display: "flex", flexDirection: "column", gap: 10, minHeight: 500 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}>
          <div style={{ fontSize: 16, fontWeight: 700 }}>Scan History</div>
          <Btn onClick={loadHistory} style={{ padding: "5px 10px" }}>Refresh</Btn>
        </div>

        {history.length === 0 ? (
          <EmptyState icon="◎" text="Signal scans will show up here after you run them." />
        ) : (
          history.map((scan) => (
            <button
              key={scan.id}
              onClick={() => loadScan(scan.id)}
              style={{
                textAlign: "left",
                background: selected?.id === scan.id ? "var(--accent)10" : "var(--bg)",
                border: `1px solid ${selected?.id === scan.id ? "var(--accent)55" : "var(--border)"}`,
                borderRadius: 6,
                color: "var(--text)",
                cursor: "pointer",
                padding: "10px 12px",
                fontFamily: "inherit",
              }}
            >
              <div style={{ fontSize: 12, fontWeight: 700 }}>{scan.top_repo || "No top repo"}</div>
              <div style={{ color: "var(--text-dim)", fontSize: 11, marginTop: 4 }}>
                {scan.total_repos} repos • {scan.total_signals} signals • {timeAgo(scan.created_at)}
              </div>
              <div style={{ color: "var(--text-muted)", fontSize: 10, marginTop: 6 }}>
                {scan.search_query || scan.topics.join(", ") || scan.languages.join(", ")}
              </div>
            </button>
          ))
        )}
      </div>

      <div style={{ display: "grid", gap: 16 }}>
        {!selected && !loading && <EmptyState icon="📂" text="Choose a saved scan to review its ranked queue." />}
        {loading && <div style={{ ...S.panel, color: "var(--text-dim)" }}>Loading scan…</div>}

        {selected && !loading && (
          <>
            <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
              <div>
                <div style={{ fontSize: 18, fontWeight: 700 }}>Saved Scan</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                  {selected.summary.total_repos} repos • {selected.summary.total_signals} signals • {timeAgo(selected.created_at)}
                </div>
              </div>
              <div style={{ color: "var(--text-muted)", fontSize: 11 }}>Scan ID {selected.id}</div>
            </div>
            {selected.repos.map((repo) => <SignalCard key={repo.full_name} repo={repo} />)}
          </>
        )}
      </div>
    </div>
  );
}

import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, Tag } from "@patchhivehq/ui";

const statusColor = (status) =>
  status === "ready" ? "var(--green)" : status === "watch" ? "var(--gold)" : "var(--accent)";

export default function HistoryPanel({ apiKey }) {
  const [runs, setRuns] = useState([]);
  const [selected, setSelected] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  const refresh = () => {
    fetch_(`${API}/history`)
      .then((res) => res.json())
      .then(setRuns)
      .catch(() => setRuns([]));
  };

  useEffect(() => {
    refresh();
  }, [apiKey]);

  const loadDetail = (id) => {
    fetch_(`${API}/history/${id}`)
      .then((res) => res.json())
      .then(setSelected)
      .catch(() => setSelected(null));
  };

  return (
    <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: 18, alignItems: "start" }}>
      <div style={{ display: "grid", gap: 12 }}>
        <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 800 }}>Release evidence history</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12 }}>Saved ReleaseSentry readiness runs.</div>
          </div>
          <Btn onClick={refresh}>Refresh</Btn>
        </div>

        {runs.length === 0 ? (
          <EmptyState icon="◌" text="No release readiness runs have been saved yet." />
        ) : (
          runs.map((run) => (
            <button
              key={run.id}
              onClick={() => loadDetail(run.id)}
              style={{
                ...S.panel,
                textAlign: "left",
                cursor: "pointer",
                borderColor: selected?.id === run.id ? statusColor(run.decision) : "var(--border)",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", gap: 10 }}>
                <div style={{ fontSize: 13, fontWeight: 800 }}>{run.repo}</div>
                <Tag color={statusColor(run.decision)}>{run.decision}</Tag>
              </div>
              <div style={{ color: "var(--text-muted)", fontSize: 11, marginTop: 4 }}>
                {run.branch} · {run.target_tag || "next release"} · {run.score}/100
              </div>
              <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5, marginTop: 8 }}>
                {run.summary}
              </div>
              <div style={{ color: "var(--text-muted)", fontSize: 10, marginTop: 8 }}>{run.created_at}</div>
            </button>
          ))
        )}
      </div>

      {selected ? (
        <div style={{ ...S.panel, display: "grid", gap: 12 }}>
          <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap" }}>
            <div>
              <div style={S.label}>{selected.repo}</div>
              <div style={{ fontSize: 22, fontWeight: 900, color: statusColor(selected.decision) }}>
                {selected.decision?.toUpperCase()} · {selected.score}/100
              </div>
            </div>
            <Tag color={statusColor(selected.decision)}>{selected.metrics?.passed || 0} passed · {selected.metrics?.warned || 0} warned · {selected.metrics?.blocked || 0} blocked</Tag>
          </div>
          <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{selected.summary}</div>
          <div style={{ display: "grid", gap: 8 }}>
            {selected.checks?.map((check) => (
              <div key={check.key} style={{ background: "var(--bg)", border: "1px solid var(--border)", borderRadius: 6, padding: 12 }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 10 }}>
                  <div style={{ fontSize: 13, fontWeight: 800 }}>{check.label}</div>
                  <Tag color={check.status === "pass" ? "var(--green)" : check.status === "warn" ? "var(--gold)" : "var(--accent)"}>
                    {check.status}
                  </Tag>
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5, marginTop: 6 }}>{check.detail}</div>
              </div>
            ))}
          </div>
        </div>
      ) : (
        <EmptyState icon="↗" text="Select a saved run to inspect its check evidence." />
      )}
    </div>
  );
}

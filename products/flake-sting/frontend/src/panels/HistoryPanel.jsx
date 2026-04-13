import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag, timeAgo } from "@patchhivehq/ui";

function trendColor(status) {
  if (status === "rising") {
    return "var(--accent)";
  }
  if (status === "improving") {
    return "var(--green)";
  }
  if (status === "shifted") {
    return "var(--gold)";
  }
  return "var(--text-dim)";
}

function trendLabel(status) {
  if (status === "rising") {
    return "rising";
  }
  if (status === "improving") {
    return "improving";
  }
  if (status === "shifted") {
    return "shifted";
  }
  return "steady";
}

export default function HistoryPanel({ apiKey, onLoadScan, activeScanId }) {
  const [items, setItems] = useState([]);
  const [query, setQuery] = useState("");
  const fetch_ = createApiFetcher(apiKey);

  function refresh() {
    fetch_(`${API}/history`)
      .then((res) => res.json())
      .then(setItems)
      .catch(() => setItems([]));
  }

  useEffect(() => {
    refresh();
  }, [apiKey, activeScanId]);

  const filteredItems = items.filter((item) => {
    const needle = query.trim().toLowerCase();
    if (!needle) {
      return true;
    }
    return (
      item.repo.toLowerCase().includes(needle) ||
      item.summary.toLowerCase().includes(needle) ||
      (item.workflow_name || "").toLowerCase().includes(needle)
    );
  });

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Scan history</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
            Reload prior FlakeSting scans and compare which repos keep accumulating CI trust debt.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh</Btn>
      </div>

      <div style={{ ...S.panel, display: "grid", gap: 8 }}>
        <div style={S.label}>Filter history</div>
        <Input value={query} onChange={setQuery} placeholder="repo, workflow, summary..." />
      </div>

      {filteredItems.length === 0 ? (
        <EmptyState
          icon="◎"
          text={
            items.length === 0
              ? "FlakeSting history will show up here after the first scan."
              : "No saved scans match that filter yet."
          }
        />
      ) : (
        filteredItems.map((item) => (
          <div key={item.id} style={{ ...S.panel, display: "grid", gap: 12, borderColor: item.id === activeScanId ? "var(--accent)" : "var(--border)" }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
              <div style={{ display: "grid", gap: 6 }}>
                <div style={{ fontSize: 16, fontWeight: 700 }}>
                  {item.repo}
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{item.summary}</div>
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Tag color="var(--gold)">{item.flaky_signals} signals</Tag>
                <Tag color="var(--accent)">{item.quarantine_candidates} quarantine</Tag>
                {item.trend && <Tag color={trendColor(item.trend.status)}>{trendLabel(item.trend.status)}</Tag>}
                <Tag color="var(--text-dim)">{timeAgo(item.created_at)}</Tag>
              </div>
            </div>

            {item.trend && (
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Tag color={item.trend.flaky_signal_delta > 0 ? "var(--accent)" : item.trend.flaky_signal_delta < 0 ? "var(--green)" : "var(--text-dim)"}>
                  signals {item.trend.flaky_signal_delta > 0 ? "+" : ""}{item.trend.flaky_signal_delta}
                </Tag>
                <Tag color={item.trend.quarantine_delta > 0 ? "var(--accent)" : item.trend.quarantine_delta < 0 ? "var(--green)" : "var(--text-dim)"}>
                  quarantine {item.trend.quarantine_delta > 0 ? "+" : ""}{item.trend.quarantine_delta}
                </Tag>
                <Tag color={item.trend.rerun_delta > 0 ? "var(--gold)" : item.trend.rerun_delta < 0 ? "var(--green)" : "var(--text-dim)"}>
                  reruns {item.trend.rerun_delta > 0 ? "+" : ""}{item.trend.rerun_delta}
                </Tag>
              </div>
            )}

            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
              <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
                {item.branch ? `branch ${item.branch}` : "all branches"} · {item.workflow_name || "all workflows"}
              </div>
              <Btn onClick={() => onLoadScan(item.id)}>Load scan</Btn>
            </div>
          </div>
        ))
      )}
    </div>
  );
}

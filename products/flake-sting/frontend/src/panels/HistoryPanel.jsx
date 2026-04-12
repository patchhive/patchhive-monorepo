import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, Tag, timeAgo } from "@patchhivehq/ui";

export default function HistoryPanel({ apiKey, onLoadScan, activeScanId }) {
  const [items, setItems] = useState([]);
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

      {items.length === 0 ? (
        <EmptyState icon="◎" text="FlakeSting history will show up here after the first scan." />
      ) : (
        items.map((item) => (
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
                <Tag color="var(--text-dim)">{timeAgo(item.created_at)}</Tag>
              </div>
            </div>

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

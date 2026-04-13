import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag, timeAgo } from "@patchhivehq/ui";

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
      item.summary.toLowerCase().includes(needle)
    );
  });

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Scan history</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
            Reload earlier dependency triage calls and compare which repos keep accumulating update pressure.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh</Btn>
      </div>

      <div style={{ ...S.panel, display: "grid", gap: 8 }}>
        <div style={S.label}>Filter history</div>
        <Input value={query} onChange={setQuery} placeholder="repo or summary..." />
      </div>

      {filteredItems.length === 0 ? (
        <EmptyState
          icon="◎"
          text={
            items.length === 0
              ? "DepTriage history will show up here after the first scan."
              : "No saved scans match that filter yet."
          }
        />
      ) : (
        filteredItems.map((item) => (
          <div key={item.id} style={{ ...S.panel, display: "grid", gap: 12, borderColor: item.id === activeScanId ? "var(--accent)" : "var(--border)" }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
              <div style={{ display: "grid", gap: 6 }}>
                <div style={{ fontSize: 16, fontWeight: 700 }}>{item.repo}</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{item.summary}</div>
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Tag color="var(--accent)">{item.update_now} now</Tag>
                <Tag color="var(--gold)">{item.watch} watch</Tag>
                <Tag color="var(--text-dim)">{item.ignore_for_now} ignore</Tag>
                <Tag color="var(--blue)">{item.tracked_items} items</Tag>
                <Tag color="var(--text-dim)">{timeAgo(item.created_at)}</Tag>
              </div>
            </div>

            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
              <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
                Saved dependency queue snapshot
              </div>
              <Btn onClick={() => onLoadScan(item.id)}>Load scan</Btn>
            </div>
          </div>
        ))
      )}
    </div>
  );
}

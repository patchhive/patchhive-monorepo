import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, Tag, timeAgo } from "@patchhivehq/ui";

function statusColor(status) {
  if (status === "clear" || status === "resolved") {
    return "var(--green)";
  }
  if (status === "attention") {
    return "var(--accent)";
  }
  return "var(--gold)";
}

export default function HistoryPanel({ apiKey, onLoadReview, activeReviewId }) {
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
  }, [apiKey, activeReviewId]);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Review history</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
            Reload older PR checklists and compare where review churn tends to pile up.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh</Btn>
      </div>

      {items.length === 0 ? (
        <EmptyState icon="◎" text="ReviewBee history will show up here after the first PR review run." />
      ) : (
        items.map((item) => (
          <div key={item.id} style={{ ...S.panel, display: "grid", gap: 12, borderColor: item.id === activeReviewId ? "var(--accent)" : "var(--border)" }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
              <div style={{ display: "grid", gap: 6 }}>
                <div style={{ fontSize: 16, fontWeight: 700 }}>
                  {item.repo} · PR #{item.pr_number}
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{item.summary}</div>
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Tag color={statusColor(item.status)}>{item.status}</Tag>
                <Tag color="var(--accent)">{item.open_items} open</Tag>
                <Tag color="var(--green)">{item.resolved_items} resolved</Tag>
                <Tag color="var(--blue)">{item.reviewer_count} reviewers</Tag>
              </div>
            </div>

            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
              <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
                {item.pr_title} · {timeAgo(item.created_at)}
              </div>
              <Btn onClick={() => onLoadReview(item.id)}>Load review</Btn>
            </div>
          </div>
        ))
      )}
    </div>
  );
}

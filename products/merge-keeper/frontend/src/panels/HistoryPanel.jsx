import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, Tag, timeAgo } from "@patchhivehq/ui";

function readinessColor(readiness) {
  if (readiness === "ready") {
    return "var(--green)";
  }
  if (readiness === "blocked") {
    return "var(--accent)";
  }
  return "var(--gold)";
}

export default function HistoryPanel({ apiKey, onLoadAssessment, activeAssessmentId }) {
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
  }, [apiKey, activeAssessmentId]);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Run history</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
            Reload past readiness calls and compare where merge pressure tends to get stuck.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh</Btn>
      </div>

      {items.length === 0 ? (
        <EmptyState icon="◎" text="MergeKeeper history will show up here after the first PR readiness run." />
      ) : (
        items.map((item) => (
          <div key={item.id} style={{ ...S.panel, display: "grid", gap: 12, borderColor: item.id === activeAssessmentId ? "var(--accent)" : "var(--border)" }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
              <div style={{ display: "grid", gap: 6 }}>
                <div style={{ fontSize: 16, fontWeight: 700 }}>
                  {item.repo} · PR #{item.pr_number}
                </div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{item.summary}</div>
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                <Tag color={readinessColor(item.readiness)}>{item.readiness}</Tag>
                <Tag color="var(--accent)">{item.blockers_count} blockers</Tag>
                <Tag color="var(--gold)">{item.warnings_count} holds</Tag>
                <Tag color="var(--green)">{item.approvals_count} approvals</Tag>
              </div>
            </div>

            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
              <div style={{ color: "var(--text-dim)", fontSize: 11 }}>
                {item.pr_title} · failing {item.failing_checks_count} · pending {item.pending_checks_count} · {timeAgo(item.created_at)}
              </div>
              <Btn onClick={() => onLoadAssessment(item.id)}>Load run</Btn>
            </div>
          </div>
        ))
      )}
    </div>
  );
}

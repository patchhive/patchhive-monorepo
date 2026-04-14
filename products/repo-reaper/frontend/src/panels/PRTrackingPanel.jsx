import { useEffect, useState } from "react";
import { useApiFetcher } from "@patchhivehq/product-shell";
import { S, Btn, EmptyState } from "@patchhivehq/ui";
import { API } from "../config.js";

export function PRTrackingPanel({ apiKey = "" }) {
  const [prs, setPrs] = useState([]);
  const fetch_ = useApiFetcher(apiKey);

  const load = () => fetch_(`${API}/pr-tracking`).then(r => r.json()).then(d => setPrs(d.prs || []));

  useEffect(load, [fetch_]);

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", marginBottom: 12, gap: 8 }}>
        <span style={{ fontSize: 13, fontWeight: 700, color: "#d4d4e8" }}>↗ PR Monitor</span>
        <div style={{ flex: 1 }} />
        <Btn onClick={load} color="#484868" style={{ fontSize: 10 }}>↻ Refresh</Btn>
      </div>

      {prs.length === 0 ? (
        <EmptyState icon="↗" text="No PRs tracked yet." />
      ) : (
        <div style={{ ...S.panel }}>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "60px 1fr 100px 80px 80px",
              gap: 10,
              fontSize: 9,
              color: "#484868",
              fontWeight: 700,
              letterSpacing: "0.08em",
              padding: "0 0 8px",
              borderBottom: "1px solid #1c1c30",
              marginBottom: 8,
            }}
          >
            <div>PR</div>
            <div>REPO</div>
            <div>STATUS</div>
            <div>REVIEW</div>
            <div>MERGED</div>
          </div>
          {prs.map(pr => (
            <div
              key={`${pr.repo}-${pr.pr_number}`}
              style={{
                display: "grid",
                gridTemplateColumns: "60px 1fr 100px 80px 80px",
                gap: 10,
                fontSize: 11,
                padding: "5px 0",
                borderBottom: "1px solid #10101e",
                alignItems: "center",
              }}
            >
              <div style={{ color: "#2a8a4a" }}>#{pr.pr_number}</div>
              <div style={{ color: "#d4d4e8", fontSize: 10 }}>{pr.repo}</div>
              <div style={{ color: pr.state === "open" ? "#c8922a" : pr.state === "closed" ? "#484868" : "#d4d4e8", fontSize: 10 }}>
                {pr.state}
              </div>
              <div style={{ fontSize: 10, color: "#484868" }}>{pr.review_state || "—"}</div>
              <div style={{ fontSize: 10, color: pr.merged ? "#2a8a4a" : "#484868" }}>
                {pr.merged ? "✓ merged" : "—"}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

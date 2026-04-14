import { useEffect, useState } from "react";
import { useApiFetcher } from "@patchhivehq/product-shell";
import { EmptyState } from "@patchhivehq/ui";
import { API } from "../config.js";

const LEVEL_COLOR = { ok: "#2a8a4a", warn: "#c8922a", error: "#c41e3a" };
const LEVEL_ICON = { ok: "✓", warn: "⚠", error: "✗" };

export function StartupChecksPanel({ apiKey = "" }) {
  const [checks, setChecks] = useState([]);
  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => {
    fetch_(`${API}/startup/checks`)
      .then(r => r.json())
      .then(d => setChecks(d.checks || []));
  }, [fetch_]);

  const colorFor = level => LEVEL_COLOR[level] || "#484868";
  const iconFor = level => LEVEL_ICON[level] || "◌";

  return (
    <div>
      <div style={{ fontSize: 13, fontWeight: 700, color: "#d4d4e8", marginBottom: 16 }}>◎ Startup Checks</div>
      {checks.length === 0 ? (
        <EmptyState icon="◌" text="No checks loaded." />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {checks.map((check, index) => (
            <div
              key={index}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 10,
                padding: "8px 12px",
                background: "#0d0d18",
                border: `1px solid ${colorFor(check.level)}33`,
                borderRadius: 5,
              }}
            >
              <span style={{ color: colorFor(check.level), fontSize: 12 }}>{iconFor(check.level)}</span>
              <span style={{ fontSize: 11, color: "#d4d4e8" }}>{check.msg}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

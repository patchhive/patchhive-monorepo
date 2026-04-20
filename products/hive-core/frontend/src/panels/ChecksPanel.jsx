import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, Tag } from "@patchhivehq/ui";

function levelColor(level) {
  if (level === "error") return "var(--accent)";
  if (level === "warn") return "var(--gold)";
  if (level === "ok") return "var(--green)";
  return "var(--blue)";
}

export default function ChecksPanel({ apiKey }) {
  const [health, setHealth] = useState(null);
  const [checks, setChecks] = useState([]);
  const fetch_ = createApiFetcher(apiKey);

  const refresh = () => {
    fetch_(`${API}/health`)
      .then((res) => res.json())
      .then(setHealth)
      .catch(() => setHealth(null));
    fetch_(`${API}/startup/checks`)
      .then((res) => res.json())
      .then((data) => setChecks(data.checks || []))
      .catch(() => setChecks([]));
  };

  useEffect(() => {
    refresh();
  }, [apiKey]);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 800 }}>Checks</div>
          <div style={{ fontSize: 12, color: "var(--text-dim)" }}>
            HiveCore startup validation plus control-plane health details.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh</Btn>
      </div>

      {health && (
        <div style={{ display: "grid", gap: 12, gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))" }}>
          <div style={{ ...S.panel, display: "grid", gap: 4 }}>
            <div style={S.label}>Status</div>
            <div style={{ fontSize: 22, fontWeight: 800, color: health.status === "ok" ? "var(--green)" : "var(--gold)" }}>
              {health.status}
            </div>
          </div>
          <div style={{ ...S.panel, display: "grid", gap: 4 }}>
            <div style={S.label}>Overrides</div>
            <div style={{ fontSize: 22, fontWeight: 800 }}>{health.product_override_count}</div>
          </div>
          <div style={{ ...S.panel, display: "grid", gap: 4 }}>
            <div style={S.label}>Auth</div>
            <div style={{ fontSize: 22, fontWeight: 800 }}>{health.auth_enabled ? "on" : "off"}</div>
          </div>
          <div style={{ ...S.panel, display: "grid", gap: 4 }}>
            <div style={S.label}>Database</div>
            <div style={{ fontSize: 22, fontWeight: 800, color: health.db_ok ? "var(--green)" : "var(--accent)" }}>
              {health.db_ok ? "ok" : "degraded"}
            </div>
          </div>
        </div>
      )}

      {checks.length === 0 ? (
        <EmptyState icon="⬢" text="HiveCore did not return any startup checks yet." />
      ) : (
        <div style={{ display: "grid", gap: 10 }}>
          {checks.map((check, index) => (
            <div
              key={`${check.msg}-${index}`}
              style={{
                ...S.panel,
                display: "flex",
                justifyContent: "space-between",
                gap: 12,
                alignItems: "flex-start",
              }}
            >
              <div style={{ fontSize: 13, lineHeight: 1.6, color: "var(--text)" }}>{check.msg}</div>
              <Tag color={levelColor(check.level)}>{check.level}</Tag>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Tag } from "@patchhivehq/ui";
import {
  CommandHero,
  CommandPanel,
  MetricTile,
  SectionHeader,
  commandGridStyle,
  commandPanelStyle,
} from "../components/CommandChrome.jsx";

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
    <div style={{ ...commandGridStyle, gap: 14 }}>
      <CommandHero
        kicker="Diagnostics station"
        title="Control-plane checks"
        body="HiveCore startup validation plus control-plane health details."
        tone="var(--blue)"
        actions={<Btn onClick={refresh}>Refresh</Btn>}
      />

      {health && (
        <div style={{ display: "grid", gap: 12, gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))" }}>
          <MetricTile label="Status" value={health.status} tone={health.status === "ok" ? "var(--green)" : "var(--gold)"} />
          <MetricTile label="Overrides" value={health.product_override_count} tone="var(--blue)" />
          <MetricTile label="Auth" value={health.auth_enabled ? "on" : "off"} tone={health.auth_enabled ? "var(--green)" : "var(--gold)"} />
          <MetricTile label="Database" value={health.db_ok ? "ok" : "degraded"} tone={health.db_ok ? "var(--green)" : "var(--accent)"} />
        </div>
      )}

      {checks.length === 0 ? (
        <EmptyState icon="⬢" text="HiveCore did not return any startup checks yet." />
      ) : (
        <CommandPanel tone="var(--accent)" style={{ display: "grid", gap: 10 }}>
          <SectionHeader
            kicker="Startup sequence"
            title="Boot validation log"
            body="Checks are emitted by HiveCore startup helpers and should stay boring before exposing the suite beyond local development."
          />
          {checks.map((check, index) => (
            <div
              key={`${check.msg}-${index}`}
              style={commandPanelStyle(levelColor(check.level), {
                display: "flex",
                justifyContent: "space-between",
                gap: 12,
                alignItems: "flex-start",
                padding: "10px 12px",
              })}
            >
              <div style={{ fontSize: 13, lineHeight: 1.6, color: "var(--text)" }}>{check.msg}</div>
              <Tag color={levelColor(check.level)}>{check.level}</Tag>
            </div>
          ))}
        </CommandPanel>
      )}
    </div>
  );
}

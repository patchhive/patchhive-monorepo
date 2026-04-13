import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, Tag } from "@patchhivehq/ui";

export default function ChecksPanel({ apiKey }) {
  const [health, setHealth] = useState(null);
  const [checks, setChecks] = useState([]);
  const fetch_ = createApiFetcher(apiKey);

  const refresh = () => {
    fetch_(`${API}/health`).then((res) => res.json()).then(setHealth).catch(() => setHealth(null));
    fetch_(`${API}/startup/checks`).then((res) => res.json()).then((data) => setChecks(data.checks || [])).catch(() => setChecks([]));
  };

  useEffect(() => {
    refresh();
  }, [apiKey]);

  return (
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Startup Checks</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
            VulnTriage needs healthy GitHub security reads and a writable local DB before the queue means much.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh</Btn>
      </div>

      {health && (
        <div style={{ ...S.panel, display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
          <Stat label="Status" value={health.status} color={health.status === "ok" ? "var(--green)" : "var(--accent)"} />
          <Stat label="Version" value={health.version} />
          <Stat label="Auth Enabled" value={health.auth_enabled ? "yes" : "no"} />
          <Stat label="GitHub Ready" value={health.github_ready ? "yes" : "no"} color={health.github_ready ? "var(--green)" : "var(--gold)"} />
          <Stat label="Stored scans" value={health.scan_count} />
          <Stat label="Repos seen" value={health.repo_count} />
          <Stat label="Tracked findings" value={health.tracked_finding_count} />
          <Stat label="Fix now" value={health.fix_now_count} color="var(--accent)" />
          <Stat label="Plan next" value={health.plan_next_count} color="var(--gold)" />
          <Stat label="Watch" value={health.watch_count} color="var(--text-dim)" />
          <div>
            <div style={S.label}>Mode</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>{health.mode}</div>
          </div>
          <div>
            <div style={S.label}>DB Path</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)", lineHeight: 1.5 }}>{health.db_path}</div>
          </div>
        </div>
      )}

      {checks.length === 0 ? (
        <EmptyState icon="◌" text="No startup checks were returned." />
      ) : (
        checks.map((check, index) => (
          <div key={`${check.msg}-${index}`} style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" }}>
            <div style={{ color: "var(--text)", fontSize: 13, lineHeight: 1.5 }}>{check.msg}</div>
            <Tag
              color={
                check.level === "error"
                  ? "var(--accent)"
                  : check.level === "warn"
                    ? "var(--gold)"
                    : "var(--green)"
              }
            >
              {check.level}
            </Tag>
          </div>
        ))
      )}
    </div>
  );
}

function Stat({ label, value, color }) {
  return (
    <div>
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 18, fontWeight: 700, color: color || "var(--text)" }}>{value}</div>
    </div>
  );
}

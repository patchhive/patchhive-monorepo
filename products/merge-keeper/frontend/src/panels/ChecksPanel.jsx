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
            MergeKeeper needs GitHub read access and a healthy local DB before its readiness calls mean much. ReviewBee, TrustGate, and RepoMemory stay optional.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh</Btn>
      </div>

      {health && (
        <div style={{ display: "grid", gap: 12 }}>
          <div style={{ ...S.panel, display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
            <Stat label="Status" value={health.status} color={health.status === "ok" ? "var(--green)" : "var(--accent)"} />
            <Stat label="Version" value={health.version} />
            <Stat label="Auth Enabled" value={health.auth_enabled ? "yes" : "no"} />
            <Stat label="GitHub Ready" value={health.github_ready ? "yes" : "no"} color={health.github_ready ? "var(--green)" : "var(--accent)"} />
            <Stat label="Stored Runs" value={health.assessment_count} />
            <Stat label="Repos Seen" value={health.repo_count} />
            <Stat label="Ready Calls" value={health.ready_count} color="var(--green)" />
            <Stat label="Hold Calls" value={health.hold_count} color="var(--gold)" />
            <Stat label="Blocked Calls" value={health.blocked_count} color="var(--accent)" />
            <div>
              <div style={S.label}>Mode</div>
              <div style={{ fontSize: 12, color: "var(--text-dim)" }}>{health.mode}</div>
            </div>
            <div>
              <div style={S.label}>DB Path</div>
              <div style={{ fontSize: 12, color: "var(--text-dim)", lineHeight: 1.5 }}>{health.db_path}</div>
            </div>
          </div>

          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ fontSize: 15, fontWeight: 700 }}>Cross-product integrations</div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <Tag color={health.integrations?.review_bee_configured ? "var(--green)" : "var(--text-dim)"}>
                ReviewBee {health.integrations?.review_bee_configured ? "linked" : "off"}
              </Tag>
              <Tag color={health.integrations?.trust_gate_configured ? "var(--green)" : "var(--text-dim)"}>
                TrustGate {health.integrations?.trust_gate_configured ? "linked" : "off"}
              </Tag>
              <Tag color={health.integrations?.repo_memory_configured ? "var(--green)" : "var(--text-dim)"}>
                RepoMemory {health.integrations?.repo_memory_configured ? "linked" : "off"}
              </Tag>
            </div>
            <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
              When these are configured, MergeKeeper can fold review churn, diff risk, and repo-specific expectations into the final readiness call. When they are off, the product still works on GitHub signals alone.
            </div>
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

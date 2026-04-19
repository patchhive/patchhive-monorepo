import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, S, Tag } from "@patchhivehq/ui";

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
    <div style={{ display: "grid", gap: 18 }}>
      <div style={{ ...S.panel, display: "flex", justifyContent: "space-between", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 700 }}>Startup checks</div>
          <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
            RefactorScout needs a healthy local DB and a sane filesystem allowlist before its queue is worth trusting.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh</Btn>
      </div>

      {health && (
        <div style={{ ...S.panel, display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
          <Stat label="Status" value={health.status} color={health.status === "ok" ? "var(--green)" : "var(--accent)"} />
          <Stat label="Version" value={health.version} />
          <Stat label="Auth enabled" value={health.auth_enabled ? "yes" : "no"} />
          <Stat label="Stored scans" value={health.scan_count} />
          <Stat label="Repos seen" value={health.repo_count} />
          <Stat label="Opportunities" value={health.opportunity_count} />
          <Stat
            label="High-safety leads"
            value={health.high_safety_count}
            color="var(--green)"
          />
          <Stat label="Allowed roots" value={health.allowed_roots?.length || 0} />
          <Stat
            label="Remote FS"
            value={health.remote_fs_enabled ? "enabled" : "local only"}
            color={health.remote_fs_enabled ? "var(--gold)" : "var(--green)"}
          />
          <div>
            <div style={S.label}>Mode</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>{health.mode}</div>
          </div>
          <div>
            <div style={S.label}>DB path</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)", lineHeight: 1.5 }}>
              {health.db_path}
            </div>
          </div>
          <div style={{ gridColumn: "1 / -1" }}>
            <div style={S.label}>Allowed roots</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)", lineHeight: 1.6 }}>
              {health.allowed_roots?.length
                ? health.allowed_roots.join(" · ")
                : "No readable roots configured."}
            </div>
          </div>
        </div>
      )}

      {checks.length === 0 ? (
        <EmptyState icon="◌" text="No startup checks were returned." />
      ) : (
        checks.map((check, index) => (
          <div
            key={`${check.msg}-${index}`}
            style={{ ...S.panel, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" }}
          >
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

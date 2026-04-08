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
            SignalHive is intentionally read-only. These checks focus on GitHub access and local storage readiness.
          </div>
        </div>
        <Btn onClick={refresh}>Refresh</Btn>
      </div>

      {health && (
        <div style={{ ...S.panel, display: "flex", gap: 18, flexWrap: "wrap" }}>
          <div>
            <div style={S.label}>Status</div>
            <div style={{ fontSize: 18, fontWeight: 700, color: health.status === "ok" ? "var(--green)" : "var(--accent)" }}>
              {health.status}
            </div>
          </div>
          <div>
            <div style={S.label}>Version</div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{health.version}</div>
          </div>
          <div>
            <div style={S.label}>Saved Scans</div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{health.scan_count}</div>
          </div>
          <div>
            <div style={S.label}>DB Path</div>
            <div style={{ fontSize: 12, color: "var(--text-dim)" }}>{health.db_path}</div>
          </div>
          <div>
            <div style={S.label}>Allowlist</div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{health.repo_lists?.allowlist || 0}</div>
          </div>
          <div>
            <div style={S.label}>Denylist</div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{health.repo_lists?.denylist || 0}</div>
          </div>
          <div>
            <div style={S.label}>Opt-Out</div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{health.repo_lists?.opt_out || 0}</div>
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

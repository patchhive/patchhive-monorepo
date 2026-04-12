import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag, timeAgo } from "@patchhivehq/ui";

function statusColor(status) {
  if (status === "quarantine") {
    return "var(--accent)";
  }
  if (status === "suspect") {
    return "var(--gold)";
  }
  return "var(--blue)";
}

export default function ScanPanel({
  apiKey,
  form,
  setForm,
  running,
  onRun,
  scan,
  onLoadScan,
}) {
  const [overview, setOverview] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    fetch_(`${API}/overview`)
      .then((res) => res.json())
      .then(setOverview)
      .catch(() => setOverview(null));
  }, [apiKey, scan?.id]);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>Scan workflow history</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
              FlakeSting reads recent GitHub Actions runs, looks for fail/pass swings in test jobs and steps, and ranks the flakiest signals first.
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <Tag color="var(--gold)">suspect</Tag>
            <Tag color="var(--accent)">quarantine</Tag>
            <Tag color="var(--blue)">GitHub Actions</Tag>
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "minmax(260px, 2fr) minmax(160px, 1fr)", gap: 12 }}>
          <div>
            <div style={S.label}>Repository</div>
            <Input value={form.repo} onChange={(value) => setForm((prev) => ({ ...prev, repo: value }))} placeholder="owner/repo" />
          </div>
          <div>
            <div style={S.label}>Branch</div>
            <Input value={form.branch} onChange={(value) => setForm((prev) => ({ ...prev, branch: value }))} placeholder="main (optional)" />
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "minmax(240px, 2fr) minmax(120px, 1fr) auto", gap: 12, alignItems: "end" }}>
          <div>
            <div style={S.label}>Workflow filter</div>
            <Input value={form.workflow_name} onChange={(value) => setForm((prev) => ({ ...prev, workflow_name: value }))} placeholder="CI, test, integration (optional)" />
          </div>
          <div>
            <div style={S.label}>Lookback runs</div>
            <Input value={form.lookback_runs} onChange={(value) => setForm((prev) => ({ ...prev, lookback_runs: value }))} placeholder="25" />
          </div>
          <Btn onClick={onRun} disabled={running}>
            {running ? "Scanning..." : "Run FlakeSting"}
          </Btn>
        </div>
      </div>

      {scan ? (
        <div style={{ display: "grid", gap: 16 }}>
          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
              <div style={{ display: "grid", gap: 8 }}>
                <div style={{ fontSize: 18, fontWeight: 700 }}>{scan.repo}</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{scan.summary}</div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <Tag color="var(--blue)">{scan.repo}</Tag>
                  {scan.branch && <Tag color="var(--text-dim)">branch: {scan.branch}</Tag>}
                  {scan.workflow_name && <Tag color="var(--text-dim)">workflow: {scan.workflow_name}</Tag>}
                  <Tag color="var(--text-dim)">{timeAgo(scan.created_at)}</Tag>
                </div>
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 10 }}>
              <MetricCard label="Workflow runs" value={scan.metrics.workflow_runs} />
              <MetricCard label="Failed runs" value={scan.metrics.failed_runs} color="var(--accent)" />
              <MetricCard label="Reruns" value={scan.metrics.rerun_like_runs} color="var(--gold)" />
              <MetricCard label="Signals" value={scan.metrics.flaky_signals} color="var(--gold)" />
              <MetricCard label="Quarantine candidates" value={scan.metrics.quarantine_candidates} color="var(--accent)" />
            </div>
          </div>

          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ fontSize: 15, fontWeight: 700 }}>Likely flaky signals</div>
            {!scan.signals?.length ? (
              <EmptyState icon="✓" text="No fail/pass swings were found in the scanned workflow history." />
            ) : (
              scan.signals.map((signal) => (
                <div key={signal.key} style={{ borderTop: "1px solid var(--border)", paddingTop: 10, display: "grid", gap: 8 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
                    <div style={{ display: "grid", gap: 4 }}>
                      <div style={{ fontWeight: 700 }}>
                        {signal.step_name || signal.job_name}
                      </div>
                      <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
                        {signal.summary}
                      </div>
                    </div>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                      <Tag color={statusColor(signal.status)}>{signal.status}</Tag>
                      <Tag color="var(--text-dim)">score {signal.score}</Tag>
                    </div>
                  </div>

                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    <Tag color="var(--blue)">{signal.workflow_name}</Tag>
                    <Tag color="var(--text-dim)">{signal.job_name}</Tag>
                    <Tag color="var(--accent)">{signal.failure_count} fail</Tag>
                    <Tag color="var(--green)">{signal.success_count} pass</Tag>
                    {signal.rerun_hits > 0 && <Tag color="var(--gold)">{signal.rerun_hits} rerun hit{signal.rerun_hits === 1 ? "" : "s"}</Tag>}
                  </div>

                  {signal.environment_hints?.length > 0 && (
                    <div style={{ display: "grid", gap: 6 }}>
                      {signal.environment_hints.map((hint, index) => (
                        <div key={`${signal.key}-hint-${index}`} style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>
                          - {hint}
                        </div>
                      ))}
                    </div>
                  )}

                  {signal.evidence?.length > 0 && (
                    <div style={{ display: "grid", gap: 6 }}>
                      {signal.evidence.map((line, index) => (
                        <div key={`${signal.key}-evidence-${index}`} style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>
                          - {line}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        </div>
      ) : overview ? (
        <div style={{ display: "grid", gap: 16 }}>
          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{overview.product}</div>
            <div style={{ color: "var(--accent)", fontSize: 12 }}>{overview.tagline}</div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 10 }}>
              <MetricCard label="Stored scans" value={overview.counts.scans} />
              <MetricCard label="Repos seen" value={overview.counts.repos} />
              <MetricCard label="Flaky signals" value={overview.counts.flaky_signals} color="var(--gold)" />
              <MetricCard label="Quarantine candidates" value={overview.counts.quarantine_candidates} color="var(--accent)" />
            </div>
          </div>

          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ fontSize: 15, fontWeight: 700 }}>Recent FlakeSting scans</div>
            {overview.recent_scans?.length ? (
              overview.recent_scans.map((item) => (
                <div key={item.id} style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center", borderTop: "1px solid var(--border)", paddingTop: 10 }}>
                  <div style={{ display: "grid", gap: 4 }}>
                    <div style={{ fontWeight: 700 }}>{item.repo}</div>
                    <div style={{ color: "var(--text-dim)", fontSize: 12 }}>{item.summary}</div>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                      <Tag color="var(--gold)">{item.flaky_signals} signals</Tag>
                      <Tag color="var(--accent)">{item.quarantine_candidates} quarantine</Tag>
                      <Tag color="var(--text-dim)">{timeAgo(item.created_at)}</Tag>
                    </div>
                  </div>
                  <Btn onClick={() => onLoadScan(item.id)}>Load</Btn>
                </div>
              ))
            ) : (
              <EmptyState icon="🦂" text="Run FlakeSting once and your scan history will start showing up here." />
            )}
          </div>
        </div>
      ) : (
        <EmptyState icon="…" text="FlakeSting overview is loading." />
      )}
    </div>
  );
}

function MetricCard({ label, value, color }) {
  return (
    <div style={{ ...S.panel, padding: 12, display: "grid", gap: 4 }}>
      <div style={S.label}>{label}</div>
      <div style={{ fontSize: 22, fontWeight: 700, color: color || "var(--text)" }}>{value}</div>
    </div>
  );
}

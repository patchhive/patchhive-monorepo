import { useEffect, useState } from "react";
import { createApiFetcher } from "@patchhivehq/product-shell";
import { API } from "../config.js";
import { Btn, EmptyState, Input, S, Tag, timeAgo } from "@patchhivehq/ui";

function readinessColor(readiness) {
  if (readiness === "ready") {
    return "var(--green)";
  }
  if (readiness === "blocked") {
    return "var(--accent)";
  }
  return "var(--gold)";
}

function signalColor(severity) {
  if (severity === "block") {
    return "var(--accent)";
  }
  if (severity === "warn") {
    return "var(--gold)";
  }
  return "var(--blue)";
}

export default function KeeperPanel({
  apiKey,
  form,
  setForm,
  running,
  onRun,
  assessment,
  onLoadAssessment,
}) {
  const [overview, setOverview] = useState(null);
  const fetch_ = createApiFetcher(apiKey);

  useEffect(() => {
    fetch_(`${API}/overview`)
      .then((res) => res.json())
      .then(setOverview)
      .catch(() => setOverview(null));
  }, [apiKey, assessment?.id]);

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <div style={{ ...S.panel, display: "grid", gap: 14 }}>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>Assess PR readiness</div>
            <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
              MergeKeeper reads GitHub merge pressure and gives you a simple answer: ready, hold, or blocked.
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <Tag color="var(--green)">ready</Tag>
            <Tag color="var(--gold)">hold</Tag>
            <Tag color="var(--accent)">blocked</Tag>
          </div>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "minmax(260px, 2fr) minmax(140px, 1fr) auto", gap: 12, alignItems: "end" }}>
          <div>
            <div style={S.label}>Repository</div>
            <Input value={form.repo} onChange={(value) => setForm((prev) => ({ ...prev, repo: value }))} placeholder="owner/repo" />
          </div>
          <div>
            <div style={S.label}>PR Number</div>
            <Input value={form.pr_number} onChange={(value) => setForm((prev) => ({ ...prev, pr_number: value }))} placeholder="123" />
          </div>
          <Btn onClick={onRun} disabled={running}>
            {running ? "Reading GitHub..." : "Run MergeKeeper"}
          </Btn>
        </div>
      </div>

      {assessment ? (
        <div style={{ display: "grid", gap: 16 }}>
          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
              <div style={{ display: "grid", gap: 8 }}>
                <div style={{ fontSize: 18, fontWeight: 700 }}>{assessment.pr_title}</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{assessment.summary}</div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <Tag color="var(--blue)">{assessment.repo}</Tag>
                  <Tag color="var(--blue)">PR #{assessment.pr_number}</Tag>
                  <Tag color={readinessColor(assessment.readiness)}>{assessment.readiness}</Tag>
                  <Tag color="var(--text-dim)">{timeAgo(assessment.created_at)}</Tag>
                </div>
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                {assessment.pr_url && (
                  <Btn onClick={() => window.open(assessment.pr_url, "_blank", "noreferrer")} style={{ padding: "6px 10px" }}>
                    Open PR
                  </Btn>
                )}
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 10 }}>
              <MetricCard label="Approvals" value={assessment.metrics.approvals} color="var(--green)" />
              <MetricCard label="Requested changes" value={assessment.metrics.changes_requested} color="var(--accent)" />
              <MetricCard label="Failing checks" value={assessment.metrics.failing_checks} color="var(--accent)" />
              <MetricCard label="Pending checks" value={assessment.metrics.pending_checks} color="var(--gold)" />
              <MetricCard label="Open review threads" value={assessment.metrics.actionable_open_threads} color="var(--gold)" />
              <MetricCard label="Changed files" value={assessment.metrics.changed_files} color="var(--blue)" />
            </div>

            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <Tag color="var(--text-dim)">mergeable: {assessment.mergeable}</Tag>
              <Tag color="var(--text-dim)">state: {assessment.mergeable_state}</Tag>
              {assessment.base_ref && <Tag color="var(--blue)">base: {assessment.base_ref}</Tag>}
              {assessment.head_ref && <Tag color="var(--blue)">head: {assessment.head_ref}</Tag>}
              <Tag color="var(--text-dim)">+{assessment.metrics.additions} / -{assessment.metrics.deletions}</Tag>
            </div>
          </div>

          {assessment.reviewer_states?.length > 0 && (
            <div style={{ ...S.panel, display: "grid", gap: 10 }}>
              <div style={{ fontSize: 15, fontWeight: 700 }}>Latest reviewer states</div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                {assessment.reviewer_states.map((reviewer) => (
                  <Tag key={`${reviewer.login}-${reviewer.state}`} color={reviewer.state === "APPROVED" ? "var(--green)" : reviewer.state === "CHANGES_REQUESTED" ? "var(--accent)" : "var(--gold)"}>
                    @{reviewer.login}: {reviewer.state.toLowerCase()}
                  </Tag>
                ))}
              </div>
            </div>
          )}

          <SignalSection title="Blockers" emptyText="No hard blockers right now." items={assessment.blockers} />
          <SignalSection title="Holds" emptyText="No hold-level warnings right now." items={assessment.warnings} />
        </div>
      ) : overview ? (
        <div style={{ display: "grid", gap: 16 }}>
          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ fontSize: 18, fontWeight: 700 }}>{overview.product}</div>
            <div style={{ color: "var(--accent)", fontSize: 12 }}>{overview.tagline}</div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))", gap: 10 }}>
              <MetricCard label="Stored runs" value={overview.counts.runs} />
              <MetricCard label="Repos seen" value={overview.counts.repos} />
              <MetricCard label="Ready calls" value={overview.counts.ready_runs} color="var(--green)" />
              <MetricCard label="Hold calls" value={overview.counts.hold_runs} color="var(--gold)" />
              <MetricCard label="Blocked calls" value={overview.counts.blocked_runs} color="var(--accent)" />
            </div>
          </div>

          <div style={{ ...S.panel, display: "grid", gap: 12 }}>
            <div style={{ fontSize: 15, fontWeight: 700 }}>Recent MergeKeeper runs</div>
            {overview.recent_runs?.length ? (
              overview.recent_runs.map((item) => (
                <div key={item.id} style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "center", borderTop: "1px solid var(--border)", paddingTop: 10 }}>
                  <div style={{ display: "grid", gap: 4 }}>
                    <div style={{ fontWeight: 700 }}>{item.repo} · PR #{item.pr_number}</div>
                    <div style={{ color: "var(--text-dim)", fontSize: 12 }}>{item.summary}</div>
                    <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                      <Tag color={readinessColor(item.readiness)}>{item.readiness}</Tag>
                      <Tag color="var(--accent)">{item.blockers_count} blockers</Tag>
                      <Tag color="var(--gold)">{item.warnings_count} holds</Tag>
                      <Tag color="var(--text-dim)">{timeAgo(item.created_at)}</Tag>
                    </div>
                  </div>
                  <Btn onClick={() => onLoadAssessment(item.id)}>Load</Btn>
                </div>
              ))
            ) : (
              <EmptyState icon="🪢" text="Run MergeKeeper on a PR and your recent readiness calls will show up here." />
            )}
          </div>
        </div>
      ) : (
        <EmptyState icon="…" text="MergeKeeper overview is loading." />
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

function SignalSection({ title, emptyText, items }) {
  return (
    <div style={{ ...S.panel, display: "grid", gap: 12 }}>
      <div style={{ fontSize: 15, fontWeight: 700 }}>{title}</div>
      {!items?.length ? (
        <EmptyState icon="✓" text={emptyText} />
      ) : (
        items.map((item) => (
          <div key={item.key} style={{ borderTop: "1px solid var(--border)", paddingTop: 10, display: "grid", gap: 8 }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
              <div style={{ display: "grid", gap: 4 }}>
                <div style={{ fontWeight: 700 }}>{item.label}</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{item.detail}</div>
              </div>
              <Tag color={signalColor(item.severity)}>{item.severity}</Tag>
            </div>
            {item.evidence?.length > 0 && (
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                {item.evidence.map((line, index) => (
                  <Tag key={`${item.key}-${index}`} color="var(--text-dim)">{line}</Tag>
                ))}
              </div>
            )}
          </div>
        ))
      )}
    </div>
  );
}

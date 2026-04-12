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

function statusTone(value) {
  if (value === "blocked" || value === "block" || value === "attention") {
    return "var(--accent)";
  }
  if (value === "hold" || value === "warn" || value === "mixed") {
    return "var(--gold)";
  }
  if (value === "ready" || value === "safe" || value === "clear") {
    return "var(--green)";
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

        <label style={{ display: "flex", gap: 10, alignItems: "start", color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>
          <input
            type="checkbox"
            checked={!!form.publish_report}
            onChange={(event) => setForm((prev) => ({ ...prev, publish_report: event.target.checked }))}
            style={{ marginTop: 2 }}
          />
          <span>
            Maintain a MergeKeeper PR artifact when this run completes.
            <span style={{ display: "block", fontSize: 11, color: "var(--text-dim)" }}>
              Leave this on to upsert the PR comment and publish a check-style readiness signal. Turn it off for a local-only pass.
            </span>
          </span>
        </label>
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
              {assessment.github?.trigger && <Tag color="var(--blue)">trigger: {assessment.github.trigger}</Tag>}
              {assessment.github?.event && <Tag color="var(--text-dim)">event: {assessment.github.event}</Tag>}
              {assessment.github?.action && <Tag color="var(--text-dim)">action: {assessment.github.action}</Tag>}
            </div>
          </div>

          {assessment.github_report && (
            <div style={{ ...S.panel, display: "grid", gap: 12 }}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: 12, flexWrap: "wrap", alignItems: "start" }}>
                <div style={{ display: "grid", gap: 6 }}>
                  <div style={{ fontSize: 15, fontWeight: 700 }}>GitHub artifact</div>
                  <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>
                    {assessment.github_report.message}
                  </div>
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <Tag
                    color={
                      assessment.github_report.delivered
                        ? "var(--green)"
                        : assessment.github_report.state === "report_failed" || assessment.github_report.state === "missing_token"
                          ? "var(--accent)"
                          : "var(--gold)"
                    }
                  >
                    {assessment.github_report.state}
                  </Tag>
                  {assessment.github_report.comment_mode && (
                    <Tag color="var(--blue)">{assessment.github_report.comment_mode}</Tag>
                  )}
                </div>
              </div>

              <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                {assessment.github_report.comment_url && (
                  <Btn onClick={() => window.open(assessment.github_report.comment_url, "_blank", "noreferrer")} style={{ padding: "6px 10px" }}>
                    Open comment
                  </Btn>
                )}
                {assessment.github_report.check_url && (
                  <Btn onClick={() => window.open(assessment.github_report.check_url, "_blank", "noreferrer")} style={{ padding: "6px 10px" }}>
                    Open check
                  </Btn>
                )}
                {assessment.github_report.report_markdown && navigator?.clipboard && (
                  <Btn onClick={() => navigator.clipboard.writeText(assessment.github_report.report_markdown)} style={{ padding: "6px 10px" }}>
                    Copy report
                  </Btn>
                )}
              </div>

              {assessment.github_report.details?.length > 0 && (
                <div style={{ display: "grid", gap: 6 }}>
                  {assessment.github_report.details.map((line, index) => (
                    <div key={`gh-detail-${index}`} style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>
                      - {line}
                    </div>
                  ))}
                </div>
              )}

              {assessment.github_report.report_markdown && (
                <div style={{ border: "1px solid var(--border)", borderRadius: 6, padding: 12, background: "var(--bg-input)", color: "var(--text-dim)", lineHeight: 1.6, whiteSpace: "pre-wrap" }}>
                  {assessment.github_report.report_markdown}
                </div>
              )}
            </div>
          )}

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

          {(assessment.review_bee || assessment.trust_gate || assessment.repo_memory) && (
            <div style={{ ...S.panel, display: "grid", gap: 14 }}>
              <div style={{ display: "grid", gap: 4 }}>
                <div style={{ fontSize: 15, fontWeight: 700 }}>Cross-product context</div>
                <div style={{ color: "var(--text-dim)", fontSize: 12 }}>
                  MergeKeeper can optionally layer sibling PatchHive products into the merge call so review churn, policy risk, and repo memory all show up in one place.
                </div>
              </div>

              <div style={{ display: "grid", gap: 12, gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))" }}>
                {assessment.review_bee && (
                  <ContextCard
                    title="ReviewBee"
                    summary={assessment.review_bee.summary}
                    badge={assessment.review_bee.status || "linked"}
                    badgeColor={statusTone(assessment.review_bee.status)}
                    stats={[
                      `${assessment.review_bee.open_items} open item${assessment.review_bee.open_items === 1 ? "" : "s"}`,
                      `${assessment.review_bee.actionable_threads} actionable thread${assessment.review_bee.actionable_threads === 1 ? "" : "s"}`,
                    ]}
                    items={assessment.review_bee.top_items}
                  />
                )}

                {assessment.trust_gate && (
                  <ContextCard
                    title="TrustGate"
                    summary={assessment.trust_gate.summary}
                    badge={assessment.trust_gate.recommendation || "linked"}
                    badgeColor={statusTone(assessment.trust_gate.recommendation)}
                    stats={[
                      `risk ${assessment.trust_gate.risk_score}`,
                      `${assessment.trust_gate.blocked_findings} blocked finding${assessment.trust_gate.blocked_findings === 1 ? "" : "s"}`,
                      `${assessment.trust_gate.warning_findings} warning finding${assessment.trust_gate.warning_findings === 1 ? "" : "s"}`,
                    ]}
                    items={assessment.trust_gate.top_findings}
                  />
                )}

                {assessment.repo_memory && (
                  <ContextCard
                    title="RepoMemory"
                    summary={assessment.repo_memory.summary}
                    badge="context"
                    badgeColor="var(--blue)"
                    stats={[
                      `${assessment.repo_memory.policy_entries} policy entr${assessment.repo_memory.policy_entries === 1 ? "y" : "ies"}`,
                      `${assessment.repo_memory.pinned_entries} pinned entr${assessment.repo_memory.pinned_entries === 1 ? "y" : "ies"}`,
                    ]}
                    items={
                      assessment.repo_memory.top_entries?.length
                        ? assessment.repo_memory.top_entries
                        : assessment.repo_memory.prompt_lines
                    }
                  />
                )}
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

function ContextCard({ title, summary, badge, badgeColor, stats, items }) {
  return (
    <div style={{ border: "1px solid var(--border)", borderRadius: 10, padding: 12, display: "grid", gap: 10, background: "rgba(255,255,255,0.02)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "start", flexWrap: "wrap" }}>
        <div style={{ fontWeight: 700 }}>{title}</div>
        {badge ? <Tag color={badgeColor}>{badge}</Tag> : null}
      </div>

      {summary ? (
        <div style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.6 }}>{summary}</div>
      ) : (
        <div style={{ color: "var(--text-dim)", fontSize: 12 }}>No summary returned.</div>
      )}

      {stats?.length > 0 && (
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          {stats.map((item) => (
            <Tag key={`${title}-${item}`} color="var(--text-dim)">{item}</Tag>
          ))}
        </div>
      )}

      {items?.length > 0 && (
        <div style={{ display: "grid", gap: 6 }}>
          {items.slice(0, 4).map((item, index) => (
            <div key={`${title}-${index}`} style={{ color: "var(--text-dim)", fontSize: 12, lineHeight: 1.5 }}>
              - {item}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

import { useEffect, useMemo, useState } from "react";
import { createApiFetcher, useApiKeyAuth, useProductRuntime } from "@patchhivehq/product-shell/auth";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductV2AuthGate,
  ProductV2Shell,
  ProductRail,
  SuiteRadar,
  SuiteTopline,
  usePersistentProductTab,
} from "@patchhivehq/ui-v2";
import { API } from "./config.js";

const TABS = [
  { id: "readiness", label: "Readiness" },
  { id: "history", label: "Decision log" },
  { id: "checks", label: "Checks" },
];

const POSITIONS = [
  { left: "49%", top: "22%" },
  { left: "72%", top: "48%" },
  { left: "57%", top: "74%" },
  { left: "28%", top: "61%" },
  { left: "31%", top: "34%" },
  { left: "42%", top: "53%" },
  { left: "66%", top: "34%" },
  { left: "37%", top: "76%" },
];

const DEFAULT_FORM = {
  repo: "",
  pr_number: "",
  publish_report: false,
  require_approval: true,
};

function asCount(value) {
  const number = Number(value || 0);
  return Number.isFinite(number) ? number : 0;
}

function timeAgo(value) {
  if (!value) {
    return "never";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return String(value);
  }
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function githubReady(health) {
  return Boolean(health?.github_ready || health?.github?.token_configured);
}

function readinessTone(readiness) {
  const normalized = String(readiness || "").toLowerCase();
  if (normalized === "ready" || normalized === "clear") return "green";
  if (normalized === "blocked" || normalized === "block") return "red";
  if (normalized === "hold" || normalized === "warn" || normalized === "attention") return "amber";
  return "signal";
}

function metricTone(readiness) {
  const tone = readinessTone(readiness);
  if (tone === "green") return "ok";
  if (tone === "red") return "hot";
  if (tone === "amber") return "warn";
  return "sig";
}

function signalTone(signal) {
  const severity = String(signal?.severity || signal?.state || "").toLowerCase();
  if (severity === "block" || severity === "blocked" || severity === "error") return "red";
  if (severity === "warn" || severity === "hold" || severity === "warning") return "amber";
  return "green";
}

function reportTone(report) {
  if (!report) return "signal";
  if (report.delivered) return "green";
  return report.attempted ? "amber" : "signal";
}

async function parseJsonResponse(response, fallbackError) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || data.message || fallbackError);
  }
  return data;
}

function buildTopline(health, overview, assessment, history) {
  const latest = assessment || history[0] || {};
  const integrations = health?.integrations || {};
  const integrationCount = Object.values(integrations).filter(Boolean).length;
  return [
    { label: "MergeKeeper", value: "Merge readiness", tone: "sig" },
    { label: "System", value: health?.status || "checking", tone: health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: "Readiness" },
    { label: "GitHub", value: githubReady(health) ? "PR state" : "token missing", tone: githubReady(health) ? "sig" : "warn" },
    { label: "Suite input", value: `${integrationCount} sources`, tone: integrationCount >= 2 ? "sig" : "warn" },
    { label: "Last call", value: latest.created_at ? timeAgo(latest.created_at) : overview?.counts?.runs ? "loaded" : "none" },
  ];
}

function buildMetrics(assessment, overview, health) {
  if (assessment) {
    const metrics = assessment.metrics || {};
    const successful = asCount(metrics.successful_checks);
    const pending = asCount(metrics.pending_checks);
    const failing = asCount(metrics.failing_checks);
    return [
      { label: "Readiness", value: String(assessment.readiness || "ready").toUpperCase(), tone: metricTone(assessment.readiness), sub: assessment.summary || "latest assessment" },
      { label: "Review pressure", value: String(asCount(metrics.actionable_open_threads || metrics.open_review_threads)), tone: asCount(metrics.actionable_open_threads || metrics.open_review_threads) ? "warn" : "ok", sub: `${asCount(metrics.reviewer_count)} reviewers - ${assessment.approval_required === false ? "approval optional" : "approval required"}` },
      { label: "Checks", value: `${successful}/${successful + pending + failing}`, tone: failing ? "hot" : pending ? "warn" : "ok", sub: `${failing} failing, ${pending} pending` },
      { label: "Risk", value: assessment.trust_gate?.recommendation ? String(assessment.trust_gate.recommendation).toUpperCase() : "LOCAL", tone: metricTone(assessment.trust_gate?.recommendation || "ready"), sub: assessment.trust_gate?.summary || "TrustGate optional" },
      { label: "Changed files", value: String(asCount(metrics.changed_files)), tone: "sig", sub: `+${asCount(metrics.additions)} -${asCount(metrics.deletions)}` },
    ];
  }

  const counts = overview?.counts || {};
  return [
    { label: "Saved calls", value: String(asCount(counts.runs || health?.assessment_count)), tone: "sig", sub: `${asCount(counts.repos || health?.repo_count)} repos` },
    { label: "Ready", value: String(asCount(counts.ready_runs || health?.ready_count)), tone: "ok", sub: "mergeable" },
    { label: "Hold", value: String(asCount(counts.hold_runs || health?.hold_count)), tone: "warn", sub: "needs action" },
    { label: "Blocked", value: String(asCount(counts.blocked_runs || health?.blocked_count)), tone: "hot", sub: "do not merge" },
    { label: "GitHub", value: githubReady(health) ? "READY" : "MISSING", tone: githubReady(health) ? "ok" : "warn", sub: "PR reads" },
  ];
}

function buildRail(assessment, history, overview, health) {
  const latest = history[0] || {};
  const currentRepo = assessment?.repo || latest.repo || overview?.recent_runs?.[0]?.repo || "none";
  const integrations = health?.integrations || {};
  return {
    sections: [
      {
        title: "Pull requests",
        items: assessment
          ? [{ label: `${assessment.repo}#${assessment.pr_number}`, active: true, pin: true }]
          : history.slice(0, 4).map((item, index) => ({
            active: index === 0,
            label: `${item.repo}#${item.pr_number}`,
            value: item.readiness || "saved",
          })),
      },
      {
        title: "Inputs",
        items: [
          { label: "review pressure", active: true, badge: assessment?.review_bee?.status || (integrations.review_bee_configured ? "ready" : "local"), badgeTone: assessment?.review_bee?.open_items ? "amber" : integrations.review_bee_configured ? "green" : "signal" },
          { label: "risk gate", badge: assessment?.trust_gate?.recommendation || (integrations.trust_gate_configured ? "ready" : "local"), badgeTone: readinessTone(assessment?.trust_gate?.recommendation || "ready") },
          { label: "ci health", badge: assessment?.metrics?.failing_checks ? "fail" : assessment?.metrics?.pending_checks ? "pending" : "pass", badgeTone: assessment?.metrics?.failing_checks ? "red" : assessment?.metrics?.pending_checks ? "amber" : "green" },
          { label: "memory rules", badge: assessment?.repo_memory?.policy_entries ? String(assessment.repo_memory.policy_entries) : integrations.repo_memory_configured ? "ready" : "local", badgeTone: "signal" },
        ],
      },
    ],
    stats: {
      title: "Active PR",
      items: [
        { label: "Repository", value: currentRepo },
        { label: "Decision", value: String(assessment?.readiness || latest.readiness || "ready").toUpperCase(), large: true, tone: metricTone(assessment?.readiness || latest.readiness) },
        { label: "Report", value: assessment?.github_report?.state || "local" },
      ],
    },
  };
}

function currentAssessmentItem(assessment) {
  const readiness = assessment?.readiness || "ready";
  return {
    detail: assessment?.repo ? `${assessment.repo}#${assessment.pr_number}` : "Current PR assessment",
    gain: readiness,
    gainMeta: `${asCount(assessment?.blockers?.length)} block / ${asCount(assessment?.warnings?.length)} warn`,
    id: assessment?.id || "current-assessment",
    label: readiness,
    minWindow: 7,
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Repo", value: assessment?.repo || "repo" },
      { label: "PR", value: assessment?.pr_number ? `#${assessment.pr_number}` : "none" },
      { label: "Checks", value: String(asCount(assessment?.metrics?.successful_checks)) },
      { label: "Approval", value: assessment?.approval_required === false ? "optional" : "required" },
      { label: "Blockers", value: String(asCount(assessment?.blockers?.length)) },
      { label: "Warnings", value: String(asCount(assessment?.warnings?.length)) },
    ],
    summary: assessment?.summary || "MergeKeeper current readiness call.",
    title: `${String(readiness).toUpperCase()} call`,
    tone: readinessTone(readiness),
    vector: readiness,
    vectorTone: readinessTone(readiness) === "amber" || readinessTone(readiness) === "red" ? "warn" : "",
  };
}

function signalItem(signal, index, kind) {
  const tone = signalTone(signal);
  return {
    detail: signal.detail || signal.label || kind,
    gain: signal.severity || kind,
    gainMeta: signal.evidence?.[0] || signal.key || kind,
    id: signal.key || `${kind}-${index + 1}`,
    label: signal.severity || kind,
    minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
    position: POSITIONS[(index + 1) % POSITIONS.length],
    stats: [
      { label: "Kind", value: kind },
      { label: "Severity", value: signal.severity || kind },
      { label: "Evidence", value: String(signal.evidence?.length || 0) },
      { label: "Key", value: signal.key || "signal" },
      { label: "State", value: tone },
    ],
    summary: signal.detail || signal.label || "MergeKeeper signal.",
    title: signal.label || signal.key || `${kind} ${index + 1}`,
    tone,
    vector: signal.key || kind,
    vectorTone: tone === "amber" || tone === "red" ? "warn" : "",
  };
}

function contextItems(assessment, startIndex) {
  const items = [];
  if (assessment?.review_bee) {
    items.push({
      detail: "ReviewBee",
      gain: assessment.review_bee.status || "review",
      gainMeta: `${asCount(assessment.review_bee.open_items)} open`,
      id: "review-bee-context",
      label: "RB",
      minWindow: 14,
      position: POSITIONS[startIndex % POSITIONS.length],
      stats: [
        { label: "Status", value: assessment.review_bee.status || "review" },
        { label: "Open", value: String(asCount(assessment.review_bee.open_items)) },
        { label: "Threads", value: String(asCount(assessment.review_bee.actionable_threads)) },
        { label: "Items", value: String(assessment.review_bee.top_items?.length || 0) },
        { label: "Source", value: "ReviewBee" },
      ],
      summary: assessment.review_bee.summary || "ReviewBee context imported into MergeKeeper.",
      title: "ReviewBee context",
      tone: asCount(assessment.review_bee.open_items) ? "amber" : "green",
      vector: "review",
      vectorTone: asCount(assessment.review_bee.open_items) ? "warn" : "",
    });
  }
  if (assessment?.trust_gate) {
    const tone = readinessTone(assessment.trust_gate.recommendation);
    items.push({
      detail: "TrustGate",
      gain: assessment.trust_gate.recommendation || "risk",
      gainMeta: `${asCount(assessment.trust_gate.risk_score)} risk`,
      id: "trust-gate-context",
      label: "TG",
      minWindow: 14,
      position: POSITIONS[(startIndex + items.length) % POSITIONS.length],
      stats: [
        { label: "Decision", value: assessment.trust_gate.recommendation || "risk" },
        { label: "Risk", value: String(asCount(assessment.trust_gate.risk_score)) },
        { label: "Block", value: String(asCount(assessment.trust_gate.blocked_findings)) },
        { label: "Warn", value: String(asCount(assessment.trust_gate.warning_findings)) },
        { label: "Source", value: "TrustGate" },
      ],
      summary: assessment.trust_gate.summary || "TrustGate context imported into MergeKeeper.",
      title: "TrustGate context",
      tone,
      vector: "risk",
      vectorTone: tone === "amber" || tone === "red" ? "warn" : "",
    });
  }
  if (assessment?.repo_memory) {
    items.push({
      detail: "RepoMemory",
      gain: "context",
      gainMeta: `${asCount(assessment.repo_memory.policy_entries)} policy`,
      id: "repo-memory-context",
      label: "RM",
      minWindow: 30,
      position: POSITIONS[(startIndex + items.length) % POSITIONS.length],
      stats: [
        { label: "Policy", value: String(asCount(assessment.repo_memory.policy_entries)) },
        { label: "Pinned", value: String(asCount(assessment.repo_memory.pinned_entries)) },
        { label: "Prompts", value: String(assessment.repo_memory.prompt_lines?.length || 0) },
        { label: "Entries", value: String(assessment.repo_memory.top_entries?.length || 0) },
        { label: "Source", value: "RepoMemory" },
      ],
      summary: assessment.repo_memory.summary || "RepoMemory context imported into MergeKeeper.",
      title: "RepoMemory context",
      tone: "signal",
      vector: "memory",
    });
  }
  return items;
}

function buildRadarItems(assessment, history) {
  if (assessment) {
    const blockers = (assessment.blockers || []).map((signal, index) => signalItem(signal, index, "blocker"));
    const warnings = (assessment.warnings || []).map((signal, index) => signalItem(signal, index + blockers.length, "warning"));
    return [currentAssessmentItem(assessment), ...blockers, ...warnings, ...contextItems(assessment, blockers.length + warnings.length + 1)];
  }

  if (history.length) {
    return history.slice(0, 8).map((item, index) => ({
      detail: `${item.repo}#${item.pr_number}`,
      gain: item.readiness || "saved",
      gainMeta: `${asCount(item.blockers_count)} block / ${asCount(item.warnings_count)} warn`,
      id: item.id || `history-${index + 1}`,
      label: item.readiness || `PR${item.pr_number}`,
      minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
      position: POSITIONS[index % POSITIONS.length],
      stats: [
        { label: "Repo", value: item.repo },
        { label: "PR", value: `#${item.pr_number}` },
        { label: "Block", value: String(asCount(item.blockers_count)) },
        { label: "Warn", value: String(asCount(item.warnings_count)) },
        { label: "Age", value: timeAgo(item.created_at) },
      ],
      summary: item.summary || "Saved MergeKeeper readiness call.",
      title: item.pr_title || `${item.repo}#${item.pr_number}`,
      tone: readinessTone(item.readiness),
      vector: item.readiness || "saved",
      vectorTone: readinessTone(item.readiness) === "amber" || readinessTone(item.readiness) === "red" ? "warn" : "",
    }));
  }

  return [{
    detail: "No PR assessed yet",
    gain: "standby",
    gainMeta: "GitHub PR",
    id: "mergekeeper-ready",
    label: "MK",
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Mode", value: "readiness" },
      { label: "GitHub", value: "required" },
      { label: "ReviewBee", value: "optional" },
      { label: "TrustGate", value: "optional" },
      { label: "Action", value: "assess" },
    ],
    summary: "Assess a GitHub PR to populate MergeKeeper's live readiness radar.",
    title: "MergeKeeper ready",
    tone: "signal",
    vector: "READY",
  }];
}

function buildRadarFeed(assessment, history, health) {
  if (assessment) {
    const report = assessment.github_report;
    return [
      { text: assessment.summary || "MergeKeeper completed the readiness assessment.", tone: readinessTone(assessment.readiness) },
      { text: `${assessment.blockers?.length || 0} blockers and ${assessment.warnings?.length || 0} warnings are active.`, tone: assessment.blockers?.length ? "red" : assessment.warnings?.length ? "amber" : "green" },
      { text: report?.message || "GitHub reporting is optional for this call.", tone: reportTone(report) },
    ];
  }
  return [
    { text: history.length ? `${history.length} saved readiness calls are available.` : "MergeKeeper is waiting for a PR assessment.", tone: "signal" },
    { text: githubReady(health) ? "GitHub token is ready for PR state reads." : "Configure GitHub token access before live PR assessment.", tone: githubReady(health) ? "green" : "amber" },
    { text: "ReviewBee, TrustGate, and RepoMemory integrations strengthen the final call when configured.", tone: "signal" },
  ];
}

function StatusBanner({ tone = "signal", children }) {
  if (!children) return null;
  return <div className={`status-banner ${tone}`}>{children}</div>;
}

function ReadinessMap({ assessment, health, history }) {
  const items = useMemo(() => buildRadarItems(assessment, history), [assessment, history]);
  const feed = useMemo(() => buildRadarFeed(assessment, history, health), [assessment, history, health]);
  return (
    <SuiteRadar
      ariaLabel="MergeKeeper readiness radar"
      detailLabel="Readiness reason"
      feed={feed}
      gainLabel="State"
      itemQueryParam="merge"
      items={items}
      signalLabel={assessment ? "signals" : "runs"}
      vectorLabel={assessment ? "Selected signal" : "Selected run"}
    />
  );
}

function AssessmentForm({ error, form, onChange, onRun, running }) {
  return (
    <Panel eyebrow="Assessment" title="GitHub PR intake" action={<span className="chip signal">readiness</span>}>
      <form
        className="panelbody control-stack"
        onSubmit={(event) => {
          event.preventDefault();
          onRun();
        }}
      >
        <div className="form-grid">
          <label className="v2-field">
            Repository
            <input className="v2-input" onChange={(event) => onChange((current) => ({ ...current, repo: event.target.value }))} placeholder="owner/repo" value={form.repo} />
          </label>
          <label className="v2-field">
            PR number
            <input className="v2-input" inputMode="numeric" onChange={(event) => onChange((current) => ({ ...current, pr_number: event.target.value }))} placeholder="123" value={form.pr_number} />
          </label>
          <div className="v2-field">
            Action
            <button className="btn primary" disabled={running || !form.repo.trim() || !String(form.pr_number).trim()} type="submit">
              {running ? "Assessing..." : "Assess PR"}
            </button>
          </div>
        </div>
        <label className="rowline" style={{ alignItems: "flex-start", justifyContent: "flex-start" }}>
          <input
            checked={Boolean(form.publish_report)}
            onChange={(event) => onChange((current) => ({ ...current, publish_report: event.target.checked }))}
            style={{ marginTop: 3 }}
            type="checkbox"
          />
          <span>
            <span className="repo-name" style={{ display: "block", fontSize: "0.8rem" }}>Publish readiness report</span>
            <span className="feed-meta">Optional write-back. Leave off for local read-only assessment.</span>
          </span>
        </label>
        <label className="rowline" style={{ alignItems: "flex-start", justifyContent: "flex-start" }}>
          <input
            checked={form.require_approval !== false}
            onChange={(event) => onChange((current) => ({ ...current, require_approval: event.target.checked }))}
            style={{ marginTop: 3 }}
            type="checkbox"
          />
          <span>
            <span className="repo-name" style={{ display: "block", fontSize: "0.8rem" }}>Require active approval</span>
            <span className="feed-meta">Turn off for repos where checks and mergeability are enough for a ready call.</span>
          </span>
        </label>
        {error && <StatusBanner tone="red">{error}</StatusBanner>}
      </form>
    </Panel>
  );
}

function BlockerPanel({ assessment, history, onLoadAssessment }) {
  const blockers = assessment ? [...(assessment.blockers || []), ...(assessment.warnings || [])] : [];
  if (assessment) {
    return (
      <Panel eyebrow="Decision" title="Merge blockers" action={<span className={`chip ${readinessTone(assessment.readiness)}`}>{assessment.readiness || "ready"}</span>}>
        <div className="panelbody repo-list queue-grid">
          {blockers.length ? blockers.map((item, index) => (
            <div className="ledger-row" key={item.key || `${item.label}-${index}`}>
              <div className="rank">{String(index + 1).padStart(2, "0")}</div>
              <div>
                <div className="repo-name">{item.label || item.key}</div>
                <div className="feed-meta">{item.detail || item.evidence?.[0]}</div>
              </div>
              <span className={`chip ${signalTone(item)}`}>{item.severity || "signal"}</span>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No blockers</strong>
              <span>MergeKeeper did not find active blockers in the current assessment.</span>
            </div>
          )}
        </div>
      </Panel>
    );
  }

  return (
    <Panel eyebrow="Decision" title="Recent readiness calls" action={<span className="chip signal">{history.length} saved</span>}>
      <div className="panelbody repo-list queue-grid">
        {history.length ? history.slice(0, 5).map((item) => (
          <div className="ledger-row" key={item.id}>
            <div className="rank">#{item.pr_number}</div>
            <div>
              <div className="repo-name">{item.repo}</div>
              <div className="feed-meta">{item.summary || item.pr_title || "Saved MergeKeeper run."}</div>
            </div>
            <button className="btn" onClick={() => onLoadAssessment(item.id)} type="button">Load</button>
          </div>
        )) : (
          <div className="empty-v2">
            <strong>No readiness history</strong>
            <span>Assess a pull request and MergeKeeper will save the call here.</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function SidePanels({ assessment, health }) {
  const report = assessment?.github_report;
  return (
    <aside className="side">
      <Panel eyebrow="Evidence" title="Suite inputs">
        <div className="panelbody repo-list">
          <div className="feed-item">
            <div>
              <div className="feed-title">ReviewBee</div>
              <div className="feed-meta">{assessment?.review_bee?.summary || "Optional review-pressure context."}</div>
            </div>
            <span className={`chip ${assessment?.review_bee?.open_items ? "amber" : "green"}`}>{assessment?.review_bee?.status || "local"}</span>
          </div>
          <div className="feed-item">
            <div>
              <div className="feed-title">TrustGate</div>
              <div className="feed-meta">{assessment?.trust_gate?.summary || "Optional risk-gate context."}</div>
            </div>
            <span className={`chip ${readinessTone(assessment?.trust_gate?.recommendation || "ready")}`}>{assessment?.trust_gate?.recommendation || "local"}</span>
          </div>
          <div className="feed-item">
            <div>
              <div className="feed-title">RepoMemory</div>
              <div className="feed-meta">{assessment?.repo_memory?.summary || "Optional memory context."}</div>
            </div>
            <span className="chip signal">{asCount(assessment?.repo_memory?.policy_entries)} policy</span>
          </div>
        </div>
      </Panel>
      <Panel eyebrow="Output" title="Publish posture">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">GitHub token</span><span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "ready" : "missing"}</span></div>
          <div className="rowline"><span className="muted">Report</span><span className={`chip ${reportTone(report)}`}>{report?.state || "local"}</span></div>
          <div className="rowline"><span className="muted">Webhook</span><span className={`chip ${health?.github?.webhook_secret_configured ? "green" : "amber"}`}>{health?.github?.webhook_secret_configured ? "ready" : "optional"}</span></div>
          {assessment?.pr_url && <button className="btn" onClick={() => window.open(assessment.pr_url, "_blank", "noreferrer")} type="button">Open PR</button>}
          {report?.check_url && <button className="btn" onClick={() => window.open(report.check_url, "_blank", "noreferrer")} type="button">Open report</button>}
        </div>
      </Panel>
    </aside>
  );
}

function ReadinessSurface({
  assessment,
  error,
  form,
  health,
  history,
  onChangeForm,
  onLoadAssessment,
  onRefresh,
  onRunAssessment,
  overview,
  running,
}) {
  const rail = useMemo(() => buildRail(assessment, history, overview, health), [assessment, history, overview, health]);
  const metrics = useMemo(() => buildMetrics(assessment, overview, health), [assessment, overview, health]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, overview, assessment, history)} />
      <div className="main-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - merge readiness</div>
              <h1>Readiness Scope</h1>
              <p className="subline">GitHub state, review pressure, policy risk, and repo memory collapsed into one merge call.</p>
            </div>
            <div className="actions">
              <span className={`chip ${readinessTone(assessment?.readiness || "ready")}`}>{assessment?.readiness || "ready"}</span>
              <span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "github ready" : "github missing"}</span>
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
          </div>
          <AssessmentForm error={error} form={form} onChange={onChangeForm} onRun={onRunAssessment} running={running} />
          <MetricBand metrics={metrics} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Readiness" title="Merge pressure map" action={<span className="chip signal">merge radar</span>}>
              <ReadinessMap assessment={assessment} health={health} history={history} />
            </Panel>
            <BlockerPanel assessment={assessment} history={history} onLoadAssessment={onLoadAssessment} />
          </div>
        </main>
        <SidePanels assessment={assessment} health={health} />
      </div>
    </>
  );
}

function SecondaryFrame({ assessment, children, health, history, overview }) {
  const rail = useMemo(() => buildRail(assessment, history, overview, health), [assessment, history, overview, health]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, overview, assessment, history)} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">{children}</main>
      </div>
    </>
  );
}

function HistorySurface({ activeAssessmentId, assessment, health, history, loading, onLoadAssessment, onRefresh, overview }) {
  return (
    <SecondaryFrame assessment={assessment} health={health} history={history} overview={overview}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// MergeKeeper readiness queue</div>
          <h1>Decision Log</h1>
          <p className="subline">Saved readiness calls and the evidence that changed them.</p>
        </div>
        <button className="btn" onClick={onRefresh} type="button">{loading ? "Refreshing..." : "Refresh"}</button>
      </div>
      <Panel eyebrow="Recent" title="Readiness history" action={<span className="chip signal">{history.length} saved</span>}>
        <div className="panelbody repo-list queue-grid">
          {history.length ? history.map((item) => (
            <div className="ledger-row" key={item.id}>
              <div className="rank">{item.id === activeAssessmentId ? "SEL" : `#${item.pr_number}`}</div>
              <div>
                <div className="repo-name">{item.repo} - PR #{item.pr_number}</div>
                <div className="feed-meta">{item.summary || item.pr_title || "Saved MergeKeeper readiness call."}</div>
                <div className="repo-meta">
                  <span className={`chip ${readinessTone(item.readiness)}`}>{item.readiness || "saved"}</span>
                  <span className="chip red">{asCount(item.blockers_count)} block</span>
                  <span className="chip amber">{asCount(item.warnings_count)} warn</span>
                  <span className="chip">{timeAgo(item.created_at)}</span>
                </div>
              </div>
              <button className="btn" onClick={() => onLoadAssessment(item.id)} type="button">Load</button>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No readiness history</strong>
              <span>Assess a pull request and saved MergeKeeper calls will appear here.</span>
            </div>
          )}
        </div>
      </Panel>
    </SecondaryFrame>
  );
}

function checkTone(level) {
  if (level === "error") return "red";
  if (level === "warn") return "amber";
  return "green";
}

function ChecksSurface({ assessment, history, overview, runtime }) {
  const health = runtime.health || {};
  const checks = runtime.checks || [];
  const warnings = checks.filter((check) => check.level === "warn" || check.level === "error").length;
  const metrics = [
    { label: "Status", value: health.status || "unknown", tone: health.status === "ok" ? "ok" : "warn", sub: health.version || "backend" },
    { label: "GitHub", value: githubReady(health) ? "ready" : "missing", tone: githubReady(health) ? "ok" : "hot", sub: "PR reads" },
    { label: "Assessments", value: String(asCount(health.assessment_count || overview?.counts?.runs)), tone: "sig", sub: `${asCount(health.repo_count || overview?.counts?.repos)} repos` },
    { label: "Integrations", value: String(Object.values(health.integrations || {}).filter(Boolean).length), tone: "sig", sub: "suite inputs" },
    { label: "Checks", value: warnings ? String(warnings) : "clear", tone: warnings ? "warn" : "ok", sub: "startup" },
  ];
  return (
    <SecondaryFrame assessment={assessment} health={health} history={history} overview={overview}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// MergeKeeper readiness checks</div>
          <h1>Checks</h1>
          <p className="subline">Backend health, GitHub access, webhooks, and integration readiness before a merge call.</p>
        </div>
        <button className="btn" onClick={runtime.refresh} type="button">{runtime.loading ? "Refreshing..." : "Refresh"}</button>
      </div>
      {runtime.error && <StatusBanner tone="red">{runtime.error}</StatusBanner>}
      <MetricBand metrics={metrics} />
      <div className="atlas-layout suite-four-layout">
        <Panel eyebrow="Health" title="Backend status" action={<span className={`chip ${health.status === "ok" ? "green" : "amber"}`}>{health.status || "unknown"}</span>}>
          <div className="panelbody repo-list">
            <div className="rowline"><span className="muted">Auth enabled</span><span className={`chip ${health.auth_enabled ? "green" : "amber"}`}>{health.auth_enabled ? "yes" : "no"}</span></div>
            <div className="rowline"><span className="muted">Webhook secret</span><span className={`chip ${health.github?.webhook_secret_configured ? "green" : "amber"}`}>{health.github?.webhook_secret_configured ? "configured" : "missing"}</span></div>
            <div className="rowline"><span className="muted">Report publish</span><span className={`chip ${health.github?.report_publish_ready ? "green" : "amber"}`}>{health.github?.report_publish_ready ? "ready" : "limited"}</span></div>
            <div className="feed-item">
              <div>
                <div className="feed-title">Database</div>
                <div className="feed-meta break-all">{health.db_path || "unknown"}</div>
              </div>
              <span className={`chip ${health.db_ok ? "green" : "red"}`}>{health.db_ok ? "ok" : "check"}</span>
            </div>
          </div>
        </Panel>
        <Panel eyebrow="Startup" title="Startup checks" action={<span className={`chip ${warnings ? "amber" : "green"}`}>{warnings ? `${warnings} warnings` : "clear"}</span>}>
          <div className="panelbody repo-list">
            {checks.length ? checks.map((check, index) => (
              <div className="feed-item" key={`${check.msg}-${index}`}>
                <div>
                  <div className="feed-title">{check.level || "info"}</div>
                  <div className="feed-meta">{check.msg}</div>
                </div>
                <span className={`chip ${checkTone(check.level)}`}>{check.level || "info"}</span>
              </div>
            )) : (
              <div className="empty-v2">
                <strong>No checks</strong>
                <span>No startup checks were returned by the backend.</span>
              </div>
            )}
          </div>
        </Panel>
      </div>
    </SecondaryFrame>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = usePersistentProductTab("merge-keeper", TABS, "readiness");
  const [assessment, setAssessment] = useState(null);
  const [error, setError] = useState("");
  const [form, setForm] = useState(DEFAULT_FORM);
  const [history, setHistory] = useState([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [overview, setOverview] = useState(null);
  const [running, setRunning] = useState(false);
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "merge-keeper_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);
  const approvalDefault = runtime.health?.policy?.approval_required_default;

  async function fetchJson(path, options, fallbackError) {
    const response = await fetch_(`${API}${path}`, options);
    return parseJsonResponse(response, fallbackError);
  }

  async function refreshMergeData() {
    if (!ready) {
      return;
    }
    setLoadingHistory(true);
    const [overviewResult, historyResult] = await Promise.allSettled([
      fetchJson("/overview", undefined, "MergeKeeper could not load overview."),
      fetchJson("/history", undefined, "MergeKeeper could not load history."),
    ]);
    setOverview(overviewResult.status === "fulfilled" ? overviewResult.value : null);
    setHistory(historyResult.status === "fulfilled" ? historyResult.value || [] : []);
    setLoadingHistory(false);
    const failed = [overviewResult, historyResult].find((result) => result.status === "rejected");
    if (failed) {
      setError(failed.reason?.message || "MergeKeeper could not load one or more backend resources.");
    }
  }

  useEffect(() => {
    refreshMergeData();
  }, [ready, fetch_]);

  useEffect(() => {
    if (typeof approvalDefault === "boolean") {
      setForm((current) => ({ ...current, require_approval: approvalDefault }));
    }
  }, [approvalDefault]);

  async function runAssessment() {
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(
        "/assess/github/pr",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            repo: form.repo,
            pr_number: Number(form.pr_number) || 0,
            publish_report: Boolean(form.publish_report),
            require_approval: form.require_approval !== false,
          }),
        },
        "MergeKeeper could not assess that pull request.",
      );
      setAssessment(result);
      setForm({
        repo: result.repo || form.repo,
        pr_number: result.pr_number ? String(result.pr_number) : form.pr_number,
        publish_report: Boolean(form.publish_report),
        require_approval: result.approval_required !== false,
      });
      setActiveTab("readiness");
      await refreshMergeData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "MergeKeeper could not assess that pull request.");
    } finally {
      setRunning(false);
    }
  }

  async function loadAssessment(id) {
    if (!id) return;
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(`/history/${id}`, undefined, "MergeKeeper could not load that assessment.");
      setAssessment(result);
      setForm({
        repo: result.repo || "",
        pr_number: result.pr_number ? String(result.pr_number) : "",
        publish_report: false,
        require_approval: result.approval_required !== false,
      });
      setActiveTab("readiness");
    } catch (err) {
      setError(err.message || "MergeKeeper could not load that assessment.");
    } finally {
      setRunning(false);
    }
  }

  if (!ready) {
    return (
      <ProductV2AuthGate
        apiBase={API}
        auth={auth}
        keyPrefix="merge-keeper-"
        productKey="merge-keeper"
        productName="MergeKeeper"
      />
    );
  }

  return (
    <ProductV2Shell authConfigured={authConfigured} runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="MergeKeeper"
        navLabel="MergeKeeper navigation"
        onTabChange={setActiveTab}
        productKey="merge-keeper"
        tabs={TABS}
      />
      {activeTab === "readiness" && (
        <ReadinessSurface
          assessment={assessment}
          error={error}
          form={form}
          health={runtime.health || {}}
          history={history}
          onChangeForm={setForm}
          onLoadAssessment={loadAssessment}
          onRefresh={() => {
            refreshMergeData();
            runtime.refresh();
          }}
          onRunAssessment={runAssessment}
          overview={overview}
          running={running}
        />
      )}
      {activeTab === "history" && (
        <HistorySurface
          activeAssessmentId={assessment?.id || ""}
          assessment={assessment}
          health={runtime.health || {}}
          history={history}
          loading={loadingHistory}
          onLoadAssessment={loadAssessment}
          onRefresh={refreshMergeData}
          overview={overview}
        />
      )}
      {activeTab === "checks" && <ChecksSurface assessment={assessment} history={history} overview={overview} runtime={runtime} />}
    </ProductV2Shell>
  );
}

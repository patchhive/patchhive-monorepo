import { useEffect, useMemo, useState } from "react";
import { createApiFetcher, useApiKeyAuth, useProductRuntime } from "@patchhivehq/product-shell/auth";
import {
  DeckBar,
  HistoryDetailGrid,
  MetricBand,
  Panel,
  ProductV2AuthGate,
  ProductV2Shell,
  ProductRail,
  SuiteRadar,
  SuiteTopline,
  radarWindowFromTimestamp,
  usePersistentProductTab,
} from "@patchhivehq/ui-v2";
import { API } from "./config.js";

const TABS = [
  { id: "gate", label: "Release gate" },
  { id: "history", label: "Run history" },
  { id: "checks", label: "Checks" },
];

const POSITIONS = [
  { left: "35%", top: "34%" },
  { left: "69%", top: "33%" },
  { left: "55%", top: "66%" },
  { left: "28%", top: "70%" },
  { left: "75%", top: "62%" },
  { left: "48%", top: "48%" },
  { left: "62%", top: "24%" },
  { left: "42%", top: "76%" },
];

const DEFAULT_FORM = {
  repo: "",
  branch: "",
  target_version: "",
  target_tag: "",
  changelog_path: "CHANGELOG.md",
  workflow_run_limit: "20",
  blocker_labels: "release-blocker, blocker, critical, regression",
};

function asCount(value) {
  const number = Number(value || 0);
  return Number.isFinite(number) ? number : 0;
}

function timeAgo(value) {
  if (!value) return "never";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
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

function decisionTone(decision) {
  const value = String(decision || "").toLowerCase();
  if (value === "hold" || value === "block") return "red";
  if (value === "watch" || value === "warn") return "amber";
  if (value === "ready" || value === "pass") return "green";
  return "signal";
}

function metricTone(decision) {
  const tone = decisionTone(decision);
  if (tone === "red") return "hot";
  if (tone === "amber") return "warn";
  if (tone === "green") return "ok";
  return "sig";
}

async function parseJsonResponse(response, fallbackError) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || data.message || fallbackError);
  }
  return data;
}

function splitLabels(value) {
  return String(value || "")
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function releaseLabel(run = {}) {
  return run.target_tag || run.target_version || run.branch || "next release";
}

function buildTopline(health, overview, run, history) {
  const counts = overview?.counts || {};
  const latest = run || history[0] || {};
  const decision = run?.decision || latest.decision;
  return [
    { label: "ReleaseSentry", value: "Release gate", tone: "sig" },
    { label: "System", value: health?.status || "checking", tone: health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: "Read only" },
    { label: "GitHub", value: githubReady(health) ? "Release read" : "token missing", tone: githubReady(health) ? "sig" : "warn" },
    { label: "Decision", value: decision || (counts.hold ? "hold" : counts.watch ? "watch" : counts.ready ? "ready" : "none"), tone: decisionTone(decision || (counts.hold ? "hold" : counts.watch ? "watch" : "ready")) },
    { label: "Last check", value: latest.created_at ? timeAgo(latest.created_at) : counts.runs ? "loaded" : "none" },
  ];
}

function buildMetrics(run, overview, health) {
  if (run) {
    const metrics = run.metrics || {};
    return [
      { label: "Readiness", value: String(asCount(run.score)), tone: metricTone(run.decision), sub: run.decision || "decision" },
      { label: "CI pass", value: `${asCount(metrics.workflow_successes)}`, tone: metricTone(metrics.workflow_failures ? "hold" : metrics.workflow_pending ? "watch" : "ready"), sub: `of ${asCount(metrics.workflow_runs)} runs` },
      { label: "Blockers", value: String(asCount(metrics.blocked || metrics.release_blockers)), tone: metricTone(metrics.blocked || metrics.release_blockers ? "hold" : "ready"), sub: "release pressure" },
      { label: "Warnings", value: String(asCount(metrics.warned)), tone: metricTone(metrics.warned ? "watch" : "ready"), sub: "watch items" },
      { label: "Evidence", value: String(asCount(metrics.checks)), tone: "sig", sub: `${asCount(metrics.passed)} passed` },
    ];
  }
  const counts = overview?.counts || {};
  return [
    { label: "Runs", value: String(asCount(counts.runs || health?.run_count)), tone: "sig", sub: `${asCount(counts.repos || health?.repo_count)} repos` },
    { label: "Ready", value: String(asCount(counts.ready || health?.ready_count)), tone: "ok", sub: "saved checks" },
    { label: "Watch", value: String(asCount(counts.watch || health?.watch_count)), tone: "warn", sub: "saved checks" },
    { label: "Hold", value: String(asCount(counts.hold || health?.hold_count)), tone: metricTone((counts.hold || health?.hold_count) ? "hold" : "ready"), sub: "saved checks" },
    { label: "GitHub", value: githubReady(health) ? "ready" : "missing", tone: githubReady(health) ? "ok" : "warn", sub: "release reads" },
  ];
}

function buildRail(run, history, overview, health) {
  const latest = history[0] || {};
  const metrics = run?.metrics || {};
  return {
    sections: [
      {
        title: "Candidate",
        items: [
          { label: `${run?.branch || latest.branch || "branch"} -> ${releaseLabel(run || latest)}`, active: true, pin: true },
          { label: "workflow runs", value: String(asCount(metrics.workflow_runs)) },
          { label: "target tag", value: run?.target_tag || latest.target_tag || "not set" },
          { label: "release blockers", value: String(asCount(metrics.release_blockers)) },
        ],
      },
      {
        title: "Checks",
        items: [
          { label: "CI health", active: true, badge: `${asCount(metrics.workflow_successes)}/${asCount(metrics.workflow_runs)}`, badgeTone: metrics.workflow_failures ? "red" : "green" },
          { label: "blocked", badge: String(asCount(metrics.blocked)), badgeTone: "red" },
          { label: "warned", badge: String(asCount(metrics.warned)), badgeTone: "amber" },
          { label: "saved runs", badge: String(history.length), badgeTone: "signal" },
        ],
      },
    ],
    stats: {
      title: "Active release",
      items: [
        { label: "Repository", value: run?.repo || latest.repo || "none" },
        { label: "Decision", value: (run?.decision || latest.decision || "READY").toUpperCase(), large: true, tone: metricTone(run?.decision || latest.decision || "ready") },
        { label: "Score", value: String(asCount(run?.score || latest.score)) },
      ],
    },
  };
}

function buildRadarItems(run, history) {
  if (run?.checks?.length) {
    return run.checks.slice(0, 8).map((check, index) => {
      const tone = decisionTone(check.status);
      return {
        detail: check.label || check.key,
        gain: check.status || "check",
        gainMeta: check.key || "evidence",
        id: check.key || `check-${index + 1}`,
        label: check.label || check.key || `R${index + 1}`,
        minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
        position: POSITIONS[index % POSITIONS.length],
        stats: [
          { label: "Status", value: check.status || "unknown" },
          { label: "Evidence", value: String(check.evidence?.length || 0) },
          { label: "Links", value: String(check.links?.length || 0) },
          { label: "Gate", value: run.decision || "watch" },
          { label: "Release", value: releaseLabel(run) },
        ],
        summary: check.detail || check.evidence?.[0] || "Release readiness check.",
        title: check.label || check.key || `Check ${index + 1}`,
        tone,
        vector: check.key || check.status || "release",
        vectorTone: tone === "red" || tone === "amber" ? "warn" : "",
      };
    });
  }
  if (history.length) {
    return history.map((item, index) => {
      const minWindow = radarWindowFromTimestamp(item.created_at);
      if (!minWindow) {
        return null;
      }
      const tone = decisionTone(item.decision);
      return {
        detail: item.repo,
        gain: item.decision || "saved",
        gainMeta: `${asCount(item.score)} score`,
        id: item.id || `history-${index + 1}`,
        label: item.target_tag || item.target_version || `R${index + 1}`,
        minWindow,
        position: POSITIONS[index % POSITIONS.length],
        stats: [
          { label: "Repo", value: item.repo },
          { label: "Branch", value: item.branch || "default" },
          { label: "Decision", value: item.decision || "saved" },
          { label: "Score", value: String(asCount(item.score)) },
          { label: "Age", value: timeAgo(item.created_at) },
        ],
        summary: item.summary || "Saved ReleaseSentry check.",
        title: `${item.repo} / ${releaseLabel(item)}`,
        tone,
        vector: "saved",
        vectorTone: tone === "red" || tone === "amber" ? "warn" : "",
      };
    }).filter(Boolean);
  }
  return [{
    detail: "No release check yet",
    gain: "standby",
    gainMeta: "GitHub repo",
    id: "release-sentry-ready",
    label: "RSY",
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Mode", value: "read only" },
      { label: "Actions", value: "ready" },
      { label: "Tags", value: "ready" },
      { label: "History", value: "empty" },
      { label: "Action", value: "check" },
    ],
    summary: "Check a GitHub repository to turn release evidence into a ship, watch, or hold decision.",
    title: "ReleaseSentry ready",
    tone: "signal",
    vector: "READY",
  }];
}

function buildRadarFeed(run, history, health) {
  if (run) {
    return [
      { text: run.summary || "ReleaseSentry completed the release check.", tone: decisionTone(run.decision) },
      { text: `${asCount(run.metrics?.passed)} passed, ${asCount(run.metrics?.warned)} warned, and ${asCount(run.metrics?.blocked)} blocked checks are active.`, tone: decisionTone(run.decision) },
      { text: run.warnings?.[0] || "Release evidence is grouped into a ship/no-ship call.", tone: run.warnings?.length ? "amber" : "signal" },
    ];
  }
  return [
    { text: history.length ? `${history.length} saved release checks are available.` : "ReleaseSentry is waiting for a release check.", tone: "signal" },
    { text: githubReady(health) ? "GitHub token is ready for release, tag, Actions, and issue reads." : "Configure GitHub token access before live checks.", tone: githubReady(health) ? "green" : "amber" },
    { text: "The radar fills with release checks once a run completes.", tone: "signal" },
  ];
}

function StatusBanner({ tone = "signal", children }) {
  if (!children) return null;
  return <div className={`status-banner ${tone}`}>{children}</div>;
}

function ReleaseMap({ health, history, run }) {
  const items = useMemo(() => buildRadarItems(run, history), [run, history]);
  const feed = useMemo(() => buildRadarFeed(run, history, health), [run, history, health]);
  const hasChecks = Boolean(run?.checks?.length);
  return (
    <SuiteRadar
      ariaLabel="ReleaseSentry readiness radar"
      detailLabel={hasChecks ? "Evidence" : "Release target"}
      feed={feed}
      gainLabel="Decision"
      itemQueryParam="release"
      items={items}
      signalLabel={hasChecks ? "checks" : "runs"}
      vectorLabel={hasChecks ? "Selected check" : "Selected run"}
    />
  );
}

function CheckForm({ error, form, onChange, onRun, running }) {
  const set = (key, value) => onChange((current) => ({ ...current, [key]: value }));
  return (
    <Panel eyebrow="Check" title="GitHub release intake" action={<span className="chip signal">read only</span>}>
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
            <input className="v2-input" onChange={(event) => set("repo", event.target.value)} placeholder="owner/repo" value={form.repo} />
          </label>
          <label className="v2-field">
            Branch
            <input className="v2-input" onChange={(event) => set("branch", event.target.value)} placeholder="default branch" value={form.branch} />
          </label>
          <label className="v2-field">
            Target version
            <input className="v2-input" onChange={(event) => set("target_version", event.target.value)} placeholder="0.2.0" value={form.target_version} />
          </label>
          <label className="v2-field">
            Target tag
            <input className="v2-input" onChange={(event) => set("target_tag", event.target.value)} placeholder="v0.2.0" value={form.target_tag} />
          </label>
          <label className="v2-field">
            Changelog
            <input className="v2-input" onChange={(event) => set("changelog_path", event.target.value)} value={form.changelog_path} />
          </label>
          <label className="v2-field">
            Run limit
            <input className="v2-input" min="5" max="100" onChange={(event) => set("workflow_run_limit", event.target.value)} type="number" value={form.workflow_run_limit} />
          </label>
          <label className="v2-field">
            Blocker labels
            <input className="v2-input" onChange={(event) => set("blocker_labels", event.target.value)} value={form.blocker_labels} />
          </label>
          <div className="v2-field">
            Action
            <button className="btn primary" disabled={running || !form.repo.trim()} type="submit">
              {running ? "Checking..." : "Check release"}
            </button>
          </div>
        </div>
        {error && <StatusBanner tone="red">{error}</StatusBanner>}
      </form>
    </Panel>
  );
}

function GateQueuePanel({ history, onLoadRun, run }) {
  if (run) {
    return (
      <Panel eyebrow="Queue" title="Release evidence" action={<span className={`chip ${decisionTone(run.decision)}`}>{run.decision || "decision"}</span>}>
        <div className="panelbody repo-list queue-grid">
          {run.checks?.length ? run.checks.map((check, index) => (
            <div className="ledger-row" key={check.key || index}>
              <div className="rank">{String(index + 1).padStart(2, "0")}</div>
              <div>
                <div className="repo-name">{check.label || check.key}</div>
                <div className="feed-meta">{check.detail}</div>
                <div className="repo-meta">
                  <span className={`chip ${decisionTone(check.status)}`}>{check.status || "check"}</span>
                  <span className="chip signal">{check.key || "evidence"}</span>
                </div>
              </div>
              <span className={`chip ${decisionTone(check.status)}`}>{check.evidence?.length || 0}</span>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No checks</strong>
              <span>This run did not return release checks.</span>
            </div>
          )}
        </div>
      </Panel>
    );
  }
  return (
    <Panel eyebrow="Queue" title="Recent release checks" action={<span className="chip signal">{history.length} saved</span>}>
      <div className="panelbody repo-list queue-grid">
        {history.length ? history.slice(0, 5).map((item) => (
          <div className="ledger-row" key={item.id}>
            <div className="rank">{asCount(item.score)}</div>
            <div>
              <div className="repo-name">{item.repo} / {releaseLabel(item)}</div>
              <div className="feed-meta">{item.summary || "Saved ReleaseSentry run."}</div>
              <div className="repo-meta">
                <span className={`chip ${decisionTone(item.decision)}`}>{item.decision || "decision"}</span>
                <span className="chip signal">{item.branch || "branch"}</span>
                <span className="chip">{timeAgo(item.created_at)}</span>
              </div>
            </div>
            <button className="btn" onClick={() => onLoadRun(item.id)} type="button">Load</button>
          </div>
        )) : (
          <div className="empty-v2">
            <strong>No checks yet</strong>
            <span>Check a GitHub repo to populate the release gate.</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function SidePanels({ run }) {
  const warnings = run?.warnings || [];
  return (
    <aside className="side">
      <Panel eyebrow="Evidence" title="Ship/no-ship call">
        <div className="panelbody repo-list">
          {warnings.length ? warnings.slice(0, 3).map((warning) => (
            <div className="feed-item" key={warning}>
              <div>
                <div className="feed-title">Run warning</div>
                <div className="feed-meta">{warning}</div>
              </div>
              <span className="chip amber">warn</span>
            </div>
          )) : (
            <>
              <div className="rowline"><span className="muted">Decision</span><span className={`chip ${decisionTone(run?.decision)}`}>{run?.decision || "ready"}</span></div>
              <div className="rowline"><span className="muted">Blocked checks</span><span className={`chip ${run?.metrics?.blocked ? "red" : "green"}`}>{asCount(run?.metrics?.blocked)}</span></div>
              <div className="rowline"><span className="muted">Warnings</span><span className={`chip ${run?.metrics?.warned ? "amber" : "green"}`}>{asCount(run?.metrics?.warned)}</span></div>
            </>
          )}
        </div>
      </Panel>
      <Panel eyebrow="Consumers" title="Signal handoff">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">MergeKeeper</span><span className="chip signal">PR ready</span></div>
          <div className="rowline"><span className="muted">VulnTriage</span><span className={`chip ${run?.decision === "hold" ? "red" : "amber"}`}>{run?.decision === "hold" ? "blockers" : "watch"}</span></div>
          <div className="rowline"><span className="muted">HiveCore</span><span className={`chip ${decisionTone(run?.decision)}`}>{run?.decision || "gate"}</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function GateSurface({
  error,
  form,
  health,
  history,
  onChangeForm,
  onClearRun,
  onLoadRun,
  onRefresh,
  onRunCheck,
  overview,
  running,
  run,
}) {
  const rail = useMemo(() => buildRail(run, history, overview, health), [run, history, overview, health]);
  const metrics = useMemo(() => buildMetrics(run, overview, health), [run, overview, health]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, overview, run, history)} />
      <div className="main-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - release readiness</div>
              <h1>Release Gate</h1>
              <p className="subline">Branch health, blockers, changelog drift, tag alignment, and cross-product pressure turned into a ship call.</p>
            </div>
            <div className="actions">
              <span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "github ready" : "token missing"}</span>
              {run && <button className="btn" onClick={onClearRun} type="button">Clear run</button>}
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
          </div>
          <CheckForm error={error} form={form} onChange={onChangeForm} onRun={onRunCheck} running={running} />
          <MetricBand metrics={metrics} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Gate" title="Readiness map" action={<span className="chip signal">release radar</span>}>
              <ReleaseMap health={health} history={history} run={null} />
            </Panel>
            <GateQueuePanel history={history} onLoadRun={onLoadRun} run={run} />
          </div>
        </main>
        <SidePanels run={run} />
      </div>
    </>
  );
}

function SecondaryFrame({ children, health, history, overview, run }) {
  const rail = useMemo(() => buildRail(run, history, overview, health), [run, history, overview, health]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, overview, run, history)} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">{children}</main>
      </div>
    </>
  );
}

function HistorySurface({ activeRunId, health, history, loading, onClearRun, onLoadRun, onRefresh, overview, run }) {
  return (
    <SecondaryFrame health={health} history={history} overview={overview} run={run}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// ReleaseSentry release gate</div>
          <h1>Run History</h1>
          <p className="subline">Saved release checks with readiness changes, blockers, and final ship/no-ship evidence.</p>
        </div>
        <div className="actions">
          {run && <button className="btn" onClick={onClearRun} type="button">Clear run</button>}
          <button className="btn" onClick={onRefresh} type="button">{loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      <Panel eyebrow="Recent" title="Release checks" action={<span className="chip signal">{history.length} saved</span>}>
        <div className="panelbody repo-list queue-grid">
          {history.length ? history.map((item) => (
            <div className="ledger-row" key={item.id}>
              <div className="rank">{item.id === activeRunId ? "SEL" : asCount(item.score)}</div>
              <div>
                <div className="repo-name">{item.repo} / {releaseLabel(item)}</div>
                <div className="feed-meta">{item.summary || "Saved ReleaseSentry check."}</div>
                <div className="repo-meta">
                  <span className={`chip ${decisionTone(item.decision)}`}>{item.decision || "decision"}</span>
                  <span className="chip signal">{item.branch || "branch"}</span>
                  <span className="chip">{timeAgo(item.created_at)}</span>
                </div>
              </div>
              <button className="btn" onClick={() => onLoadRun(item.id)} type="button">Load</button>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No checks saved</strong>
              <span>Check a release candidate to create history.</span>
            </div>
          )}
        </div>
      </Panel>
      {run && (
        <HistoryDetailGrid>
          <Panel eyebrow="Gate" title="Selected readiness map" action={<span className="chip signal">release radar</span>}>
            <ReleaseMap health={health} history={history} run={run} />
          </Panel>
          <GateQueuePanel history={history} onLoadRun={onLoadRun} run={run} />
        </HistoryDetailGrid>
      )}
    </SecondaryFrame>
  );
}

function checkTone(level) {
  if (level === "error") return "red";
  if (level === "warn") return "amber";
  return "green";
}

function ChecksSurface({ history, onClearRun, overview, runtime, run }) {
  const health = runtime.health || {};
  const checks = runtime.checks || [];
  const warnings = checks.filter((check) => check.level === "warn" || check.level === "error").length;
  const metrics = [
    { label: "Status", value: health.status || "unknown", tone: health.status === "ok" ? "ok" : "warn", sub: health.version || "backend" },
    { label: "GitHub", value: githubReady(health) ? "ready" : "missing", tone: githubReady(health) ? "ok" : "warn", sub: "release reads" },
    { label: "Runs", value: String(asCount(health.run_count || overview?.counts?.runs)), tone: "sig", sub: `${asCount(health.repo_count || overview?.counts?.repos)} repos` },
    { label: "Holds", value: String(asCount(health.hold_count || overview?.counts?.hold)), tone: metricTone((health.hold_count || overview?.counts?.hold) ? "hold" : "ready"), sub: "saved checks" },
    { label: "Checks", value: warnings ? String(warnings) : "clear", tone: warnings ? "warn" : "ok", sub: "startup" },
  ];
  return (
    <SecondaryFrame health={health} history={history} overview={overview} run={run}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// ReleaseSentry readiness</div>
          <h1>Checks</h1>
          <p className="subline">Backend health, GitHub release permissions, saved run counts, and startup checks.</p>
        </div>
        <div className="actions">
          {run && <button className="btn" onClick={onClearRun} type="button">Clear run</button>}
          <button className="btn" onClick={runtime.refresh} type="button">{runtime.loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      {runtime.error && <StatusBanner tone="red">{runtime.error}</StatusBanner>}
      <MetricBand metrics={metrics} />
      <div className="atlas-layout suite-four-layout">
        <Panel eyebrow="Health" title="Backend status" action={<span className={`chip ${health.status === "ok" ? "green" : "amber"}`}>{health.status || "unknown"}</span>}>
          <div className="panelbody repo-list">
            <div className="rowline"><span className="muted">Auth enabled</span><span className={`chip ${health.auth_enabled ? "green" : "amber"}`}>{health.auth_enabled ? "yes" : "no"}</span></div>
            <div className="rowline"><span className="muted">GitHub ready</span><span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "yes" : "no"}</span></div>
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
  const [activeTab, setActiveTab] = usePersistentProductTab("release-sentry", TABS, "gate");
  const [error, setError] = useState("");
  const [form, setForm] = useState(DEFAULT_FORM);
  const [history, setHistory] = useState([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [overview, setOverview] = useState(null);
  const [running, setRunning] = useState(false);
  const [run, setRun] = useState(null);
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "release-sentry_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

  async function fetchJson(path, options, fallbackError) {
    const response = await fetch_(`${API}${path}`, options);
    return parseJsonResponse(response, fallbackError);
  }

  async function refreshGateData() {
    if (!ready) return;
    setLoadingHistory(true);
    const [overviewResult, historyResult] = await Promise.allSettled([
      fetchJson("/overview", undefined, "ReleaseSentry could not load overview."),
      fetchJson("/history", undefined, "ReleaseSentry could not load history."),
    ]);
    setOverview(overviewResult.status === "fulfilled" ? overviewResult.value : null);
    setHistory(historyResult.status === "fulfilled" ? historyResult.value || [] : []);
    setLoadingHistory(false);
    const failed = [overviewResult, historyResult].find((result) => result.status === "rejected");
    if (failed) {
      setError(failed.reason?.message || "ReleaseSentry could not load one or more backend resources.");
    }
  }

  useEffect(() => {
    refreshGateData();
  }, [ready, fetch_]);

  async function runCheck() {
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(
        "/check/github/release",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            repo: form.repo,
            branch: form.branch,
            target_version: form.target_version,
            target_tag: form.target_tag,
            changelog_path: form.changelog_path,
            workflow_run_limit: Number(form.workflow_run_limit) || 20,
            blocker_labels: splitLabels(form.blocker_labels),
          }),
        },
        "ReleaseSentry could not check that release.",
      );
      setRun(result);
      setForm((current) => ({
        ...current,
        repo: result.repo || current.repo,
        branch: result.branch || current.branch,
        target_version: result.target_version || current.target_version,
        target_tag: result.target_tag || current.target_tag,
      }));
      setActiveTab("gate");
      await refreshGateData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "ReleaseSentry could not check that release.");
    } finally {
      setRunning(false);
    }
  }

  async function loadRun(id) {
    if (!id) return;
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(`/history/${id}`, undefined, "ReleaseSentry could not load that run.");
      setRun(result);
      setForm((current) => ({
        ...current,
        repo: result.repo || current.repo,
        branch: result.branch || current.branch,
        target_version: result.target_version || current.target_version,
        target_tag: result.target_tag || current.target_tag,
      }));
    } catch (err) {
      setError(err.message || "ReleaseSentry could not load that run.");
    } finally {
      setRunning(false);
    }
  }

  function clearRun() {
    setRun(null);
    setError("");
  }

  if (!ready) {
    return (
      <ProductV2AuthGate
        apiBase={API}
        auth={auth}
        keyPrefix="release-sentry-"
        productKey="release-sentry"
        productName="ReleaseSentry"
      />
    );
  }

  return (
    <ProductV2Shell authConfigured={authConfigured} productKey="release-sentry" productName="ReleaseSentry" runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="ReleaseSentry"
        navLabel="ReleaseSentry navigation"
        onTabChange={setActiveTab}
        productKey="release-sentry"
        tabs={TABS}
      />
      {activeTab === "gate" && (
        <GateSurface
          error={error}
          form={form}
          health={runtime.health || {}}
          history={history}
          onChangeForm={setForm}
          onClearRun={clearRun}
          onLoadRun={loadRun}
          onRefresh={() => {
            refreshGateData();
            runtime.refresh();
          }}
          onRunCheck={runCheck}
          overview={overview}
          running={running}
          run={run}
        />
      )}
      {activeTab === "history" && (
        <HistorySurface
          activeRunId={run?.id || ""}
          health={runtime.health || {}}
          history={history}
          loading={loadingHistory}
          onClearRun={clearRun}
          onLoadRun={loadRun}
          onRefresh={refreshGateData}
          overview={overview}
          run={run}
        />
      )}
      {activeTab === "checks" && <ChecksSurface history={history} onClearRun={clearRun} overview={overview} runtime={runtime} run={run} />}
    </ProductV2Shell>
  );
}

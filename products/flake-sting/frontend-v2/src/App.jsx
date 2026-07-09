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
  { id: "instability", label: "Instability" },
  { id: "history", label: "Scan history" },
  { id: "checks", label: "Checks" },
];

const POSITIONS = [
  { left: "50%", top: "23%" },
  { left: "72%", top: "47%" },
  { left: "59%", top: "75%" },
  { left: "28%", top: "62%" },
  { left: "31%", top: "35%" },
  { left: "46%", top: "54%" },
  { left: "69%", top: "64%" },
  { left: "37%", top: "76%" },
];

const DEFAULT_FORM = {
  repo: "",
  branch: "",
  workflow_name: "",
  lookback_runs: "25",
};

const SORT_OPTIONS = [
  { value: "risk", label: "Risk first" },
  { value: "failures", label: "Most failures" },
  { value: "reruns", label: "Most reruns" },
  { value: "workflow", label: "Workflow name" },
];

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

function signalTone(signal) {
  const status = String(signal?.status || signal?.kind || "").toLowerCase();
  if (status.includes("quarantine") || status.includes("unstable") || asCount(signal?.score) >= 80) return "red";
  if (status.includes("watch") || status.includes("runner") || asCount(signal?.score) >= 50) return "amber";
  if (status.includes("stable")) return "green";
  return "signal";
}

function metricToneFromSignalCount(count) {
  if (asCount(count) >= 8) return "hot";
  if (asCount(count) > 0) return "warn";
  return "ok";
}

function sortSignals(signals, sortBy) {
  return [...signals].sort((left, right) => {
    if (sortBy === "failures") {
      return asCount(right.failure_count) - asCount(left.failure_count) || asCount(right.score) - asCount(left.score);
    }
    if (sortBy === "reruns") {
      return asCount(right.rerun_hits) - asCount(left.rerun_hits) || asCount(right.score) - asCount(left.score);
    }
    if (sortBy === "workflow") {
      return (
        String(left.workflow_name || "").localeCompare(String(right.workflow_name || "")) ||
        String(left.job_name || "").localeCompare(String(right.job_name || "")) ||
        String(left.step_name || "").localeCompare(String(right.step_name || ""))
      );
    }

    const leftPriority = left.status === "quarantine" ? 1 : 0;
    const rightPriority = right.status === "quarantine" ? 1 : 0;
    return (
      rightPriority - leftPriority ||
      asCount(right.score) - asCount(left.score) ||
      asCount(right.failure_count) - asCount(left.failure_count) ||
      asCount(right.rerun_hits) - asCount(left.rerun_hits)
    );
  });
}

function buildScanMarkdown(scan) {
  const metrics = scan?.metrics || {};
  const lines = [
    `# FlakeSting scan for ${scan?.repo || "repository"}`,
    "",
    scan?.summary || "FlakeSting workflow scan.",
    "",
    `- Branch: ${scan?.branch || "all branches"}`,
    `- Workflow filter: ${scan?.workflow_name || "all workflows"}`,
    `- Workflow runs: ${asCount(metrics.workflow_runs)}`,
    `- Failed runs: ${asCount(metrics.failed_runs)}`,
    `- Rerun-like runs: ${asCount(metrics.rerun_like_runs)}`,
    `- Flaky signals: ${asCount(metrics.flaky_signals)}`,
    `- Quarantine candidates: ${asCount(metrics.quarantine_candidates)}`,
  ];

  if (scan?.trend?.status) {
    lines.push(
      "",
      "## Trend",
      "",
      `- Status: ${scan.trend.status}`,
      `- Signal delta: ${scan.trend.flaky_signal_delta > 0 ? "+" : ""}${asCount(scan.trend.flaky_signal_delta)}`,
      `- Quarantine delta: ${scan.trend.quarantine_delta > 0 ? "+" : ""}${asCount(scan.trend.quarantine_delta)}`,
      `- Rerun delta: ${scan.trend.rerun_delta > 0 ? "+" : ""}${asCount(scan.trend.rerun_delta)}`,
      `- New signals: ${asCount(scan.trend.new_signal_count)}`,
      `- Cleared signals: ${asCount(scan.trend.cleared_signal_count)}`,
    );
  }

  if (scan?.signals?.length) {
    lines.push("", "## Top signals", "");
    sortSignals(scan.signals, "risk").slice(0, 8).forEach((signal) => {
      lines.push(`- [${signal.status || "signal"}] ${signal.step_name || signal.job_name || signal.workflow_name} - ${signal.summary || signal.evidence?.[0] || "No summary available."}`);
    });
  }

  return lines.join("\n");
}

function evidenceUrl(value) {
  return String(value || "").match(/https?:\/\/\S+/)?.[0]?.replace(/[),.]+$/, "") || "";
}

function trendTone(status) {
  if (status === "rising") return "red";
  if (status === "improving") return "green";
  if (status === "shifted") return "amber";
  return "signal";
}

function signedCount(value) {
  const count = asCount(Math.abs(value));
  if (Number(value) > 0) return `+${count}`;
  if (Number(value) < 0) return `-${count}`;
  return "0";
}

async function parseJsonResponse(response, fallbackError) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || data.message || fallbackError);
  }
  return data;
}

function buildTopline(health, overview, scan, history) {
  const counts = overview?.counts || {};
  const latest = scan || history[0] || {};
  return [
    { label: "FlakeSting", value: "CI trust", tone: "sig" },
    { label: "System", value: health?.status || "checking", tone: health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: "Read only" },
    { label: "GitHub", value: githubReady(health) ? "Actions read" : "token missing", tone: githubReady(health) ? "sig" : "warn" },
    { label: scan ? "Flaky" : "Saved signals", value: String(asCount(scan?.metrics?.flaky_signals || counts.flaky_signals || health?.flaky_signal_count)), tone: "warn" },
    { label: "Last scan", value: latest.created_at ? timeAgo(latest.created_at) : counts.scans ? "loaded" : "none" },
  ];
}

function buildMetrics(scan, overview, health) {
  if (scan) {
    const metrics = scan.metrics || {};
    return [
      { label: "Flaky signals", value: String(asCount(metrics.flaky_signals)), tone: metricToneFromSignalCount(metrics.flaky_signals), sub: scan.trend ? `${scan.trend.flaky_signal_delta > 0 ? "+" : ""}${scan.trend.flaky_signal_delta} vs prior` : "current scan" },
      { label: "Failed runs", value: String(asCount(metrics.failed_runs)), tone: asCount(metrics.failed_runs) ? "warn" : "ok", sub: `${asCount(metrics.workflow_runs)} runs` },
      { label: "Reruns", value: String(asCount(metrics.rerun_like_runs)), tone: asCount(metrics.rerun_like_runs) ? "hot" : "ok", sub: "retry pressure" },
      { label: "Quarantine", value: String(asCount(metrics.quarantine_candidates)), tone: asCount(metrics.quarantine_candidates) ? "hot" : "ok", sub: "candidate jobs" },
      { label: "Completed", value: String(asCount(metrics.completed_runs)), tone: "sig", sub: `${asCount(metrics.successful_runs)} successful` },
    ];
  }
  const counts = overview?.counts || {};
  return [
    { label: "Scans", value: String(asCount(counts.scans || health?.scan_count)), tone: "sig", sub: `${asCount(counts.repos || health?.repo_count)} repos` },
    { label: "Saved signals", value: String(asCount(counts.flaky_signals || health?.flaky_signal_count)), tone: metricToneFromSignalCount(counts.flaky_signals || health?.flaky_signal_count), sub: "across saved scans" },
    { label: "Saved quarantine", value: String(asCount(counts.quarantine_candidates || health?.quarantine_candidate_count)), tone: asCount(counts.quarantine_candidates || health?.quarantine_candidate_count) ? "hot" : "ok", sub: "across saved scans" },
    { label: "GitHub", value: githubReady(health) ? "READY" : "MISSING", tone: githubReady(health) ? "ok" : "warn", sub: "Actions reads" },
    { label: "History", value: String(asCount(overview?.recent_scans?.length)), tone: "sig", sub: "recent runs" },
  ];
}

function workflowRailItems(scan, history) {
  const items = scan
    ? (scan.signals?.length
      ? scan.signals.map((signal) => ({
        label: signal.workflow_name || signal.job_name || signal.key,
        value: signal.status || signal.kind || "watch",
      }))
      : [{
        label: scan.workflow_name || "all matching workflows",
        value: "clear",
      }])
    : history.map((item) => ({
      label: item.workflow_name || item.repo,
      value: item.flaky_signals ? "watch" : "clear",
    }));
  const seen = new Set();

  return items
    .filter((item) => {
      const key = String(item.label || "").trim().toLocaleLowerCase();
      if (!key || seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .slice(0, 4)
    .map((item, index) => ({ ...item, active: index === 0 }));
}

function buildRail(scan, history, overview, health) {
  const latest = history[0] || {};
  const active = scan || latest;
  const activeMetrics = active.metrics || active;
  const savedHistory = !scan;
  return {
    sections: [
      {
        title: scan ? "Workflows" : "Recent workflows",
        items: workflowRailItems(scan, history),
      },
      {
        title: "Signals",
        items: [
          { label: savedHistory ? "saved signals" : "flaky signals", active: true, badge: String(asCount(savedHistory ? overview?.counts?.flaky_signals : activeMetrics.flaky_signals)), badgeTone: "amber" },
          { label: "rerun pressure", badge: String(asCount(activeMetrics.rerun_like_runs)), badgeTone: asCount(activeMetrics.rerun_like_runs) ? "red" : "green" },
          { label: savedHistory ? "saved quarantine" : "quarantine", badge: String(asCount(savedHistory ? overview?.counts?.quarantine_candidates : activeMetrics.quarantine_candidates)), badgeTone: asCount(savedHistory ? overview?.counts?.quarantine_candidates : activeMetrics.quarantine_candidates) ? "red" : "green" },
          { label: "saved scans", badge: String(history.length), badgeTone: "signal" },
        ],
      },
    ],
    stats: {
      title: scan ? "Active repo" : "Latest saved repo",
      items: [
        { label: "Repository", value: active.repo || "none" },
        { label: "CI trust", value: activeMetrics.quarantine_candidates ? "QUARANTINE" : activeMetrics.flaky_signals ? "WATCH" : "READY", large: true, tone: activeMetrics.quarantine_candidates ? "hot" : activeMetrics.flaky_signals ? "warn" : "ok" },
        { label: "Branch", value: active.branch || "default" },
      ],
    },
  };
}

function scanTone(scan) {
  if (asCount(scan?.metrics?.quarantine_candidates)) return "red";
  if (asCount(scan?.metrics?.flaky_signals) || asCount(scan?.metrics?.rerun_like_runs)) return "amber";
  return "green";
}

function supportItem(config, index) {
  return {
    detail: config.detail,
    gain: config.gain,
    gainMeta: config.gainMeta,
    id: config.id,
    label: config.label,
    minWindow: 7,
    position: POSITIONS[index % POSITIONS.length],
    stats: config.stats,
    summary: config.summary,
    title: config.title,
    tone: config.tone || "signal",
    vector: config.vector,
    vectorTone: config.vectorTone || "",
  };
}

function currentScanItem(scan) {
  const tone = scanTone(scan);
  return supportItem({
    detail: scan?.repo || "Current workflow scan",
    gain: asCount(scan?.metrics?.quarantine_candidates) ? "quarantine" : asCount(scan?.metrics?.flaky_signals) ? "watch" : "clear",
    gainMeta: `${asCount(scan?.metrics?.flaky_signals)} signals`,
    id: scan?.id || "current-flake-scan",
    label: "FS",
    stats: [
      { label: "Repo", value: scan?.repo || "repo" },
      { label: "Workflow", value: scan?.workflow_name || "all" },
      { label: "Runs", value: String(asCount(scan?.metrics?.workflow_runs)) },
      { label: "Signals", value: String(asCount(scan?.metrics?.flaky_signals)) },
      { label: "Quarantine", value: String(asCount(scan?.metrics?.quarantine_candidates)) },
    ],
    summary: scan?.summary || "FlakeSting current workflow scan.",
    title: "Current scan",
    tone,
    vector: "scan",
    vectorTone: tone === "red" || tone === "amber" ? "warn" : "",
  }, 0);
}

function supportingScanItems(scan) {
  const metrics = scan?.metrics || {};
  const failedRuns = asCount(metrics.failed_runs);
  const successfulRuns = asCount(metrics.successful_runs);
  const completedRuns = asCount(metrics.completed_runs);
  const reruns = asCount(metrics.rerun_like_runs);
  const trend = scan?.trend;
  const hasTrend = Boolean(trend?.status);
  const trendTone = trend?.status === "rising" ? "amber" : trend?.status === "improving" ? "green" : "signal";

  return [
    supportItem({
      detail: `${completedRuns} completed runs`,
      gain: completedRuns ? `${successfulRuns}/${completedRuns}` : "none",
      gainMeta: `${failedRuns} failed`,
      id: "run-outcome-signal",
      label: "RN",
      stats: [
        { label: "Completed", value: String(completedRuns) },
        { label: "Success", value: String(successfulRuns) },
        { label: "Failed", value: String(failedRuns) },
        { label: "Lookback", value: String(asCount(metrics.workflow_runs)) },
        { label: "Branch", value: scan?.branch || "default" },
      ],
      summary: failedRuns ? "Recent workflow history includes failed runs." : "Recent workflow runs are not showing failure pressure.",
      title: "Run outcomes",
      tone: failedRuns ? "amber" : "green",
      vector: "runs",
      vectorTone: failedRuns ? "warn" : "",
    }, 1),
    supportItem({
      detail: `${reruns} rerun-like runs`,
      gain: reruns ? "pressure" : "quiet",
      gainMeta: "retry pressure",
      id: "rerun-pressure-signal",
      label: "RR",
      stats: [
        { label: "Reruns", value: String(reruns) },
        { label: "Failed", value: String(failedRuns) },
        { label: "Signals", value: String(asCount(metrics.flaky_signals)) },
        { label: "Quarantine", value: String(asCount(metrics.quarantine_candidates)) },
        { label: "State", value: reruns ? "watch" : "clear" },
      ],
      summary: reruns ? "Rerun pressure is present, which can point to unstable CI." : "No retry pressure is visible in this scan.",
      title: "Rerun pressure",
      tone: reruns ? "amber" : "green",
      vector: "reruns",
      vectorTone: reruns ? "warn" : "",
    }, 2),
    supportItem({
      detail: hasTrend ? trend.status : "first comparable scan",
      gain: hasTrend ? trend.status : "baseline",
      gainMeta: hasTrend ? `${trend.flaky_signal_delta > 0 ? "+" : ""}${trend.flaky_signal_delta} signals` : "trend pending",
      id: hasTrend ? "trend-signal" : "baseline-signal",
      label: hasTrend ? "TR" : "BL",
      stats: [
        { label: "Status", value: hasTrend ? trend.status : "baseline" },
        { label: "Signal delta", value: String(trend?.flaky_signal_delta || 0) },
        { label: "Quarantine delta", value: String(trend?.quarantine_delta || 0) },
        { label: "New", value: String(trend?.new_signal_count || 0) },
        { label: "Cleared", value: String(trend?.cleared_signal_count || 0) },
      ],
      summary: hasTrend ? `Trend is ${trend.status} compared with the prior comparable scan.` : "Baseline saved. A trend appears after the next comparable scan.",
      title: hasTrend ? "Trend" : "Baseline",
      tone: trendTone,
      vector: hasTrend ? "trend" : "baseline",
      vectorTone: trendTone === "amber" ? "warn" : "",
    }, 3),
  ];
}

function buildRadarItems(scan, history) {
  if (scan) {
    const support = [currentScanItem(scan), ...supportingScanItems(scan)];
    const signalItems = (scan.signals || []).slice(0, Math.max(0, 8 - support.length)).map((signal, index) => ({
      detail: signal.job_name || signal.workflow_name || signal.key,
      gain: signal.status || signal.kind || "signal",
      gainMeta: `${asCount(signal.score)} score`,
      id: signal.key || `flake-${index + 1}`,
      label: signal.status === "quarantine" ? `Q${index + 1}` : `F${index + 1}`,
      minWindow: 7,
      position: POSITIONS[(support.length + index) % POSITIONS.length],
      stats: [
        { label: "Kind", value: signal.kind || "flake" },
        { label: "Status", value: signal.status || "watch" },
        { label: "Score", value: String(asCount(signal.score)) },
        { label: "Fail", value: String(asCount(signal.failure_count)) },
        { label: "Reruns", value: String(asCount(signal.rerun_hits)) },
      ],
      summary: signal.summary || signal.evidence?.[0] || "FlakeSting signal.",
      title: signal.step_name || signal.job_name || signal.workflow_name || signal.key,
      tone: signalTone(signal),
      vector: signal.kind || signal.workflow_name || "ci",
      vectorTone: signalTone(signal) === "red" || signalTone(signal) === "amber" ? "warn" : "",
    }));
    return [...support, ...signalItems];
  }
  if (history.length) {
    return history.map((item, index) => {
      const minWindow = radarWindowFromTimestamp(item.created_at);
      if (!minWindow) {
        return null;
      }
      return {
        detail: item.repo,
        gain: item.quarantine_candidates ? "quarantine" : item.flaky_signals ? "watch" : "clear",
        gainMeta: `${asCount(item.flaky_signals)} signals`,
        id: item.id || `history-${index + 1}`,
        label: item.workflow_name || item.repo?.split("/").pop() || `S${index + 1}`,
        minWindow,
        position: POSITIONS[index % POSITIONS.length],
        stats: [
          { label: "Repo", value: item.repo },
          { label: "Workflow", value: item.workflow_name || "all" },
          { label: "Flaky", value: String(asCount(item.flaky_signals)) },
          { label: "Quarantine", value: String(asCount(item.quarantine_candidates)) },
          { label: "Age", value: timeAgo(item.created_at) },
        ],
        summary: item.summary || "Saved FlakeSting scan.",
        title: item.workflow_name || item.repo,
        tone: item.quarantine_candidates ? "red" : item.flaky_signals ? "amber" : "green",
        vector: item.trend?.status || "saved",
        vectorTone: item.quarantine_candidates || item.flaky_signals ? "warn" : "",
      };
    }).filter(Boolean);
  }
  return [{
    detail: "No workflow scan yet",
    gain: "standby",
    gainMeta: "GitHub Actions",
    id: "flake-sting-ready",
    label: "FS",
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Mode", value: "read only" },
      { label: "Actions", value: "ready" },
      { label: "History", value: "empty" },
      { label: "Reruns", value: "unknown" },
      { label: "Action", value: "scan" },
    ],
    summary: "Scan GitHub Actions history to populate FlakeSting's live instability radar.",
    title: "FlakeSting ready",
    tone: "signal",
    vector: "READY",
  }];
}

function buildRadarFeed(scan, history, health) {
  if (scan) {
    const metrics = scan.metrics || {};
    const runCount = asCount(metrics.workflow_runs);
    return [
      { text: runCount ? `${runCount} GitHub Actions runs inspected for ${scan.repo || "the selected repo"}.` : `No matching completed GitHub Actions runs were available for ${scan.repo || "the selected repo"}.`, tone: runCount ? "signal" : "amber" },
      { text: `${asCount(metrics.flaky_signals)} flaky signals and ${asCount(metrics.quarantine_candidates)} quarantine candidates are active.`, tone: metrics.quarantine_candidates ? "red" : metrics.flaky_signals ? "amber" : "green" },
      { text: scan.trend?.status ? `Trend is ${scan.trend.status}.` : "Baseline saved. A trend appears after the next comparable scan.", tone: scan.trend?.status === "rising" ? "amber" : "signal" },
    ];
  }
  return [
    { text: history.length ? `${history.length} saved CI scans are available.` : "FlakeSting is waiting for a workflow scan.", tone: "signal" },
    { text: githubReady(health) ? "GitHub token is ready for Actions history reads." : "Configure GitHub token access before live scans.", tone: githubReady(health) ? "green" : "amber" },
    { text: "The radar fills with job and step instability signals once a scan completes.", tone: "signal" },
  ];
}

function StatusBanner({ tone = "signal", children }) {
  if (!children) return null;
  return <div className={`status-banner ${tone}`}>{children}</div>;
}

function InstabilityMap({ health, history, scan }) {
  const items = useMemo(() => buildRadarItems(scan, history), [scan, history]);
  const feed = useMemo(() => buildRadarFeed(scan, history, health), [scan, history, health]);
  return (
    <SuiteRadar
      ariaLabel="FlakeSting CI instability radar"
      detailLabel="Instability reason"
      feed={feed}
      gainLabel="State"
      itemQueryParam="flake"
      items={items}
      countLabel={scan ? `${asCount(scan.metrics?.flaky_signals)} ${asCount(scan.metrics?.flaky_signals) === 1 ? "signal" : "signals"}` : ""}
      signalLabel={scan ? "signals" : "scans"}
      vectorLabel={scan ? "Selected CI context" : "Selected scan"}
    />
  );
}

function ScanForm({ error, form, onChange, onRun, running }) {
  const set = (key, value) => onChange((current) => ({ ...current, [key]: value }));
  return (
    <Panel eyebrow="Scan" title="GitHub Actions intake" action={<span className="chip signal">read only</span>}>
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
            <input className="v2-input" onChange={(event) => set("branch", event.target.value)} placeholder="optional" value={form.branch} />
          </label>
          <label className="v2-field">
            Workflow
            <input className="v2-input" onChange={(event) => set("workflow_name", event.target.value)} placeholder="optional" value={form.workflow_name} />
          </label>
          <label className="v2-field">
            Lookback runs
            <input className="v2-input" min="5" max="40" onChange={(event) => set("lookback_runs", event.target.value)} type="number" value={form.lookback_runs} />
          </label>
        </div>
        <button className="btn primary" disabled={running || !form.repo.trim()} type="submit">
          {running ? "Scanning..." : "Scan workflow"}
        </button>
        {error && <StatusBanner tone="red">{error}</StatusBanner>}
      </form>
    </Panel>
  );
}

function TrendPanel({ onLoadScan, scan }) {
  const trend = scan?.trend;
  if (!trend) return null;

  const previousScanId = trend.compared_to_scan_id;
  const newSignals = trend.new_signals || [];
  const clearedSignals = trend.cleared_signals || [];
  return (
    <Panel eyebrow="Trend" title="Change since comparable scan" action={<span className={`chip ${trendTone(trend.status)}`}>{trend.status || "baseline"}</span>}>
      <div className="panelbody repo-list">
        <div className="repo-meta">
          <span className="chip amber">signals {signedCount(trend.flaky_signal_delta)}</span>
          <span className="chip red">quarantine {signedCount(trend.quarantine_delta)}</span>
          <span className="chip signal">reruns {signedCount(trend.rerun_delta)}</span>
          <span className="chip amber">{asCount(trend.new_signal_count)} new</span>
          <span className="chip green">{asCount(trend.cleared_signal_count)} cleared</span>
        </div>
        <div className="feed-meta">{trend.compared_to_created_at ? `Compared with ${timeAgo(trend.compared_to_created_at)}.` : "Baseline saved. A trend appears after the next comparable scan."}</div>
        {(newSignals.length > 0 || clearedSignals.length > 0) && (
          <div className="repo-list" style={{ display: "grid", gap: 8 }}>
            {newSignals.slice(0, 3).map((item, index) => <div className="feed-meta" key={`new-${index}`}>| New: {item}</div>)}
            {clearedSignals.slice(0, 3).map((item, index) => <div className="feed-meta" key={`cleared-${index}`}>| Cleared: {item}</div>)}
          </div>
        )}
        {previousScanId && <button className="btn" onClick={() => onLoadScan(previousScanId)} type="button">Load previous</button>}
      </div>
    </Panel>
  );
}

function FlakyQueuePanel({ history, onLoadScan, scan }) {
  const [copyState, setCopyState] = useState("");
  const [sortBy, setSortBy] = useState("risk");
  const signals = useMemo(() => sortSignals(scan?.signals || [], sortBy), [scan, sortBy]);

  async function copySummary() {
    if (!scan || !navigator?.clipboard?.writeText) return;
    try {
      await navigator.clipboard.writeText(buildScanMarkdown(scan));
      setCopyState("Copied");
    } catch {
      setCopyState("Copy failed");
    }
    window.setTimeout(() => setCopyState(""), 1800);
  }

  if (scan) {
    return (
      <Panel
        eyebrow="Queue"
        title="Flaky candidates"
        action={(
          <div className="actions">
            <button className="btn" onClick={copySummary} type="button">{copyState || "Copy summary"}</button>
            <select aria-label="Sort flaky candidates" className="v2-input" onChange={(event) => setSortBy(event.target.value)} style={{ minWidth: 150 }} value={sortBy}>
              {SORT_OPTIONS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
            </select>
            <span className={`chip ${scan.metrics?.quarantine_candidates ? "red" : "amber"}`}>{asCount(scan.metrics?.quarantine_candidates)} quarantine</span>
          </div>
        )}
      >
        <div className="panelbody repo-list queue-grid">
          {signals.length ? signals.slice(0, 8).map((signal, index) => (
            <div className="ledger-row" key={signal.key || index}>
              <div className="rank">{String(index + 1).padStart(2, "0")}</div>
              <div>
                <div className="repo-name">{signal.step_name || signal.job_name || signal.workflow_name}</div>
                <div className="feed-meta">{signal.summary || signal.evidence?.[0]}</div>
                <div className="repo-meta">
                  <span className={`chip ${signalTone(signal)}`}>{signal.status || signal.kind}</span>
                  <span className="chip amber">{asCount(signal.failure_count)} fail</span>
                  <span className="chip green">{asCount(signal.success_count)} pass</span>
                  {asCount(signal.rerun_hits) > 0 && <span className="chip signal">{asCount(signal.rerun_hits)} rerun</span>}
                </div>
                {signal.environment_hints?.length > 0 && <div className="feed-meta">{signal.environment_hints.slice(0, 2).join(" | ")}</div>}
                {(signal.evidence || []).slice(0, 2).map((line, evidenceIndex) => {
                  const url = evidenceUrl(line);
                  return url ? <a className="chip signal" href={url} key={`${signal.key || index}-${evidenceIndex}`} rel="noreferrer" target="_blank">Open run</a> : null;
                })}
              </div>
              <span className={`chip ${signalTone(signal)}`}>{asCount(signal.score)}</span>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No flaky signals</strong>
              <span>This workflow scan did not find unstable jobs or steps.</span>
            </div>
          )}
        </div>
      </Panel>
    );
  }
  return (
    <Panel eyebrow="Queue" title="Recent scans" action={<span className="chip signal">{history.length} saved</span>}>
      <div className="panelbody repo-list queue-grid">
        {history.length ? history.slice(0, 5).map((item, index) => (
          <div className="ledger-row" key={item.id}>
            <div className="rank">{String(index + 1).padStart(2, "0")}</div>
            <div>
              <div className="repo-name">{item.repo}</div>
              <div className="feed-meta">{item.summary || item.workflow_name || "Saved FlakeSting scan."}</div>
              <div className="repo-meta">
                <span className="chip amber">{asCount(item.flaky_signals)} flaky</span>
                <span className="chip red">{asCount(item.quarantine_candidates)} quarantine</span>
                <span className="chip signal">{item.trend?.status || "baseline"}</span>
                <span className="chip">{timeAgo(item.created_at)}</span>
              </div>
            </div>
            <button className="btn" onClick={() => onLoadScan(item.id)} type="button">Load</button>
          </div>
        )) : (
          <div className="empty-v2">
            <strong>No scans yet</strong>
            <span>Scan GitHub Actions history to populate the queue.</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function SidePanels({ health, scan }) {
  const evidence = (scan?.signals || []).flatMap((signal) => (signal.evidence || []).slice(0, 1).map((text) => ({ text, signal }))).slice(0, 3);
  return (
    <aside className="side">
      <Panel eyebrow="Evidence" title={evidence.length ? "Why it looks flaky" : "Evidence status"}>
        <div className="panelbody repo-list">
          {evidence.length ? evidence.map((item) => (
            <div className="feed-item" key={`${item.signal.key}-${item.text}`}>
              <div>
                <div className="feed-title">{item.signal.job_name || item.signal.workflow_name}</div>
                <div className="feed-meta">
                  {item.text}
                  {evidenceUrl(item.text) && <> <a href={evidenceUrl(item.text)} rel="noreferrer" target="_blank">Open run</a></>}
                </div>
              </div>
              <span className={`chip ${signalTone(item.signal)}`}>{item.signal.status || "signal"}</span>
            </div>
          )) : (
            <div className="rowline"><span className="muted">Flaky evidence</span><span className="chip green">none</span></div>
          )}
          <div className="rowline"><span className="muted">GitHub token</span><span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "ready" : "missing"}</span></div>
        </div>
      </Panel>
      <Panel eyebrow="Consumers" title="Signal handoff">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">MergeKeeper</span><span className={`chip ${scan?.metrics?.flaky_signals ? "amber" : "green"}`}>{scan?.metrics?.flaky_signals ? "caution" : "clear"}</span></div>
          <div className="rowline"><span className="muted">ReleaseSentry</span><span className={`chip ${scan?.metrics?.quarantine_candidates ? "amber" : "signal"}`}>{scan?.metrics?.quarantine_candidates ? "watch" : "context"}</span></div>
          <div className="rowline"><span className="muted">Human action</span><span className={`chip ${scan?.metrics?.quarantine_candidates ? "red" : "green"}`}>{scan?.metrics?.quarantine_candidates ? "quarantine" : "none"}</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function InstabilitySurface({
  error,
  form,
  health,
  history,
  onChangeForm,
  onClearScan,
  onLoadScan,
  onRefresh,
  onRunScan,
  overview,
  running,
  scan,
}) {
  const rail = useMemo(() => buildRail(scan, history, overview, health), [scan, history, overview, health]);
  const metrics = useMemo(() => buildMetrics(scan, overview, health), [scan, overview, health]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, overview, scan, history)} />
      <div className="main-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - CI trust</div>
              <h1>Instability Map</h1>
              <p className="subline">Workflow history, reruns, runner skew, and fail/pass swings turned into readable CI trust pressure.</p>
            </div>
            <div className="actions">
              <span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "github ready" : "github missing"}</span>
              {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
          </div>
          <ScanForm error={error} form={form} onChange={onChangeForm} onRun={onRunScan} running={running} />
          <MetricBand metrics={metrics} />
          <TrendPanel onLoadScan={onLoadScan} scan={scan} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Instability" title="CI signal map" action={<span className="chip signal">ci radar</span>}>
              <InstabilityMap health={health} history={history} scan={scan} />
            </Panel>
            <FlakyQueuePanel history={history} onLoadScan={onLoadScan} scan={scan} />
          </div>
        </main>
        <SidePanels health={health} scan={scan} />
      </div>
    </>
  );
}

function SecondaryFrame({ children, health, history, overview, scan }) {
  const rail = useMemo(() => buildRail(scan, history, overview, health), [scan, history, overview, health]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, overview, scan, history)} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">{children}</main>
      </div>
    </>
  );
}

function HistorySurface({ activeScanId, health, history, loading, onClearScan, onLoadScan, onRefresh, overview, scan }) {
  const [query, setQuery] = useState("");
  const filteredHistory = history.filter((item) => {
    const needle = query.trim().toLowerCase();
    if (!needle) return true;
    return [item.repo, item.workflow_name, item.summary].some((value) => String(value || "").toLowerCase().includes(needle));
  });
  return (
    <SecondaryFrame health={health} history={history} overview={overview} scan={scan}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// FlakeSting instability queue</div>
          <h1>Scan History</h1>
          <p className="subline">Comparable CI scans and whether trust pressure is rising or improving.</p>
        </div>
        <div className="actions">
          {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
          <button className="btn" onClick={onRefresh} type="button">{loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      {scan && (
        <HistoryDetailGrid>
          <Panel eyebrow="Instability" title="Selected CI signal map" action={<span className="chip signal">ci radar</span>}>
            <InstabilityMap health={health} history={history} scan={scan} />
          </Panel>
          <FlakyQueuePanel history={history} onLoadScan={onLoadScan} scan={scan} />
        </HistoryDetailGrid>
      )}
      {scan && <TrendPanel onLoadScan={onLoadScan} scan={scan} />}
      <Panel
        eyebrow="Recent"
        title="Workflow scans"
        action={<span className="chip signal">{history.length} saved</span>}
      >
        <div className="panelbody repo-list queue-grid">
          <label className="v2-field" style={{ maxWidth: 360 }}>
            Filter scans {query ? `(${filteredHistory.length} shown)` : ""}
            <input aria-label="Filter workflow scans" className="v2-input" onChange={(event) => setQuery(event.target.value)} placeholder="repo, workflow, summary" value={query} />
          </label>
          {filteredHistory.length ? filteredHistory.map((item, index) => (
            <div className="ledger-row" key={item.id}>
              <div className="rank">{item.id === activeScanId ? "SEL" : String(index + 1).padStart(2, "0")}</div>
              <div>
                <div className="repo-name">{item.repo}</div>
                <div className="feed-meta">{item.summary || item.workflow_name || "Saved FlakeSting scan."}</div>
                <div className="repo-meta">
                  <span className="chip amber">{asCount(item.flaky_signals)} flaky</span>
                  <span className="chip red">{asCount(item.quarantine_candidates)} quarantine</span>
                  <span className="chip signal">{item.trend?.status || "baseline"}</span>
                  <span className="chip">{timeAgo(item.created_at)}</span>
                </div>
              </div>
              <button className="btn" onClick={() => onLoadScan(item.id)} type="button">Load</button>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>{history.length ? "No scans match that filter" : "No scans saved"}</strong>
              <span>{history.length ? "Try a repository, workflow, or summary term." : "Scan a workflow to create CI trust history."}</span>
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

function ChecksSurface({ history, onClearScan, overview, runtime, scan }) {
  const health = runtime.health || {};
  const checks = runtime.checks || [];
  const warnings = checks.filter((check) => check.level === "warn" || check.level === "error").length;
  const metrics = [
    { label: "Status", value: health.status || "unknown", tone: health.status === "ok" ? "ok" : "warn", sub: health.version || "backend" },
    { label: "GitHub", value: githubReady(health) ? "ready" : "missing", tone: githubReady(health) ? "ok" : "hot", sub: "Actions reads" },
    { label: "Scans", value: String(asCount(health.scan_count || overview?.counts?.scans)), tone: "sig", sub: `${asCount(health.repo_count || overview?.counts?.repos)} repos` },
    { label: "Flaky", value: String(asCount(health.flaky_signal_count || overview?.counts?.flaky_signals)), tone: metricToneFromSignalCount(health.flaky_signal_count || overview?.counts?.flaky_signals), sub: "signals" },
    { label: "Checks", value: warnings ? String(warnings) : "clear", tone: warnings ? "warn" : "ok", sub: "startup" },
  ];
  return (
    <SecondaryFrame health={health} history={history} overview={overview} scan={scan}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// FlakeSting readiness</div>
          <h1>Checks</h1>
          <p className="subline">Backend health, GitHub Actions access, database state, and startup checks.</p>
        </div>
        <div className="actions">
          <button className="btn" onClick={runtime.refresh} type="button">{runtime.loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      {runtime.error && <StatusBanner tone="red">{runtime.error}</StatusBanner>}
      <MetricBand metrics={metrics} />
      <div className="atlas-layout suite-four-layout">
        <Panel eyebrow="Health" title="Backend status" action={<span className={`chip ${health.status === "ok" ? "green" : "amber"}`}>{health.status || "unknown"}</span>}>
          <div className="panelbody repo-list">
            <div className="rowline"><span className="muted">Auth enabled</span><span className={`chip ${health.auth_enabled ? "green" : "amber"}`}>{health.auth_enabled ? "yes" : "no"}</span></div>
            <div className="rowline"><span className="muted">GitHub token</span><span className={`chip ${githubReady(health) ? "green" : "red"}`}>{githubReady(health) ? "ready" : "missing"}</span></div>
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
  const [activeTab, setActiveTab] = usePersistentProductTab("flake-sting", TABS, "instability");
  const [error, setError] = useState("");
  const [form, setForm] = useState(DEFAULT_FORM);
  const [history, setHistory] = useState([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [overview, setOverview] = useState(null);
  const [running, setRunning] = useState(false);
  const [scan, setScan] = useState(null);
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "flake-sting_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

  async function fetchJson(path, options, fallbackError) {
    const response = await fetch_(`${API}${path}`, options);
    return parseJsonResponse(response, fallbackError);
  }

  async function refreshFlakeData() {
    if (!ready) return;
    setLoadingHistory(true);
    const [overviewResult, historyResult] = await Promise.allSettled([
      fetchJson("/overview", undefined, "FlakeSting could not load overview."),
      fetchJson("/history", undefined, "FlakeSting could not load history."),
    ]);
    setOverview(overviewResult.status === "fulfilled" ? overviewResult.value : null);
    setHistory(historyResult.status === "fulfilled" ? historyResult.value || [] : []);
    setLoadingHistory(false);
    const failed = [overviewResult, historyResult].find((result) => result.status === "rejected");
    if (failed) {
      setError(failed.reason?.message || "FlakeSting could not load one or more backend resources.");
    }
  }

  useEffect(() => {
    refreshFlakeData();
  }, [ready, fetch_]);

  async function runScan() {
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(
        "/scan/github/actions",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            repo: form.repo,
            branch: form.branch,
            workflow_name: form.workflow_name,
            lookback_runs: Number(form.lookback_runs) || 25,
          }),
        },
        "FlakeSting could not scan that workflow.",
      );
      setScan(result);
      setForm((current) => ({
        ...current,
        branch: result.branch || current.branch,
        repo: result.repo || current.repo,
        workflow_name: result.workflow_name || current.workflow_name,
      }));
      setActiveTab("instability");
      await refreshFlakeData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "FlakeSting could not scan that workflow.");
    } finally {
      setRunning(false);
    }
  }

  async function loadScan(id) {
    if (!id) return;
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(`/history/${id}`, undefined, "FlakeSting could not load that scan.");
      setScan(result);
      setForm((current) => ({
        ...current,
        branch: result.branch || current.branch,
        repo: result.repo || current.repo,
        workflow_name: result.workflow_name || current.workflow_name,
      }));
    } catch (err) {
      setError(err.message || "FlakeSting could not load that scan.");
    } finally {
      setRunning(false);
    }
  }

  function clearScan() {
    setScan(null);
    setError("");
  }

  if (!ready) {
    return (
      <ProductV2AuthGate
        apiBase={API}
        auth={auth}
        keyPrefix="flake-sting-"
        productKey="flake-sting"
        productName="FlakeSting"
      />
    );
  }

  return (
    <ProductV2Shell authConfigured={authConfigured} productKey="flake-sting" productName="FlakeSting" runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="FlakeSting"
        navLabel="FlakeSting navigation"
        onTabChange={setActiveTab}
        productKey="flake-sting"
        tabs={TABS}
      />
      {activeTab === "instability" && (
        <InstabilitySurface
          error={error}
          form={form}
          health={runtime.health || {}}
          history={history}
          onChangeForm={setForm}
          onClearScan={clearScan}
          onLoadScan={loadScan}
          onRefresh={() => {
            refreshFlakeData();
            runtime.refresh();
          }}
          onRunScan={runScan}
          overview={overview}
          running={running}
          scan={scan}
        />
      )}
      {activeTab === "history" && (
        <HistorySurface
          activeScanId={scan?.id || ""}
          health={runtime.health || {}}
          history={history}
          loading={loadingHistory}
          onClearScan={clearScan}
          onLoadScan={loadScan}
          onRefresh={refreshFlakeData}
          overview={overview}
          scan={scan}
        />
      )}
      {activeTab === "checks" && <ChecksSurface history={history} onClearScan={clearScan} overview={overview} runtime={runtime} scan={scan} />}
    </ProductV2Shell>
  );
}

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
  { id: "triage", label: "Triage" },
  { id: "history", label: "Scan history" },
  { id: "checks", label: "Checks" },
];

const POSITIONS = [
  { left: "57%", top: "25%" },
  { left: "73%", top: "46%" },
  { left: "38%", top: "34%" },
  { left: "31%", top: "69%" },
  { left: "63%", top: "73%" },
  { left: "46%", top: "55%" },
  { left: "68%", top: "61%" },
  { left: "28%", top: "42%" },
];

const DEFAULT_FORM = {
  repo: "",
  pr_limit: "25",
  include_alerts: true,
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

function recommendationTone(recommendation) {
  const value = String(recommendation || "").toLowerCase();
  if (value.includes("now") || value.includes("fix")) return "red";
  if (value.includes("watch") || value.includes("batch")) return "amber";
  if (value.includes("ignore") || value.includes("defer")) return "green";
  return "signal";
}

function recommendationLabel(recommendation) {
  return String(recommendation || "watch").replaceAll("_", " ");
}

function summaryLabel(summary) {
  const softened = String(summary || "Saved dependency triage scan.").replace(
    /([A-Za-z0-9_@./:-]+(?: [A-Za-z0-9_@./:-]+)*) is currently marked `ignore for now` because DepTriage saw ([^.]+)\./g,
    "$1 is a safe defer for now after DepTriage saw $2."
  );
  return softened.includes("0 update now, 0 watch")
    ? softened.replaceAll("Highest urgency:", "Top defer:")
    : softened;
}

function metricTone(recommendation) {
  const tone = recommendationTone(recommendation);
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

function buildTopline(health, overview, scan, history) {
  const counts = overview?.counts || {};
  const latest = scan || history[0] || {};
  return [
    { label: "DepTriage", value: "Dependency queue", tone: "sig" },
    { label: "System", value: health?.status || "checking", tone: health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: "Read only" },
    { label: "GitHub", value: githubReady(health) ? "PR + alert read" : "token missing", tone: githubReady(health) ? "sig" : "warn" },
    { label: "Alerts", value: `${asCount(scan?.metrics?.open_alerts || counts.open_alerts || health?.open_alerts)} open`, tone: "warn" },
    { label: "Last scan", value: latest.created_at ? timeAgo(latest.created_at) : counts.scans ? "loaded" : "none" },
  ];
}

function buildMetrics(scan, overview, health) {
  if (scan) {
    const metrics = scan.metrics || {};
    return [
      { label: "Update now", value: String(asCount(metrics.update_now)), tone: asCount(metrics.update_now) ? "hot" : "ok", sub: "highest urgency" },
      { label: "Watch", value: String(asCount(metrics.watch)), tone: asCount(metrics.watch) ? "warn" : "ok", sub: "monitor or batch" },
      { label: "Alerts", value: String(asCount(metrics.open_alerts)), tone: asCount(metrics.open_alerts) ? "hot" : "ok", sub: "Dependabot" },
      { label: "Safe defers", value: String(asCount(metrics.ignore_for_now)), tone: "ok", sub: "low-value churn" },
      { label: "Tracked", value: String(asCount(metrics.tracked_items)), tone: "sig", sub: `${asCount(metrics.dependency_pull_requests)} dep PRs` },
    ];
  }
  const counts = overview?.counts || {};
  return [
    { label: "Scans", value: String(asCount(counts.scans || health?.scan_count)), tone: "sig", sub: `${asCount(counts.repos || health?.repo_count)} repos` },
    { label: "Update now", value: String(asCount(counts.update_now || health?.update_now_count)), tone: "hot", sub: "saved scans" },
    { label: "Watch", value: String(asCount(counts.watch || health?.watch_count)), tone: "warn", sub: "saved scans" },
    { label: "Ignore", value: String(asCount(counts.ignore_for_now || health?.ignore_count)), tone: "ok", sub: "saved scans" },
    { label: "Tracked", value: String(asCount(counts.tracked_items || health?.tracked_item_count)), tone: "sig", sub: "items" },
  ];
}

function buildRail(scan, history, overview, health) {
  const latest = history[0] || {};
  return {
    sections: [
      {
        title: "Sources",
        items: [
          { label: "dependency PRs", active: true, badge: String(asCount(scan?.metrics?.dependency_pull_requests)), badgeTone: "signal" },
          { label: "Dependabot alerts", badge: String(asCount(scan?.metrics?.open_alerts || health?.open_alerts)), badgeTone: asCount(scan?.metrics?.open_alerts || health?.open_alerts) ? "red" : "green" },
          { label: "runtime updates", badge: String(asCount(scan?.metrics?.runtime_updates)), badgeTone: "amber" },
          { label: "major updates", badge: String(asCount(scan?.metrics?.major_updates)), badgeTone: "amber" },
        ],
      },
      {
        title: "Buckets",
        items: [
          { label: "update now", active: true, badge: String(asCount(scan?.metrics?.update_now || overview?.counts?.update_now)), badgeTone: "red" },
          { label: "watch", badge: String(asCount(scan?.metrics?.watch || overview?.counts?.watch)), badgeTone: "amber" },
          { label: "ignore for now", badge: String(asCount(scan?.metrics?.ignore_for_now || overview?.counts?.ignore_for_now)), badgeTone: "green" },
          { label: "saved scans", badge: String(history.length), badgeTone: "signal" },
        ],
      },
    ],
    stats: {
      title: "Active repo",
      items: [
        { label: "Repository", value: scan?.repo || latest.repo || "none" },
        { label: "Decision", value: scan?.metrics?.update_now ? "UPDATE" : scan?.metrics?.watch ? "WATCH" : "READY", large: true, tone: scan?.metrics?.update_now ? "hot" : scan?.metrics?.watch ? "warn" : "ok" },
        { label: "Items", value: String(asCount(scan?.metrics?.tracked_items || latest.tracked_items)) },
      ],
    },
  };
}

function buildRadarItems(scan, history) {
  if (scan?.items?.length) {
    return scan.items.slice(0, 8).map((item, index) => ({
      detail: item.package_name || item.key,
      gain: recommendationLabel(item.recommendation),
      gainMeta: `${asCount(item.score)} score`,
      id: item.key || `dep-${index + 1}`,
      label: item.package_name || `D${index + 1}`,
      minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
      position: POSITIONS[index % POSITIONS.length],
      stats: [
        { label: "Package", value: item.package_name || "package" },
        { label: "Decision", value: recommendationLabel(item.recommendation) },
        { label: "Score", value: String(asCount(item.score)) },
        { label: "Source", value: item.source || "github" },
        { label: "Alerts", value: String(item.alerts?.length || 0) },
      ],
      summary: summaryLabel(item.summary || item.reasons?.[0] || "Dependency triage item."),
      title: item.package_name || item.key || `Dependency ${index + 1}`,
      tone: recommendationTone(item.recommendation),
      vector: item.update_kind || item.ecosystem || item.source || "dependency",
      vectorTone: recommendationTone(item.recommendation) === "red" || recommendationTone(item.recommendation) === "amber" ? "warn" : "",
    }));
  }
  if (scan) {
    const metrics = scan.metrics || {};
    const hasTriagePressure = asCount(metrics.update_now) || asCount(metrics.watch) || asCount(metrics.tracked_items);
    return [{
      detail: scan.repo,
      gain: metrics.update_now ? "update now" : metrics.watch ? "watch" : hasTriagePressure ? "ignore" : "clear",
      gainMeta: `${asCount(metrics.tracked_items)} items`,
      id: scan.id || "dep-triage-scan",
      label: scan.repo?.split("/").pop() || "scan",
      minWindow: 7,
      position: POSITIONS[0],
      stats: [
        { label: "Repo", value: scan.repo },
        { label: "Tracked", value: String(asCount(metrics.tracked_items)) },
        { label: "Now", value: String(asCount(metrics.update_now)) },
        { label: "Watch", value: String(asCount(metrics.watch)) },
        { label: "Age", value: timeAgo(scan.created_at) },
      ],
      summary: summaryLabel(scan.summary),
      title: scan.repo,
      tone: metrics.update_now ? "red" : metrics.watch ? "amber" : "green",
      vector: hasTriagePressure ? "triage" : "clear",
      vectorTone: metrics.update_now || metrics.watch ? "warn" : "",
    }];
  }
  if (history.length) {
    return history.map((item, index) => {
      const minWindow = radarWindowFromTimestamp(item.created_at);
      if (!minWindow) {
        return null;
      }
      return {
        detail: item.repo,
        gain: item.update_now ? "update now" : item.watch ? "watch" : item.tracked_items ? "ignore" : "clear",
        gainMeta: `${asCount(item.tracked_items)} items`,
        id: item.id || `history-${index + 1}`,
        label: item.repo?.split("/").pop() || `S${index + 1}`,
        minWindow,
        position: POSITIONS[index % POSITIONS.length],
        stats: [
          { label: "Repo", value: item.repo },
          { label: "Tracked", value: String(asCount(item.tracked_items)) },
          { label: "Now", value: String(asCount(item.update_now)) },
          { label: "Watch", value: String(asCount(item.watch)) },
          { label: "Age", value: timeAgo(item.created_at) },
        ],
        summary: summaryLabel(item.summary),
        title: item.repo,
        tone: item.update_now ? "red" : item.watch ? "amber" : "green",
        vector: "saved",
        vectorTone: item.update_now || item.watch ? "warn" : "",
      };
    }).filter(Boolean);
  }
  return [{
    detail: "No dependency scan yet",
    gain: "standby",
    gainMeta: "GitHub repo",
    id: "dep-triage-ready",
    label: "DT",
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Mode", value: "read only" },
      { label: "PRs", value: "ready" },
      { label: "Alerts", value: "optional" },
      { label: "History", value: "empty" },
      { label: "Action", value: "scan" },
    ],
    summary: "Scan a repository to populate DepTriage with live dependency PR and alert data.",
    title: "DepTriage ready",
    tone: "signal",
    vector: "READY",
  }];
}

function buildRadarFeed(scan, history, health) {
  if (scan) {
    return [
      { text: `${asCount(scan.metrics?.update_now)} update-now items and ${asCount(scan.metrics?.watch)} watch items are active.`, tone: scan.metrics?.update_now ? "red" : scan.metrics?.watch ? "amber" : "green" },
      { text: scan.warnings?.[0] || "Dependency PRs and alerts are ranked into actionable buckets.", tone: scan.warnings?.length ? "amber" : "signal" },
    ];
  }
  return [
    { text: history.length ? `${history.length} saved dependency scans are available.` : "DepTriage is waiting for a repository scan.", tone: "signal" },
    { text: githubReady(health) ? "GitHub token is ready for dependency PR and alert reads." : "Public PR scans work without a token; add GitHub access for Dependabot alerts and stronger rate limits.", tone: githubReady(health) ? "green" : "amber" },
    { text: "The radar fills with package-level decisions once a scan completes.", tone: "signal" },
  ];
}

function StatusBanner({ tone = "signal", children }) {
  if (!children) return null;
  return <div className={`status-banner ${tone}`}>{children}</div>;
}

function DependencyMap({ health, history, scan }) {
  const items = useMemo(() => buildRadarItems(scan, history), [scan, history]);
  const feed = useMemo(() => buildRadarFeed(scan, history, health), [scan, history, health]);
  const hasPackageItems = Boolean(scan?.items?.length);
  return (
    <SuiteRadar
      ariaLabel="DepTriage dependency pressure radar"
      detailLabel="Triage reason"
      feed={feed}
      gainLabel="Decision"
      itemQueryParam="dependency"
      items={items}
      signalLabel={hasPackageItems ? "packages" : "scans"}
      vectorLabel={hasPackageItems ? "Selected package" : "Selected scan"}
    />
  );
}

function ScanForm({ error, form, onChange, onRun, running }) {
  return (
    <Panel eyebrow="Scan" title="GitHub dependency intake" action={<span className="chip signal">read only</span>}>
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
            PR limit
            <input className="v2-input" min="5" max="60" onChange={(event) => onChange((current) => ({ ...current, pr_limit: event.target.value }))} type="number" value={form.pr_limit} />
          </label>
          <label className="rowline" style={{ alignItems: "flex-start", justifyContent: "flex-start" }}>
            <input
              checked={Boolean(form.include_alerts)}
              onChange={(event) => onChange((current) => ({ ...current, include_alerts: event.target.checked }))}
              style={{ marginTop: 3 }}
              type="checkbox"
            />
            <span>
              <span className="repo-name" style={{ display: "block", fontSize: "0.8rem" }}>Include Dependabot alerts</span>
              <span className="feed-meta">Adds security urgency when alert access is available.</span>
            </span>
          </label>
          <div className="v2-field">
            Action
            <button className="btn primary" disabled={running || !form.repo.trim()} type="submit">
              {running ? "Scanning..." : "Scan dependencies"}
            </button>
          </div>
        </div>
        {error && <StatusBanner tone="red">{error}</StatusBanner>}
      </form>
    </Panel>
  );
}

function UpdateQueuePanel({ history, onLoadScan, scan }) {
  const items = scan?.items || [];
  if (scan) {
    return (
      <Panel eyebrow="Queue" title="Dependency decisions" action={<span className={`chip ${scan.metrics?.update_now ? "red" : "signal"}`}>{asCount(scan.metrics?.update_now)} now</span>}>
        <div className="panelbody repo-list queue-grid">
          {items.length ? items.slice(0, 8).map((item, index) => (
            <div className="ledger-row" key={item.key || `${item.package_name}-${index}`}>
              <div className="rank">{String(index + 1).padStart(2, "0")}</div>
              <div>
                <div className="repo-name">{item.package_name || item.key}</div>
                <div className="feed-meta">{summaryLabel(item.summary || item.reasons?.[0] || "Dependency triage item.")}</div>
                <div className="repo-meta">
                  <span className={`chip ${recommendationTone(item.recommendation)}`}>{recommendationLabel(item.recommendation)}</span>
                  <span className="chip signal">{item.ecosystem || item.source}</span>
                  <span className="chip">{asCount(item.score)} score</span>
                </div>
              </div>
              <span className={`chip ${recommendationTone(item.recommendation)}`}>{item.update_kind || "dep"}</span>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No dependency items</strong>
              <span>This scan did not find dependency work to triage.</span>
            </div>
          )}
        </div>
      </Panel>
    );
  }
  return (
    <Panel eyebrow="Queue" title="Recent scans" action={<span className="chip signal">{history.length} saved</span>}>
      <div className="panelbody repo-list queue-grid">
        {history.length ? history.slice(0, 5).map((item) => (
          <div className="ledger-row" key={item.id}>
            <div className="rank">{asCount(item.update_now)}</div>
            <div>
              <div className="repo-name">{item.repo}</div>
              <div className="feed-meta">{summaryLabel(item.summary)}</div>
              <div className="repo-meta">
                <span className="chip red">{asCount(item.update_now)} now</span>
                <span className="chip amber">{asCount(item.watch)} watch</span>
                <span className="chip green">{asCount(item.ignore_for_now)} ignore</span>
                <span className="chip">{timeAgo(item.created_at)}</span>
              </div>
            </div>
            <button className="btn" onClick={() => onLoadScan(item.id)} type="button">Load</button>
          </div>
        )) : (
          <div className="empty-v2">
            <strong>No scans yet</strong>
            <span>Scan a repo to populate the dependency queue.</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function SidePanels({ health, scan }) {
  const warnings = scan?.warnings || [];
  return (
    <aside className="side">
      <Panel eyebrow="Evidence" title="Why it matters">
        <div className="panelbody repo-list">
          {warnings.length ? warnings.slice(0, 3).map((warning) => (
            <div className="feed-item" key={warning}>
              <div>
                <div className="feed-title">Scan warning</div>
                <div className="feed-meta">{warning}</div>
              </div>
              <span className="chip amber">warn</span>
            </div>
          )) : (
            <div className="rowline"><span className="muted">Warnings</span><span className="chip green">clear</span></div>
          )}
          <div className="rowline"><span className="muted">GitHub token</span><span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "ready" : "missing"}</span></div>
          <div className="rowline"><span className="muted">Database</span><span className={`chip ${health?.db_ok ? "green" : "red"}`}>{health?.db_ok ? "ok" : "check"}</span></div>
        </div>
      </Panel>
      <Panel eyebrow="Consumers" title="Signal handoff">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">ReleaseSentry</span><span className={`chip ${scan?.metrics?.update_now ? "amber" : "green"}`}>{scan?.metrics?.update_now ? "watch" : "clear"}</span></div>
          <div className="rowline"><span className="muted">MergeKeeper</span><span className="chip signal">context</span></div>
          <div className="rowline"><span className="muted">Human action</span><span className={`chip ${scan?.metrics?.update_now ? "red" : "green"}`}>{scan?.metrics?.update_now ? "fix now" : "none"}</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function TriageSurface({
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
              <div className="eyebrow">// Module - dependency noise filter</div>
              <h1>Dependency Pressure</h1>
              <p className="subline">Open dependency PRs, Dependabot alerts, and batchable update trains compressed into a practical action queue.</p>
            </div>
            <div className="actions">
              <span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "github ready" : "github missing"}</span>
              {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
          </div>
          <ScanForm error={error} form={form} onChange={onChangeForm} onRun={onRunScan} running={running} />
          <MetricBand metrics={metrics} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Triage" title="Dependency map" action={<span className="chip signal">dependency radar</span>}>
              <DependencyMap health={health} history={history} scan={null} />
            </Panel>
            <UpdateQueuePanel history={history} onLoadScan={onLoadScan} scan={scan} />
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
  return (
    <SecondaryFrame health={health} history={history} overview={overview} scan={scan}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// DepTriage dependency queue</div>
          <h1>Scan History</h1>
          <p className="subline">Saved dependency queues with bucket movement and alert pressure over time.</p>
        </div>
        <div className="actions">
          {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
          <button className="btn" onClick={onRefresh} type="button">{loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      <Panel eyebrow="Recent" title="Dependency scans" action={<span className="chip signal">{history.length} saved</span>}>
        <div className="panelbody repo-list queue-grid">
          {history.length ? history.map((item) => (
            <div className="ledger-row" key={item.id}>
              <div className="rank">{item.id === activeScanId ? "SEL" : asCount(item.update_now)}</div>
              <div>
                <div className="repo-name">{item.repo}</div>
                <div className="feed-meta">{summaryLabel(item.summary)}</div>
                <div className="repo-meta">
                  <span className="chip red">{asCount(item.update_now)} now</span>
                  <span className="chip amber">{asCount(item.watch)} watch</span>
                  <span className="chip green">{asCount(item.ignore_for_now)} ignore</span>
                  <span className="chip">{timeAgo(item.created_at)}</span>
                </div>
              </div>
              <button className="btn" onClick={() => onLoadScan(item.id)} type="button">Load</button>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No scans saved</strong>
              <span>Scan a repository to create dependency history.</span>
            </div>
          )}
        </div>
      </Panel>
      {scan && (
        <HistoryDetailGrid>
          <Panel eyebrow="Triage" title="Selected dependency map" action={<span className="chip signal">dependency radar</span>}>
            <DependencyMap health={health} history={history} scan={scan} />
          </Panel>
          <UpdateQueuePanel history={history} onLoadScan={onLoadScan} scan={scan} />
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

function ChecksSurface({ history, onClearScan, overview, runtime, scan }) {
  const health = runtime.health || {};
  const checks = runtime.checks || [];
  const warnings = checks.filter((check) => check.level === "warn" || check.level === "error").length;
  const metrics = [
    { label: "Status", value: health.status || "unknown", tone: health.status === "ok" ? "ok" : "warn", sub: health.version || "backend" },
    { label: "GitHub", value: githubReady(health) ? "ready" : "missing", tone: githubReady(health) ? "ok" : "hot", sub: "dependency reads" },
    { label: "Scans", value: String(asCount(health.scan_count || overview?.counts?.scans)), tone: "sig", sub: `${asCount(health.repo_count || overview?.counts?.repos)} repos` },
    { label: "Tracked", value: String(asCount(health.tracked_item_count || overview?.counts?.tracked_items)), tone: "sig", sub: "items" },
    { label: "Checks", value: warnings ? String(warnings) : "clear", tone: warnings ? "warn" : "ok", sub: "startup" },
  ];
  return (
    <SecondaryFrame health={health} history={history} overview={overview} scan={scan}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// DepTriage readiness</div>
          <h1>Checks</h1>
          <p className="subline">Backend health, GitHub dependency-read access, database state, and startup checks.</p>
        </div>
        <div className="actions">
          {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
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
  const [activeTab, setActiveTab] = usePersistentProductTab("dep-triage", TABS, "triage");
  const [error, setError] = useState("");
  const [form, setForm] = useState(DEFAULT_FORM);
  const [history, setHistory] = useState([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [overview, setOverview] = useState(null);
  const [running, setRunning] = useState(false);
  const [scan, setScan] = useState(null);
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "dep-triage_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

  async function fetchJson(path, options, fallbackError) {
    const response = await fetch_(`${API}${path}`, options);
    return parseJsonResponse(response, fallbackError);
  }

  async function refreshTriageData() {
    if (!ready) return;
    setLoadingHistory(true);
    const [overviewResult, historyResult] = await Promise.allSettled([
      fetchJson("/overview", undefined, "DepTriage could not load overview."),
      fetchJson("/history", undefined, "DepTriage could not load history."),
    ]);
    setOverview(overviewResult.status === "fulfilled" ? overviewResult.value : null);
    setHistory(historyResult.status === "fulfilled" ? historyResult.value || [] : []);
    setLoadingHistory(false);
    const failed = [overviewResult, historyResult].find((result) => result.status === "rejected");
    if (failed) {
      setError(failed.reason?.message || "DepTriage could not load one or more backend resources.");
    }
  }

  useEffect(() => {
    refreshTriageData();
  }, [ready, fetch_]);

  async function runScan() {
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(
        "/scan/github/dependencies",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            repo: form.repo,
            pr_limit: Number(form.pr_limit) || 25,
            include_alerts: Boolean(form.include_alerts),
          }),
        },
        "DepTriage could not scan that repository.",
      );
      setScan(result);
      setForm((current) => ({ ...current, repo: result.repo || current.repo }));
      setActiveTab("triage");
      await refreshTriageData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "DepTriage could not scan that repository.");
    } finally {
      setRunning(false);
    }
  }

  async function loadScan(id) {
    if (!id) return;
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(`/history/${id}`, undefined, "DepTriage could not load that scan.");
      setScan(result);
      setForm((current) => ({ ...current, repo: result.repo || current.repo }));
    } catch (err) {
      setError(err.message || "DepTriage could not load that scan.");
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
        keyPrefix="dep-triage-"
        productKey="dep-triage"
        productName="DepTriage"
      />
    );
  }

  return (
    <ProductV2Shell authConfigured={authConfigured} productKey="dep-triage" productName="DepTriage" runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="DepTriage"
        navLabel="DepTriage navigation"
        onTabChange={setActiveTab}
        productKey="dep-triage"
        tabs={TABS}
      />
      {activeTab === "triage" && (
        <TriageSurface
          error={error}
          form={form}
          health={runtime.health || {}}
          history={history}
          onChangeForm={setForm}
          onClearScan={clearScan}
          onLoadScan={loadScan}
          onRefresh={() => {
            refreshTriageData();
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
          onRefresh={refreshTriageData}
          overview={overview}
          scan={scan}
        />
      )}
      {activeTab === "checks" && <ChecksSurface history={history} onClearScan={clearScan} overview={overview} runtime={runtime} scan={scan} />}
    </ProductV2Shell>
  );
}

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
  humanizeToken,
  radarWindowFromTimestamp,
  usePersistentProductTab,
} from "@patchhivehq/ui-v2";
import { API } from "./config.js";

const TABS = [
  { id: "scout", label: "Scout" },
  { id: "history", label: "Scan history" },
  { id: "checks", label: "Checks" },
];

const POSITIONS = [
  { left: "60%", top: "28%" },
  { left: "37%", top: "38%" },
  { left: "72%", top: "62%" },
  { left: "29%", top: "70%" },
  { left: "55%", top: "76%" },
  { left: "46%", top: "54%" },
  { left: "31%", top: "31%" },
  { left: "69%", top: "44%" },
];

const DEFAULT_FORM = {
  repo_path: "",
  max_files: "250",
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

function cleanSentence(value, fallback = "") {
  const text = String(value || "").trim();
  if (!text) return fallback;
  return text.replace(/\.\.+$/, ".");
}

function visibleWarnings(warnings = []) {
  return warnings
    .filter((warning) => !String(warning || "").includes("/.vite/") && !String(warning || "").includes("\\.vite\\"))
    .map((warning) => cleanSentence(warning));
}

function safetyTone(safety) {
  const value = String(safety || "").toLowerCase();
  if (value === "high") return "green";
  if (value === "medium") return "amber";
  if (value === "low") return "red";
  return "signal";
}

function scoreTone(score) {
  const value = asCount(score);
  if (value >= 75) return "green";
  if (value >= 50) return "amber";
  return "signal";
}

async function parseJsonResponse(response, fallbackError) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || data.message || fallbackError);
  }
  return data;
}

function buildTopline(health, overview, scan, history) {
  const latest = scan || history[0] || {};
  return [
    { label: "RefactorScout", value: "Refactor map", tone: "sig" },
    { label: "System", value: health?.status || "checking", tone: health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: "Local read" },
    { label: "Scope", value: (overview?.allowed_roots || health?.allowed_roots || []).length ? "Allowed root" : "local only", tone: "sig" },
    { label: "Safety", value: `${asCount(scan?.metrics?.high_safety || overview?.high_safety_count || health?.high_safety_count)} high`, tone: "ok" },
    { label: "Last scan", value: latest.created_at ? timeAgo(latest.created_at) : overview?.scan_count ? "loaded" : "none" },
  ];
}

function buildMetrics(scan, overview, health) {
  if (scan) {
    const metrics = scan.metrics || {};
    return [
      { label: "High safety", value: String(asCount(metrics.high_safety)), tone: "ok", sub: "low blast radius" },
      { label: "Medium", value: String(asCount(metrics.medium_safety)), tone: asCount(metrics.medium_safety) ? "warn" : "ok", sub: "needs owner eye" },
      { label: "Large files", value: String(asCount(metrics.large_file_count)), tone: "sig", sub: "split candidates" },
      { label: "Repeat strings", value: String(asCount(metrics.repeated_literal_count)), tone: "sig", sub: "constant candidates" },
      { label: "Files scanned", value: String(asCount(metrics.files_scanned)), tone: "ok", sub: `${asCount(metrics.files_skipped)} skipped` },
    ];
  }
  return [
    { label: "Scans", value: String(asCount(overview?.scan_count || health?.scan_count)), tone: "sig", sub: `${asCount(overview?.repo_count || health?.repo_count)} repos` },
    { label: "Opportunities", value: String(asCount(overview?.opportunity_count || health?.opportunity_count)), tone: "sig", sub: "saved scans" },
    { label: "High safety", value: String(asCount(overview?.high_safety_count || health?.high_safety_count)), tone: "ok", sub: "safe first moves" },
    { label: "Large files", value: String(asCount(overview?.large_file_count)), tone: "sig", sub: "file splits" },
    { label: "Repeat strings", value: String(asCount(overview?.repeated_literal_count)), tone: "sig", sub: "constants" },
  ];
}

function buildRail(scan, history, overview, health) {
  const latest = history[0] || {};
  const roots = overview?.allowed_roots || health?.allowed_roots || [];
  const highSafety = asCount(scan?.metrics?.high_safety || latest.high_safety || overview?.high_safety_count || health?.high_safety_count);
  const mediumSafety = asCount(scan?.metrics?.medium_safety || latest.medium_safety || overview?.medium_safety_count || health?.medium_safety_count);
  const opportunities = asCount(scan?.metrics?.opportunities || latest.opportunities || overview?.opportunity_count || health?.opportunity_count);
  const activeDecision = opportunities ? "SCOUT" : "READY";
  return {
    sections: [
      {
        title: "Roots",
        items: [
          { label: roots[0] || "localhost", active: true, pin: true },
          { label: "max files", value: String(scan?.metrics?.files_scanned || "250") },
          { label: "remote fs", value: overview?.remote_fs_enabled || health?.remote_fs_enabled ? "enabled" : "blocked" },
          { label: "last repo", value: scan?.repo_name || overview?.last_repo || latest.repo_name || "none" },
        ],
      },
      {
        title: "Heuristics",
        items: [
          { label: "high safety", active: true, badge: String(highSafety), badgeTone: "green" },
          { label: "medium safety", badge: String(mediumSafety), badgeTone: "amber" },
          { label: "oversized files", badge: String(asCount(scan?.metrics?.large_file_count || overview?.large_file_count)), badgeTone: "signal" },
          { label: "literal repeats", badge: String(asCount(scan?.metrics?.repeated_literal_count || overview?.repeated_literal_count)), badgeTone: "signal" },
        ],
      },
    ],
    stats: {
      title: "Active repo",
      items: [
        { label: "Repository", value: scan?.repo_name || latest.repo_name || overview?.last_repo || "none" },
        { label: "Decision", value: activeDecision, large: true, tone: opportunities ? "ok" : "sig" },
        { label: "Opportunities", value: String(opportunities) },
      ],
    },
  };
}

function buildRadarItems(scan, history) {
  if (scan?.opportunities?.length) {
    return scan.opportunities.slice(0, 8).map((lead, index) => {
      const kind = humanizeToken(lead.kind, "refactor");
      const safety = humanizeToken(lead.safety, "lead");
      return {
        detail: lead.path || lead.title,
        gain: safety,
        gainMeta: `${asCount(lead.score)} score`,
        id: lead.id || `lead-${index + 1}`,
        label: kind || `R${index + 1}`,
        minWindow: index < 3 ? 7 : index < 6 ? 14 : 30,
        position: POSITIONS[index % POSITIONS.length],
        stats: [
          { label: "Kind", value: kind },
          { label: "Safety", value: humanizeToken(lead.safety, "unknown") },
          { label: "Score", value: String(asCount(lead.score)) },
          { label: "Effort", value: humanizeToken(lead.effort, "unknown") },
          { label: "Lang", value: lead.language || "n/a" },
        ],
        summary: cleanSentence(lead.summary || lead.suggestion || lead.evidence?.[0], "RefactorScout opportunity."),
        title: lead.title || lead.path || `Lead ${index + 1}`,
        tone: safetyTone(lead.safety) === "signal" ? scoreTone(lead.score) : safetyTone(lead.safety),
        vector: kind,
        vectorTone: safetyTone(lead.safety) === "amber" || safetyTone(lead.safety) === "red" ? "warn" : "",
      };
    });
  }
  if (history.length) {
    return history.map((item, index) => {
      const minWindow = radarWindowFromTimestamp(item.created_at);
      if (!minWindow) {
        return null;
      }
      return {
        detail: item.repo_path,
        gain: item.high_safety ? "high safety" : "saved",
        gainMeta: `${asCount(item.opportunities)} leads`,
        id: item.id || `history-${index + 1}`,
        label: item.repo_name || `S${index + 1}`,
        minWindow,
        position: POSITIONS[index % POSITIONS.length],
        stats: [
          { label: "Repo", value: item.repo_name || "repo" },
          { label: "Leads", value: String(asCount(item.opportunities)) },
          { label: "High", value: String(asCount(item.high_safety)) },
          { label: "Medium", value: String(asCount(item.medium_safety)) },
          { label: "Age", value: timeAgo(item.created_at) },
        ],
        summary: cleanSentence(item.summary, "Saved RefactorScout scan."),
        title: item.repo_name || item.repo_path,
        tone: item.high_safety ? "green" : item.medium_safety ? "amber" : "signal",
        vector: "saved",
        vectorTone: item.medium_safety ? "warn" : "",
      };
    }).filter(Boolean);
  }
  return [{
    detail: "No local scan yet",
    gain: "standby",
    gainMeta: "allowed path",
    id: "refactor-scout-ready",
    label: "RS",
    position: { left: "50%", top: "44%" },
    stats: [
      { label: "Mode", value: "local" },
      { label: "Root", value: "required" },
      { label: "History", value: "empty" },
      { label: "Safety", value: "first" },
      { label: "Action", value: "scan" },
    ],
    summary: "Scan an allowed local repo path to populate RefactorScout's live opportunity radar.",
    title: "RefactorScout ready",
    tone: "signal",
    vector: "READY",
  }];
}

function buildRadarFeed(scan, history, health) {
  if (scan) {
    const warnings = visibleWarnings(scan.warnings);
    return [
      { text: cleanSentence(scan.summary, "RefactorScout completed the local scan."), tone: scan.metrics?.high_safety ? "green" : scan.metrics?.medium_safety ? "amber" : "signal" },
      { text: `${asCount(scan.metrics?.high_safety)} high-safety and ${asCount(scan.metrics?.medium_safety)} medium-safety leads are active.`, tone: scan.metrics?.high_safety ? "green" : "amber" },
      { text: warnings[0] || "Scan stayed inside configured filesystem guardrails.", tone: warnings.length ? "amber" : "signal" },
    ];
  }
  return [
    { text: history.length ? `${history.length} saved local scans are available.` : "RefactorScout is waiting for a local repo scan.", tone: "signal" },
    { text: (health?.allowed_roots || []).length ? "Allowed roots are configured for local scanning." : "Configure allowed roots before scanning arbitrary paths.", tone: (health?.allowed_roots || []).length ? "green" : "amber" },
    { text: "The radar fills with file-level cleanup leads after a scan completes.", tone: "signal" },
  ];
}

function StatusBanner({ tone = "signal", children }) {
  if (!children) return null;
  return <div className={`status-banner ${tone}`}>{children}</div>;
}

function RefactorMap({ health, history, scan }) {
  const items = useMemo(() => buildRadarItems(scan, history), [scan, history]);
  const feed = useMemo(() => buildRadarFeed(scan, history, health), [scan, history, health]);
  return (
    <SuiteRadar
      ariaLabel="RefactorScout opportunity radar"
      detailLabel="Suggested first move"
      feed={feed}
      gainLabel="Safety"
      itemQueryParam="refactor"
      items={items}
      signalLabel={scan ? "leads" : "scans"}
      vectorLabel={scan ? "Selected lead" : "Selected scan"}
    />
  );
}

function ScanForm({ error, form, onChange, onRun, running }) {
  const set = (key, value) => onChange((current) => ({ ...current, [key]: value }));
  return (
    <Panel eyebrow="Scan" title="Local repo intake" action={<span className="chip signal">local read</span>}>
      <form
        className="panelbody control-stack"
        onSubmit={(event) => {
          event.preventDefault();
          onRun();
        }}
      >
        <div className="form-grid">
          <label className="v2-field">
            Repo path
            <input className="v2-input" onChange={(event) => set("repo_path", event.target.value)} placeholder="/mnt/docker/code/patchhive" value={form.repo_path} />
          </label>
          <label className="v2-field">
            Max files
            <input className="v2-input" min="25" max="1000" onChange={(event) => set("max_files", event.target.value)} type="number" value={form.max_files} />
          </label>
          <div className="v2-field">
            Action
            <button className="btn primary" disabled={running || !form.repo_path.trim()} type="submit">
              {running ? "Scanning..." : "Scan path"}
            </button>
          </div>
        </div>
        {error && <StatusBanner tone="red">{error}</StatusBanner>}
      </form>
    </Panel>
  );
}

function LeadQueuePanel({ history, onLoadScan, scan }) {
  if (scan) {
    return (
      <Panel eyebrow="Queue" title="Refactor leads" action={<span className="chip green">{asCount(scan.metrics?.high_safety)} high</span>}>
        <div className="panelbody repo-list queue-grid">
          {scan.opportunities?.length ? scan.opportunities.slice(0, 8).map((lead, index) => (
            <div className="ledger-row" key={lead.id || index}>
              <div className="rank">{String(index + 1).padStart(2, "0")}</div>
              <div>
                <div className="repo-name">{lead.title || lead.path}</div>
                <div className="feed-meta">{cleanSentence(lead.summary || lead.suggestion)}</div>
                <div className="repo-meta">
                  <span className={`chip ${safetyTone(lead.safety)}`}>{humanizeToken(lead.safety, "lead")}</span>
                  <span className="chip signal">{humanizeToken(lead.kind, "refactor")}</span>
                  <span className="chip">{lead.path}</span>
                </div>
              </div>
              <span className={`chip ${scoreTone(lead.score)}`}>{asCount(lead.score)}</span>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No refactor leads</strong>
              <span>This scan did not find clear cleanup opportunities.</span>
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
              <div className="repo-name">{item.repo_name || item.repo_path}</div>
              <div className="feed-meta">{cleanSentence(item.summary, "Saved RefactorScout scan.")}</div>
              <div className="repo-meta">
                <span className="chip green">{asCount(item.high_safety)} high</span>
                <span className="chip amber">{asCount(item.medium_safety)} medium</span>
                <span className="chip signal">{asCount(item.opportunities)} leads</span>
                <span className="chip">{timeAgo(item.created_at)}</span>
              </div>
            </div>
            <button className="btn" onClick={() => onLoadScan(item.id)} type="button">Load</button>
          </div>
        )) : (
          <div className="empty-v2">
            <strong>No scans yet</strong>
            <span>Scan an allowed local repo path to populate the queue.</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function SidePanels({ health, scan }) {
  const warnings = visibleWarnings(scan?.warnings || []);
  return (
    <aside className="side">
      <Panel eyebrow="Evidence" title="Why it is safe">
        <div className="panelbody repo-list">
          {warnings.length ? warnings.slice(0, 3).map((warning) => (
            <div className="feed-item" key={warning}>
              <div>
                <div className="feed-title">Scan warning</div>
                <div className="feed-meta">{cleanSentence(warning)}</div>
              </div>
              <span className="chip amber">warn</span>
            </div>
          )) : (
            <div className="rowline"><span className="muted">Warnings</span><span className="chip green">clear</span></div>
          )}
          <div className="rowline"><span className="muted">Allowed roots</span><span className="chip signal">{(health?.allowed_roots || []).length}</span></div>
          <div className="rowline"><span className="muted">Remote FS</span><span className={`chip ${health?.remote_fs_enabled ? "amber" : "green"}`}>{health?.remote_fs_enabled ? "enabled" : "blocked"}</span></div>
        </div>
      </Panel>
      <Panel eyebrow="Consumers" title="Signal handoff">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">RepoMemory</span><span className="chip signal">context</span></div>
          <div className="rowline"><span className="muted">TrustGate</span><span className={`chip ${scan?.metrics?.high_safety ? "green" : "signal"}`}>{scan?.metrics?.high_safety ? "safe" : "ready"}</span></div>
          <div className="rowline"><span className="muted">RepoReaper</span><span className="chip amber">later</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function ScoutSurface({
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
              <div className="eyebrow">// Module - conservative cleanup</div>
              <h1>Refactor Opportunity</h1>
              <p className="subline">Local repo paths, explicit scan caps, and explainable heuristics turned into safe cleanup leads.</p>
            </div>
            <div className="actions">
              <span className={`chip ${(health?.allowed_roots || []).length ? "green" : "amber"}`}>{(health?.allowed_roots || []).length ? "root ready" : "root missing"}</span>
              {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
          </div>
          <ScanForm error={error} form={form} onChange={onChangeForm} onRun={onRunScan} running={running} />
          <MetricBand metrics={metrics} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Scout" title="Opportunity map" action={<span className="chip signal">scout radar</span>}>
              <RefactorMap health={health} history={history} scan={scan} />
            </Panel>
            <LeadQueuePanel history={history} onLoadScan={onLoadScan} scan={scan} />
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
          <div className="eyebrow">// RefactorScout cleanup queue</div>
          <h1>Scan History</h1>
          <p className="subline">Saved local scans with high-safety lead movement and ignored-path evidence.</p>
        </div>
        <div className="actions">
          {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
          <button className="btn" onClick={onRefresh} type="button">{loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      <Panel eyebrow="Recent" title="Local scans" action={<span className="chip signal">{history.length} saved</span>}>
        <div className="panelbody repo-list queue-grid">
          {history.length ? history.map((item, index) => (
            <div className="ledger-row" key={item.id}>
              <div className="rank">{item.id === activeScanId ? "SEL" : String(index + 1).padStart(2, "0")}</div>
              <div>
                <div className="repo-name">{item.repo_name || item.repo_path}</div>
                <div className="feed-meta">{cleanSentence(item.summary, "Saved RefactorScout scan.")}</div>
                <div className="repo-meta">
                  <span className="chip green">{asCount(item.high_safety)} high</span>
                  <span className="chip amber">{asCount(item.medium_safety)} medium</span>
                  <span className="chip signal">{asCount(item.opportunities)} leads</span>
                  <span className="chip">{timeAgo(item.created_at)}</span>
                </div>
              </div>
              <button className="btn" onClick={() => onLoadScan(item.id)} type="button">Load</button>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No scans saved</strong>
              <span>Scan an allowed local repo path to create history.</span>
            </div>
          )}
        </div>
      </Panel>
      {scan && (
        <HistoryDetailGrid>
          <Panel eyebrow="Scout" title="Selected opportunity map" action={<span className="chip signal">scout radar</span>}>
            <RefactorMap health={health} history={history} scan={scan} />
          </Panel>
          <LeadQueuePanel history={history} onLoadScan={onLoadScan} scan={scan} />
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
    { label: "Allowed roots", value: String((health.allowed_roots || overview?.allowed_roots || []).length), tone: (health.allowed_roots || overview?.allowed_roots || []).length ? "ok" : "warn", sub: "local scope" },
    { label: "Scans", value: String(asCount(health.scan_count || overview?.scan_count)), tone: "sig", sub: `${asCount(health.repo_count || overview?.repo_count)} repos` },
    { label: "Opportunities", value: String(asCount(health.opportunity_count || overview?.opportunity_count)), tone: "sig", sub: "saved leads" },
    { label: "Checks", value: warnings ? String(warnings) : "clear", tone: warnings ? "warn" : "ok", sub: "startup" },
  ];
  return (
    <SecondaryFrame health={health} history={history} overview={overview} scan={scan}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// RefactorScout readiness</div>
          <h1>Checks</h1>
          <p className="subline">Backend health, filesystem guardrails, local scan scope, and startup checks.</p>
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
            <div className="rowline"><span className="muted">Remote filesystem</span><span className={`chip ${health.remote_fs_enabled ? "amber" : "green"}`}>{health.remote_fs_enabled ? "enabled" : "blocked"}</span></div>
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
  const [activeTab, setActiveTab] = usePersistentProductTab("refactor-scout", TABS, "scout");
  const [error, setError] = useState("");
  const [form, setForm] = useState(DEFAULT_FORM);
  const [history, setHistory] = useState([]);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [overview, setOverview] = useState(null);
  const [running, setRunning] = useState(false);
  const [scan, setScan] = useState(null);
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "refactor-scout_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

  async function fetchJson(path, options, fallbackError) {
    const response = await fetch_(`${API}${path}`, options);
    return parseJsonResponse(response, fallbackError);
  }

  async function refreshScoutData() {
    if (!ready) return;
    setLoadingHistory(true);
    const [overviewResult, historyResult] = await Promise.allSettled([
      fetchJson("/overview", undefined, "RefactorScout could not load overview."),
      fetchJson("/history", undefined, "RefactorScout could not load history."),
    ]);
    setOverview(overviewResult.status === "fulfilled" ? overviewResult.value : null);
    setHistory(historyResult.status === "fulfilled" ? historyResult.value || [] : []);
    setLoadingHistory(false);
    const failed = [overviewResult, historyResult].find((result) => result.status === "rejected");
    if (failed) {
      setError(failed.reason?.message || "RefactorScout could not load one or more backend resources.");
    }
  }

  useEffect(() => {
    refreshScoutData();
  }, [ready, fetch_]);

  async function runScan() {
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(
        "/scan/local",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            repo_path: form.repo_path,
            max_files: Number(form.max_files) || 250,
          }),
        },
        "RefactorScout could not scan that path.",
      );
      setScan(result);
      setForm((current) => ({ ...current, repo_path: result.repo_path || current.repo_path }));
      setActiveTab("scout");
      await refreshScoutData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "RefactorScout could not scan that path.");
    } finally {
      setRunning(false);
    }
  }

  async function loadScan(id) {
    if (!id) return;
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(`/history/${id}`, undefined, "RefactorScout could not load that scan.");
      setScan(result);
      setForm((current) => ({ ...current, repo_path: result.repo_path || current.repo_path }));
    } catch (err) {
      setError(err.message || "RefactorScout could not load that scan.");
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
        keyPrefix="refactor-scout-"
        productKey="refactor-scout"
        productName="RefactorScout"
      />
    );
  }

  return (
    <ProductV2Shell authConfigured={authConfigured} productKey="refactor-scout" productName="RefactorScout" runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="RefactorScout"
        navLabel="RefactorScout navigation"
        onTabChange={setActiveTab}
        productKey="refactor-scout"
        tabs={TABS}
      />
      {activeTab === "scout" && (
        <ScoutSurface
          error={error}
          form={form}
          health={runtime.health || {}}
          history={history}
          onChangeForm={setForm}
          onClearScan={clearScan}
          onLoadScan={loadScan}
          onRefresh={() => {
            refreshScoutData();
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
          onRefresh={refreshScoutData}
          overview={overview}
          scan={scan}
        />
      )}
      {activeTab === "checks" && <ChecksSurface history={history} onClearScan={clearScan} overview={overview} runtime={runtime} scan={scan} />}
    </ProductV2Shell>
  );
}

import { useMemo, useState } from "react";
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
} from "@patchhivehq/ui-v2";
import { API } from "./config.js";

const TABS = [
  { id: "triage", label: "Triage" },
  { id: "history", label: "Scan history" },
  { id: "checks", label: "Checks" },
];

const TOPLINE_CELLS = [
  { label: "DepTriage", value: "Dependency queue", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Read only" },
  { label: "GitHub", value: "PR + alert read", tone: "sig" },
  { label: "Alerts", value: "11 open", tone: "warn" },
  { label: "Last scan", value: "T-01:20" },
];

const RAIL_SECTIONS = [
  {
    title: "Sources",
    items: [
      { label: "Dependency PRs", active: true, pin: true },
      { label: "Dependabot alerts", value: "11" },
      { label: "Update trains", value: "5" },
      { label: "Ignored packages", value: "22" },
    ],
  },
  {
    title: "Buckets",
    items: [
      { label: "update now", active: true, badge: "6", badgeTone: "red" },
      { label: "watch", badge: "14", badgeTone: "amber" },
      { label: "ignore for now", badge: "22", badgeTone: "green" },
      { label: "access limited", badge: "3", badgeTone: "signal" },
    ],
  },
];

const RAIL_STATS = {
  title: "Active repo",
  items: [
    { label: "Repository", value: "patchhive/hive-core" },
    { label: "Decision", value: "WATCH", large: true, tone: "warn" },
    { label: "Focus", value: "transitive risk" },
  ],
};

const METRICS = [
  { label: "Update now", value: "6", tone: "hot", sub: "security + breakage" },
  { label: "Watch", value: "14", tone: "warn", sub: "non-blocking drift" },
  { label: "Alerts", value: "11", tone: "hot", sub: "4 high severity" },
  { label: "Safe defers", value: "22", tone: "ok", sub: "low-value churn" },
  { label: "Noise cut", value: "73%", tone: "sig", sub: "queue compression" },
];

const PACKAGES = [
  {
    id: "DT-01",
    title: "vite",
    bucket: "now",
    tone: "amber",
    state: "update now",
    value: "major +2",
    position: { left: "57%", top: "25%" },
    summary: "Multiple update PRs touch the same frontend build path and one is pinned behind a failing plugin range.",
  },
  {
    id: "DT-02",
    title: "openssl",
    bucket: "alert",
    tone: "red",
    state: "fix now",
    value: "high alert",
    position: { left: "73%", top: "46%" },
    summary: "A high-severity advisory overlaps runtime paths, making this the clearest dependency action.",
  },
  {
    id: "DT-03",
    title: "react",
    bucket: "watch",
    tone: "signal",
    state: "watch",
    value: "minor train",
    position: { left: "38%", top: "34%" },
    summary: "The open PR is compatible, but it should ride with the Vite train instead of landing alone.",
  },
  {
    id: "DT-04",
    title: "eslint",
    bucket: "ignore",
    tone: "green",
    state: "ignore for now",
    value: "dev only",
    position: { left: "31%", top: "69%" },
    summary: "Low impact dev-tool churn with no alert pressure and no release-blocking effect.",
  },
  {
    id: "DT-05",
    title: "tokio",
    bucket: "now",
    tone: "amber",
    state: "batch",
    value: "runtime drift",
    position: { left: "63%", top: "73%" },
    summary: "Runtime dependency drift is not urgent alone, but it should be batched with adjacent async stack updates.",
  },
];

const LINKS = [
  { from: "DT-01", to: "DT-03", style: { left: "41%", top: "30%", width: "122px", transform: "rotate(-13deg)" } },
  { from: "DT-01", to: "DT-02", style: { left: "58%", top: "34%", width: "112px", transform: "rotate(35deg)" } },
  { from: "DT-02", to: "DT-05", style: { left: "64%", top: "57%", width: "118px", transform: "rotate(104deg)" } },
  { from: "DT-04", to: "DT-05", style: { left: "35%", top: "71%", width: "156px", transform: "rotate(4deg)" } },
];

const FILTERS = [
  { id: "all", label: "all" },
  { id: "now", label: "now" },
  { id: "watch", label: "watch" },
  { id: "ignore", label: "ignore" },
  { id: "alert", label: "alert" },
];

const UPDATE_QUEUE = [
  { rank: "01", title: "openssl", meta: "high advisory overlaps runtime path", tone: "red", label: "fix now" },
  { rank: "02", title: "vite + react train", meta: "batch frontend toolchain PRs together", tone: "amber", label: "batch" },
  { rank: "03", title: "tokio", meta: "runtime drift; schedule with async stack updates", tone: "amber", label: "watch" },
  { rank: "04", title: "eslint", meta: "dev-only churn with no alert pressure", tone: "green", label: "defer" },
];

const EVIDENCE = [
  { title: "Alert pressure is concentrated", meta: "4 high alerts map to 2 packages", label: "high", tone: "red" },
  { title: "Batchable train found", meta: "Vite, React, and plugin ranges should land together", label: "batch", tone: "amber" },
  { title: "Noise safely suppressed", meta: "22 updates are dev-only, patch-only, or duplicate PRs", label: "clear", tone: "green" },
];

const HISTORY = [
  { title: "hive-core / dependency queue", meta: "update-now bucket dropped from 9 to 6", label: "better", tone: "green" },
  { title: "repo-reaper / Dependabot alerts", meta: "two high advisories still open", label: "watch", tone: "amber" },
  { title: "signal-hive / frontend train", meta: "safe to batch after Vite plugin range clears", label: "batch", tone: "signal" },
];

function DependencyMap() {
  return (
    <SuiteRadar
      ariaLabel="DepTriage dependency pressure radar"
      detailLabel="Triage reason"
      feed={[
        { text: "High advisory pressure concentrates around two packages.", tone: "red" },
        { text: "Vite and React should move as one update train.", tone: "amber" },
        { text: "Dev-only update churn stays safely deferred." },
      ]}
      gainLabel="Decision"
      items={PACKAGES.map((pkg) => ({
        ...pkg,
        detail: pkg.title,
        gain: pkg.state,
        gainMeta: pkg.value,
        label: pkg.id,
        stats: [
          { label: "Bucket", value: pkg.bucket },
          { label: "State", value: pkg.state },
          { label: "Value", value: pkg.value },
          { label: "Action", value: pkg.tone === "red" ? "fix" : pkg.tone === "green" ? "defer" : "watch" },
          { label: "Queue", value: "deps" },
        ],
        vector: pkg.id,
        vectorTone: pkg.tone === "amber" || pkg.tone === "red" ? "warn" : "",
      }))}
      signalLabel="packages"
      vectorLabel="Selected package"
    />
  );
}

function UpdateQueuePanel() {
  return (
    <Panel eyebrow="Queue" title="Dependency decisions" action={<span className="chip red">6 now</span>}>
      <div className="panelbody repo-list queue-grid">
        {UPDATE_QUEUE.map((item) => (
          <div className="ledger-row" key={item.rank}>
            <div className="rank">{item.rank}</div>
            <div>
              <div className="repo-name">{item.title}</div>
              <div className="feed-meta">{item.meta}</div>
            </div>
            <span className={`chip ${item.tone}`}>{item.label}</span>
          </div>
        ))}
      </div>
    </Panel>
  );
}

function SidePanels() {
  return (
    <aside className="side">
      <Panel eyebrow="Evidence" title="Why it matters">
        <div className="panelbody repo-list">
          {EVIDENCE.map((item) => (
            <div className="feed-item" key={item.title}>
              <div>
                <div className="feed-title">{item.title}</div>
                <div className="feed-meta">{item.meta}</div>
              </div>
              <span className={`chip ${item.tone}`}>{item.label}</span>
            </div>
          ))}
        </div>
      </Panel>
      <Panel eyebrow="Consumers" title="Signal handoff">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">ReleaseSentry</span><span className="chip amber">watch</span></div>
          <div className="rowline"><span className="muted">MergeKeeper</span><span className="chip signal">context</span></div>
          <div className="rowline"><span className="muted">Human action</span><span className="chip red">fix now</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function TriageSurface() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - dependency noise filter</div>
              <h1>Dependency Pressure</h1>
              <p className="subline">Open dependency PRs, Dependabot alerts, and batchable update trains compressed into a practical action queue.</p>
            </div>
            <div className="actions">
              <span className="chip red">update now</span>
              <span className="chip signal">read-only</span>
              <button className="btn primary" type="button">Scan repo</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Triage" title="Dependency map" action={<span className="chip signal">dependency radar</span>}>
              <DependencyMap />
            </Panel>
            <UpdateQueuePanel />
          </div>
        </main>
        <SidePanels />
      </div>
    </>
  );
}

function SecondaryFrame({ children }) {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">{children}</main>
      </div>
    </>
  );
}

function HistorySurface() {
  return (
    <SecondaryFrame>
      <div>
        <div className="eyebrow">// DepTriage dependency queue</div>
        <h1>Scan History</h1>
        <p className="subline">Saved dependency queues with bucket movement and alert pressure over time.</p>
      </div>
      <Panel eyebrow="Recent" title="Dependency scans">
        <div className="panelbody repo-list queue-grid">
          {HISTORY.map((item) => (
            <div className="feed-item" key={item.title}>
              <div>
                <div className="feed-title">{item.title}</div>
                <div className="feed-meta">{item.meta}</div>
              </div>
              <span className={`chip ${item.tone}`}>{item.label}</span>
            </div>
          ))}
        </div>
      </Panel>
    </SecondaryFrame>
  );
}

function Placeholder({ title, body }) {
  return (
    <SecondaryFrame>
      <div className="eyebrow">// DepTriage dependency queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </SecondaryFrame>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("triage");
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "dep-triage_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

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
    <ProductV2Shell authConfigured={authConfigured} runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="DepTriage"
        navLabel="DepTriage navigation"
        onTabChange={setActiveTab}
        productKey="dep-triage"
        tabs={TABS}
      />
      {activeTab === "triage" && <TriageSurface />}
      {activeTab === "history" && <HistorySurface />}
      {activeTab === "checks" && (
        <Placeholder
          title="Checks"
          body="GitHub token, Dependabot alert access, and backend readiness checks for DepTriage."
        />
      )}
    </ProductV2Shell>
  );
}

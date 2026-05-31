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
  { id: "gate", label: "Release gate" },
  { id: "history", label: "Run history" },
  { id: "checks", label: "Checks" },
];

const TOPLINE_CELLS = [
  { label: "ReleaseSentry", value: "Release gate", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Read only" },
  { label: "GitHub", value: "Release read", tone: "sig" },
  { label: "Decision", value: "Watch", tone: "warn" },
  { label: "Last check", value: "T-00:35" },
];

const RAIL_SECTIONS = [
  {
    title: "Candidate",
    items: [
      { label: "main -> v0.2.0", active: true, pin: true },
      { label: "workflow runs", value: "20" },
      { label: "changelog", value: "drift" },
      { label: "release blockers", value: "2" },
    ],
  },
  {
    title: "Checks",
    items: [
      { label: "CI health", active: true, badge: "17/20", badgeTone: "green" },
      { label: "blockers", badge: "2", badgeTone: "red" },
      { label: "version drift", badge: "3", badgeTone: "amber" },
      { label: "security pressure", badge: "1", badgeTone: "signal" },
    ],
  },
];

const RAIL_STATS = {
  title: "Active release",
  items: [
    { label: "Repository", value: "patchhive/patchhive" },
    { label: "Decision", value: "WATCH", large: true, tone: "warn" },
    { label: "Focus", value: "changelog drift" },
  ],
};

const METRICS = [
  { label: "Readiness", value: "68", tone: "warn", sub: "watch threshold" },
  { label: "CI pass", value: "17", tone: "ok", sub: "of 20 runs" },
  { label: "Blockers", value: "2", tone: "hot", sub: "release labels" },
  { label: "Drift", value: "3", tone: "warn", sub: "version surfaces" },
  { label: "Evidence", value: "12", tone: "sig", sub: "supporting signals" },
];

const SIGNALS = [
  {
    id: "RL-01",
    title: "CI branch health",
    bucket: "ci",
    tone: "green",
    state: "ready",
    value: "17/20 pass",
    position: { left: "35%", top: "34%" },
    summary: "Most workflow runs are green, and the failing runs do not point at the release path.",
  },
  {
    id: "RL-02",
    title: "open blocker issues",
    bucket: "blocker",
    tone: "red",
    state: "hold pressure",
    value: "2 blockers",
    position: { left: "69%", top: "33%" },
    summary: "Two open issues carry release-blocker labels and should be resolved or explicitly waived.",
  },
  {
    id: "RL-03",
    title: "changelog drift",
    bucket: "drift",
    tone: "amber",
    state: "watch",
    value: "missing v0.2.0",
    position: { left: "55%", top: "66%" },
    summary: "The target version appears in package metadata but not in the changelog section.",
  },
  {
    id: "RL-04",
    title: "tag alignment",
    bucket: "drift",
    tone: "signal",
    state: "watch",
    value: "tag absent",
    position: { left: "28%", top: "70%" },
    summary: "No release tag exists yet, which is fine before ship but should not be missed at publish time.",
  },
  {
    id: "RL-05",
    title: "dependency pressure",
    bucket: "risk",
    tone: "amber",
    state: "watch",
    value: "one advisory",
    position: { left: "75%", top: "62%" },
    summary: "Dependency pressure is not enough to block alone, but it keeps the release out of ready status.",
  },
];

const LINKS = [
  { from: "RL-01", to: "RL-02", style: { left: "38%", top: "33%", width: "156px", transform: "rotate(-1deg)" } },
  { from: "RL-02", to: "RL-05", style: { left: "69%", top: "44%", width: "120px", transform: "rotate(82deg)" } },
  { from: "RL-03", to: "RL-04", style: { left: "31%", top: "68%", width: "118px", transform: "rotate(-5deg)" } },
  { from: "RL-03", to: "RL-05", style: { left: "57%", top: "64%", width: "96px", transform: "rotate(-8deg)" } },
];

const FILTERS = [
  { id: "all", label: "all" },
  { id: "ci", label: "ci" },
  { id: "blocker", label: "blocker" },
  { id: "drift", label: "drift" },
  { id: "risk", label: "risk" },
];

const GATE_QUEUE = [
  { rank: "01", title: "Resolve release blockers", meta: "2 open issues carry release-blocker labels", tone: "red", label: "hold" },
  { rank: "02", title: "Patch changelog drift", meta: "target version missing from release notes", tone: "amber", label: "watch" },
  { rank: "03", title: "Review dependency pressure", meta: "one advisory keeps release out of ready", tone: "amber", label: "watch" },
  { rank: "04", title: "Confirm CI branch health", meta: "17 of 20 workflow runs are green", tone: "green", label: "ready" },
];

const EVIDENCE = [
  { title: "Watch, not ready", meta: "CI is mostly green, but blockers and drift remain", label: "watch", tone: "amber" },
  { title: "Release blockers are explicit", meta: "two issues carry release-blocker labels", label: "hold", tone: "red" },
  { title: "Ship checklist is close", meta: "tag alignment can happen at publish time", label: "near", tone: "signal" },
];

const HISTORY = [
  { title: "patchhive / v0.2.0", meta: "decision stayed watch after blocker scan", label: "watch", tone: "amber" },
  { title: "repo-memory / v0.1.4", meta: "ready after changelog drift resolved", label: "ready", tone: "green" },
  { title: "repo-reaper / main", meta: "security pressure blocked release check", label: "hold", tone: "red" },
];

function ReleaseMap() {
  return (
    <SuiteRadar
      ariaLabel="ReleaseSentry readiness radar"
      detailLabel="Evidence"
      feed={[
        { text: "Release blockers keep the decision at watch.", tone: "amber" },
        { text: "CI branch health is mostly clear.", tone: "green" },
        { text: "Changelog drift must be patched before ship." },
      ]}
      gainLabel="Decision"
      items={SIGNALS.map((signal) => ({
        ...signal,
        detail: signal.title,
        gain: signal.state,
        gainMeta: signal.value,
        label: signal.id,
        stats: [
          { label: "Bucket", value: signal.bucket },
          { label: "State", value: signal.state },
          { label: "Value", value: signal.value },
          { label: "Gate", value: signal.tone === "red" ? "hold" : signal.tone === "green" ? "ready" : "watch" },
          { label: "Release", value: "v0.2.0" },
        ],
        vector: signal.id,
        vectorTone: signal.tone === "amber" || signal.tone === "red" ? "warn" : "",
      }))}
      signalLabel="signals"
      vectorLabel="Selected signal"
    />
  );
}

function GateQueuePanel() {
  return (
    <Panel eyebrow="Queue" title="Release evidence" action={<span className="chip amber">watch</span>}>
      <div className="panelbody repo-list queue-grid">
        {GATE_QUEUE.map((item) => (
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
      <Panel eyebrow="Evidence" title="Ship/no-ship call">
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
          <div className="rowline"><span className="muted">MergeKeeper</span><span className="chip green">clear</span></div>
          <div className="rowline"><span className="muted">VulnTriage</span><span className="chip amber">watch</span></div>
          <div className="rowline"><span className="muted">HiveCore</span><span className="chip amber">gate</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function GateSurface() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - release readiness</div>
              <h1>Release Gate</h1>
              <p className="subline">Branch health, blockers, changelog drift, tag alignment, and cross-product pressure turned into a ship call.</p>
            </div>
            <div className="actions">
              <span className="chip amber">watch</span>
              <span className="chip signal">read-only</span>
              <button className="btn primary" type="button">Check release</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Gate" title="Readiness map" action={<span className="chip signal">release radar</span>}>
              <ReleaseMap />
            </Panel>
            <GateQueuePanel />
          </div>
        </main>
        <SidePanels />
      </div>
    </>
  );
}

function HistorySurface() {
  return (
    <div className="product-page-shell">
      <div>
        <div className="eyebrow">// ReleaseSentry release gate</div>
        <h1>Run History</h1>
        <p className="subline">Saved release checks with readiness changes, blockers, and final ship/no-ship evidence.</p>
      </div>
      <Panel eyebrow="Recent" title="Release checks">
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
    </div>
  );
}

function Placeholder({ title, body }) {
  return (
    <div className="product-page-shell">
      <div className="eyebrow">// ReleaseSentry release gate</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("gate");
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "release-sentry_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

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
    <ProductV2Shell authConfigured={authConfigured} runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="ReleaseSentry"
        navLabel="ReleaseSentry navigation"
        onTabChange={setActiveTab}
        productKey="release-sentry"
        tabs={TABS}
      />
      {activeTab === "gate" && <GateSurface />}
      {activeTab === "history" && <HistorySurface />}
      {activeTab === "checks" && (
        <Placeholder
          title="Checks"
          body="GitHub release, Actions, issue-label, changelog, and backend readiness checks for ReleaseSentry."
        />
      )}
    </ProductV2Shell>
  );
}

import { useState } from "react";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductRail,
  SuiteTopline,
} from "@patchhivehq/ui-v2";

const TABS = [
  { id: "gate", label: "Release gate" },
  { id: "history", label: "Run history" },
  { id: "checks", label: "Checks" },
];

const TOPLINE_CELLS = [
  { label: "ReleaseSentry", value: "Gate 12", tone: "sig" },
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
  const [filter, setFilter] = useState("all");
  const [activeSignal, setActiveSignal] = useState(SIGNALS[1]);
  const visibleSignals = SIGNALS.filter((signal) => filter === "all" || signal.bucket === filter);
  const visibleIds = new Set(visibleSignals.map((signal) => signal.id));
  const visibleLinks = LINKS.filter((link) => visibleIds.has(link.from) && visibleIds.has(link.to));

  const changeFilter = (nextFilter) => {
    setFilter(nextFilter);
    const nextSignal = SIGNALS.find((signal) => nextFilter === "all" || signal.bucket === nextFilter);
    if (nextSignal) {
      setActiveSignal(nextSignal);
    }
  };

  return (
    <div className="signal-map release-map" data-window="30">
      <div className="range-panel">
        <span className="chip signal">{visibleSignals.length} signals</span>
        <div className="range-switch" aria-label="Release gate filter">
          {FILTERS.map((item) => (
            <button
              className={`range-btn${filter === item.id ? " active" : ""}`}
              key={item.id}
              onClick={() => changeFilter(item.id)}
              type="button"
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>
      <div className="release-scope" aria-label="Release readiness map">
        <span className="release-core">
          <span className="memory-core-title">ReleaseSentry</span>
          <span className="memory-core-count">68</span>
          <span className="micro">readiness</span>
        </span>
        {visibleLinks.map((link) => (
          <span className="memory-link release-link" key={`${link.from}-${link.to}`} style={link.style} />
        ))}
        {visibleSignals.map((signal) => (
          <button
            aria-label={`Show release signal ${signal.id}`}
            className={`memory-node release-node ${signal.tone}${activeSignal.id === signal.id ? " active" : ""}`}
            data-label={signal.id}
            key={signal.id}
            onClick={() => setActiveSignal(signal)}
            style={signal.position}
            type="button"
          />
        ))}
        <span className="release-gate gate-a" />
        <span className="release-gate gate-b" />
        <span className="release-gate gate-c" />
      </div>
      <div className="radar-readout release-readout">
        <div className="readout-card">
          <span className="label">Selected signal</span>
          <span className={`readout-value ${activeSignal.tone === "amber" || activeSignal.tone === "red" ? "warn" : ""}`}>{activeSignal.id}</span>
          <span className="micro">{activeSignal.bucket} bucket</span>
        </div>
        <div className="readout-card">
          <span className="label">Decision</span>
          <span className="readout-value">{activeSignal.state}</span>
          <span className="micro">{activeSignal.value}</span>
        </div>
        <div className="readout-card selected-scan">
          <span className="label">Evidence</span>
          <span className="readout-value">{activeSignal.title}</span>
          <span className="micro">{activeSignal.summary}</span>
        </div>
      </div>
    </div>
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
            <Panel eyebrow="Gate" title="Readiness map" action={<span className="chip signal">react port</span>}>
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
    <div className="placeholder-shell">
      <div>
        <div className="eyebrow">// ReleaseSentry v2 extraction queue</div>
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
    <div className="placeholder-shell">
      <div className="eyebrow">// ReleaseSentry v2 extraction queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("gate");

  return (
    <>
      <DeckBar
        activeTab={activeTab}
        brandName="ReleaseSentry frontend v2"
        navLabel="ReleaseSentry v2 surfaces"
        onTabChange={setActiveTab}
        tabs={TABS}
      />
      {activeTab === "gate" && <GateSurface />}
      {activeTab === "history" && <HistorySurface />}
      {activeTab === "checks" && (
        <Placeholder
          title="Checks"
          body="This becomes the shared v2 GitHub release, Actions, issue-label, changelog, and backend readiness surface."
        />
      )}
    </>
  );
}

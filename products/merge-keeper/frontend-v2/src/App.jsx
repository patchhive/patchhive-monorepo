import { useState } from "react";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductRail,
  SuiteTopline,
} from "@patchhivehq/ui-v2";

const TABS = [
  { id: "readiness", label: "Readiness" },
  { id: "history", label: "Decision log" },
  { id: "checks", label: "Checks" },
];

const TOPLINE_CELLS = [
  { label: "MergeKeeper", value: "Merge array 05", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Readiness" },
  { label: "GitHub", value: "PR state", tone: "sig" },
  { label: "Suite input", value: "3 sources", tone: "warn" },
  { label: "Last call", value: "T-04:00" },
];

const RAIL_SECTIONS = [
  {
    title: "Pull requests",
    items: [
      { label: "patchhive/repo-reaper#42", active: true, pin: true },
      { label: "review-bee#19", value: "ready" },
      { label: "trust-gate#27", value: "hold" },
      { label: "signal-hive#33", value: "clear" },
    ],
  },
  {
    title: "Inputs",
    items: [
      { label: "review pressure", active: true, badge: "warn", badgeTone: "amber" },
      { label: "risk gate", badge: "clear", badgeTone: "green" },
      { label: "ci health", badge: "pass", badgeTone: "green" },
      { label: "memory rules", badge: "watch", badgeTone: "signal" },
    ],
  },
];

const RAIL_STATS = {
  title: "Active PR",
  items: [
    { label: "Repository", value: "patchhive/repo-reaper" },
    { label: "Decision", value: "HOLD", large: true, tone: "warn" },
    { label: "Report", value: "published" },
  ],
};

const METRICS = [
  { label: "Readiness", value: "HOLD", tone: "warn", sub: "2 blockers remain" },
  { label: "Review pressure", value: "5", tone: "warn", sub: "from ReviewBee" },
  { label: "Checks", value: "12/12", tone: "ok", sub: "all required green" },
  { label: "Risk", value: "LOW", tone: "ok", sub: "TrustGate clear" },
  { label: "Time waiting", value: "3h", tone: "sig", sub: "review response" },
];

const SIGNALS = [
  {
    id: "MK-01",
    title: "ReviewBee unresolved action group",
    source: "review",
    tone: "amber",
    state: "hold",
    value: "5 open",
    position: { left: "49%", top: "22%" },
    summary: "Two review asks still look merge-relevant: auth regression test and retry fallback copy.",
  },
  {
    id: "MK-02",
    title: "Required checks passing",
    source: "ci",
    tone: "green",
    state: "ready",
    value: "12/12",
    position: { left: "72%", top: "48%" },
    summary: "All required checks are green and no flaky rerun pressure is visible.",
  },
  {
    id: "MK-03",
    title: "TrustGate risk clear",
    source: "risk",
    tone: "green",
    state: "clear",
    value: "low",
    position: { left: "57%", top: "74%" },
    summary: "Diff avoids sensitive paths and stays under the configured scope cap.",
  },
  {
    id: "MK-04",
    title: "RepoMemory expectation warning",
    source: "memory",
    tone: "signal",
    state: "watch",
    value: "policy",
    position: { left: "28%", top: "61%" },
    summary: "Repo history prefers explicit startup-check notes when product routes change.",
  },
  {
    id: "MK-05",
    title: "Approval state incomplete",
    source: "review",
    tone: "red",
    state: "blocked",
    value: "1 missing",
    position: { left: "31%", top: "34%" },
    summary: "One required reviewer has not re-approved since the latest push.",
  },
];

const LINKS = [
  { from: "MK-01", to: "MK-05", style: { left: "34%", top: "27%", width: "122px", transform: "rotate(-18deg)" } },
  { from: "MK-01", to: "MK-02", style: { left: "51%", top: "35%", width: "132px", transform: "rotate(33deg)" } },
  { from: "MK-02", to: "MK-03", style: { left: "61%", top: "60%", width: "102px", transform: "rotate(112deg)" } },
  { from: "MK-04", to: "MK-03", style: { left: "34%", top: "68%", width: "160px", transform: "rotate(14deg)" } },
];

const FILTERS = [
  { id: "all", label: "all" },
  { id: "review", label: "review" },
  { id: "ci", label: "ci" },
  { id: "risk", label: "risk" },
];

const BLOCKERS = [
  { rank: "01", title: "Missing re-approval after latest push", meta: "required reviewer has not re-approved", tone: "red", label: "block" },
  { rank: "02", title: "Auth regression test still requested", meta: "ReviewBee action group remains open", tone: "amber", label: "hold" },
  { rank: "03", title: "Retry fallback copy needs confirmation", meta: "config clarity requested in review", tone: "amber", label: "hold" },
  { rank: "04", title: "Required CI checks passing", meta: "all required checks are green", tone: "green", label: "clear" },
];

const EVIDENCE = [
  { title: "ReviewBee pressure imported", meta: "5 open asks collapsed from review threads", label: "warn", tone: "amber" },
  { title: "TrustGate clear", meta: "policy risk low, no sensitive-path hit", label: "clear", tone: "green" },
  { title: "RepoMemory expectation present", meta: "startup-check note recommended", label: "watch", tone: "signal" },
];

const HISTORY = [
  { title: "repo-reaper#41", meta: "ready after ReviewBee checklist cleared", label: "ready", tone: "green" },
  { title: "trust-gate#27", meta: "held on policy warning and missing reviewer", label: "hold", tone: "amber" },
  { title: "signal-hive#33", meta: "checks green and no unresolved reviews", label: "ready", tone: "green" },
];

function ReadinessMap() {
  const [filter, setFilter] = useState("all");
  const [activeSignal, setActiveSignal] = useState(SIGNALS[0]);
  const visibleSignals = SIGNALS.filter((signal) => filter === "all" || signal.source === filter);
  const visibleIds = new Set(visibleSignals.map((signal) => signal.id));
  const visibleLinks = LINKS.filter((link) => visibleIds.has(link.from) && visibleIds.has(link.to));

  const changeFilter = (nextFilter) => {
    setFilter(nextFilter);
    const nextSignal = SIGNALS.find((signal) => nextFilter === "all" || signal.source === nextFilter);
    if (nextSignal) {
      setActiveSignal(nextSignal);
    }
  };

  return (
    <div className="signal-map merge-map" data-window="14">
      <div className="range-panel">
        <span className="chip signal">{visibleSignals.length} signals</span>
        <div className="range-switch" aria-label="Readiness signal filter">
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
      <div className="merge-scope" aria-label="Merge readiness scope">
        <span className="merge-core">
          <span className="memory-core-title">MergeKeeper</span>
          <span className="memory-core-count">HOLD</span>
          <span className="micro">readiness call</span>
        </span>
        {visibleLinks.map((link) => (
          <span className="memory-link merge-link" key={`${link.from}-${link.to}`} style={link.style} />
        ))}
        {visibleSignals.map((signal) => (
          <button
            aria-label={`Show signal ${signal.id}`}
            className={`memory-node merge-node ${signal.tone}${activeSignal.id === signal.id ? " active" : ""}`}
            data-label={signal.id}
            key={signal.id}
            onClick={() => setActiveSignal(signal)}
            style={signal.position}
            type="button"
          />
        ))}
        <span className="merge-axis axis-a" />
        <span className="merge-axis axis-b" />
      </div>
      <div className="radar-readout merge-readout">
        <div className="readout-card">
          <span className="label">Selected signal</span>
          <span className={`readout-value ${activeSignal.tone === "amber" || activeSignal.tone === "red" ? "warn" : ""}`}>{activeSignal.id}</span>
          <span className="micro">{activeSignal.source} source</span>
        </div>
        <div className="readout-card">
          <span className="label">State</span>
          <span className="readout-value">{activeSignal.state}</span>
          <span className="micro">{activeSignal.value}</span>
        </div>
        <div className="readout-card selected-scan">
          <span className="label">Readiness reason</span>
          <span className="readout-value">{activeSignal.title}</span>
          <span className="micro">{activeSignal.summary}</span>
        </div>
      </div>
    </div>
  );
}

function BlockerPanel() {
  return (
    <Panel eyebrow="Decision" title="Merge blockers" action={<span className="chip amber">hold</span>}>
      <div className="panelbody repo-list queue-grid">
        {BLOCKERS.map((item) => (
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
      <Panel eyebrow="Evidence" title="Suite inputs">
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
      <Panel eyebrow="Output" title="Publish posture">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">GitHub check</span><span className="chip amber">hold</span></div>
          <div className="rowline"><span className="muted">Maintained comment</span><span className="chip green">fresh</span></div>
          <div className="rowline"><span className="muted">HiveCore signal</span><span className="chip signal">ready</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function ReadinessSurface() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - merge readiness</div>
              <h1>Readiness Scope</h1>
              <p className="subline">GitHub state, review pressure, policy risk, and repo memory collapsed into one merge call.</p>
            </div>
            <div className="actions">
              <span className="chip amber">hold</span>
              <span className="chip signal">3 suite inputs</span>
              <button className="btn primary" type="button">Assess PR</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Readiness" title="Merge pressure map" action={<span className="chip signal">react port</span>}>
              <ReadinessMap />
            </Panel>
            <BlockerPanel />
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
        <div className="eyebrow">// MergeKeeper v2 extraction queue</div>
        <h1>Decision Log</h1>
        <p className="subline">Saved readiness calls and the evidence that changed them.</p>
      </div>
      <Panel eyebrow="Recent" title="Readiness history">
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
      <div className="eyebrow">// MergeKeeper v2 extraction queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("readiness");

  return (
    <>
      <DeckBar
        activeTab={activeTab}
        brandName="MergeKeeper frontend v2"
        navLabel="MergeKeeper v2 surfaces"
        onTabChange={setActiveTab}
        tabs={TABS}
      />
      {activeTab === "readiness" && <ReadinessSurface />}
      {activeTab === "history" && <HistorySurface />}
      {activeTab === "checks" && (
        <Placeholder
          title="Checks"
          body="This becomes the shared v2 GitHub, webhook, and integration readiness surface."
        />
      )}
    </>
  );
}

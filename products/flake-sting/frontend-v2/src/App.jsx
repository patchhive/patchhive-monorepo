import { useState } from "react";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductRail,
  SuiteRadar,
  SuiteTopline,
} from "@patchhivehq/ui-v2";

const TABS = [
  { id: "instability", label: "Instability" },
  { id: "history", label: "Scan history" },
  { id: "checks", label: "Checks" },
];

const TOPLINE_CELLS = [
  { label: "FlakeSting", value: "CI array 06", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Read only" },
  { label: "GitHub", value: "Actions read", tone: "sig" },
  { label: "Lookback", value: "25 runs", tone: "warn" },
  { label: "Last scan", value: "T-02:00" },
];

const RAIL_SECTIONS = [
  {
    title: "Workflows",
    items: [
      { label: "ci.yml / test", active: true, pin: true },
      { label: "release.yml", value: "clear" },
      { label: "lint.yml", value: "watch" },
      { label: "nightly.yml", value: "noisy" },
    ],
  },
  {
    title: "Signals",
    items: [
      { label: "fail/pass swing", active: true, badge: "9", badgeTone: "amber" },
      { label: "rerun pressure", badge: "6", badgeTone: "red" },
      { label: "runner skew", badge: "3", badgeTone: "signal" },
      { label: "stable jobs", badge: "18", badgeTone: "green" },
    ],
  },
];

const RAIL_STATS = {
  title: "Active repo",
  items: [
    { label: "Repository", value: "patchhive/repo-reaper" },
    { label: "CI trust", value: "WARN", large: true, tone: "warn" },
    { label: "Candidate", value: "quarantine" },
  ],
};

const METRICS = [
  { label: "Flake score", value: "72", tone: "warn", sub: "+18 vs last scan" },
  { label: "Swing jobs", value: "9", tone: "warn", sub: "fail/pass flips" },
  { label: "Reruns", value: "6", tone: "hot", sub: "operator retries" },
  { label: "Runner skew", value: "3", tone: "sig", sub: "ubuntu-latest" },
  { label: "Stable jobs", value: "18", tone: "ok", sub: "safe signal" },
];

const SIGNALS = [
  {
    id: "FS-01",
    title: "api-route-tests",
    source: "swing",
    tone: "amber",
    state: "unstable",
    value: "7 flips",
    position: { left: "50%", top: "23%" },
    summary: "The same test job alternates pass and fail across adjacent runs without code touching the tested path.",
  },
  {
    id: "FS-02",
    title: "integration-linux",
    source: "rerun",
    tone: "red",
    state: "quarantine",
    value: "4 reruns",
    position: { left: "72%", top: "47%" },
    summary: "Manual reruns repeatedly turn failure into pass, which makes this check unsafe as a merge blocker.",
  },
  {
    id: "FS-03",
    title: "docs-link-check",
    source: "stable",
    tone: "green",
    state: "stable",
    value: "25 pass",
    position: { left: "59%", top: "75%" },
    summary: "The job stays green across the full lookback window and can be trusted as normal signal.",
  },
  {
    id: "FS-04",
    title: "browser-e2e",
    source: "runner",
    tone: "signal",
    state: "runner skew",
    value: "3 hosts",
    position: { left: "28%", top: "62%" },
    summary: "Failures cluster on one runner image and disappear on rerun, suggesting environment pressure.",
  },
  {
    id: "FS-05",
    title: "cargo-nextest",
    source: "swing",
    tone: "amber",
    state: "watch",
    value: "2 flips",
    position: { left: "31%", top: "35%" },
    summary: "Low-volume pass/fail movement is visible, but the evidence is not strong enough for quarantine.",
  },
];

const LINKS = [
  { from: "FS-01", to: "FS-05", style: { left: "35%", top: "29%", width: "120px", transform: "rotate(-16deg)" } },
  { from: "FS-01", to: "FS-02", style: { left: "52%", top: "35%", width: "126px", transform: "rotate(36deg)" } },
  { from: "FS-02", to: "FS-03", style: { left: "63%", top: "60%", width: "104px", transform: "rotate(109deg)" } },
  { from: "FS-04", to: "FS-03", style: { left: "34%", top: "68%", width: "170px", transform: "rotate(12deg)" } },
];

const FILTERS = [
  { id: "all", label: "all" },
  { id: "swing", label: "swing" },
  { id: "rerun", label: "rerun" },
  { id: "runner", label: "runner" },
];

const FLAKY_QUEUE = [
  { rank: "01", title: "integration-linux", meta: "manual reruns turn red to green", tone: "red", label: "quarantine" },
  { rank: "02", title: "api-route-tests", meta: "7 pass/fail swings in 25 runs", tone: "amber", label: "watch" },
  { rank: "03", title: "browser-e2e", meta: "failures cluster on one runner image", tone: "signal", label: "runner" },
  { rank: "04", title: "docs-link-check", meta: "stable across full lookback window", tone: "green", label: "stable" },
];

const EVIDENCE = [
  { title: "Rerun pressure detected", meta: "4 of 6 failed runs passed on retry", label: "high", tone: "red" },
  { title: "Runner image skew", meta: "failures cluster on ubuntu-latest 20260519", label: "watch", tone: "signal" },
  { title: "MergeKeeper caution", meta: "CI signal should be weighted lower for this PR", label: "warn", tone: "amber" },
];

const HISTORY = [
  { title: "repo-reaper / ci.yml", meta: "flake score rose from 54 to 72", label: "worse", tone: "amber" },
  { title: "signal-hive / lint.yml", meta: "stable across 30 runs", label: "clear", tone: "green" },
  { title: "trust-gate / test.yml", meta: "runner skew still present", label: "watch", tone: "signal" },
];

function InstabilityMap() {
  return (
    <SuiteRadar
      ariaLabel="FlakeSting CI instability radar"
      detailLabel="Instability reason"
      feed={[
        { text: "Manual reruns repeatedly turn failure into pass.", tone: "red" },
        { text: "Fail/pass swings are clustered in route and integration jobs.", tone: "amber" },
        { text: "Docs link checks are stable across the full lookback window." },
      ]}
      gainLabel="State"
      items={SIGNALS.map((signal) => ({
        ...signal,
        detail: signal.title,
        gain: signal.state,
        gainMeta: signal.value,
        label: signal.id,
        stats: [
          { label: "Source", value: signal.source },
          { label: "State", value: signal.state },
          { label: "Value", value: signal.value },
          { label: "Trust", value: signal.tone === "green" ? "stable" : "noisy" },
          { label: "Action", value: signal.tone === "red" ? "quarantine" : "watch" },
        ],
        vector: signal.id,
        vectorTone: signal.tone === "amber" || signal.tone === "red" ? "warn" : "",
      }))}
      signalLabel="signals"
      vectorLabel="Selected signal"
    />
  );
}

function FlakyQueuePanel() {
  return (
    <Panel eyebrow="Queue" title="Flaky candidates" action={<span className="chip amber">2 watch</span>}>
      <div className="panelbody repo-list queue-grid">
        {FLAKY_QUEUE.map((item) => (
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
      <Panel eyebrow="Evidence" title="Why it looks flaky">
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
          <div className="rowline"><span className="muted">MergeKeeper</span><span className="chip amber">caution</span></div>
          <div className="rowline"><span className="muted">ReleaseSentry</span><span className="chip signal">watch</span></div>
          <div className="rowline"><span className="muted">Human action</span><span className="chip red">quarantine</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function InstabilitySurface() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - CI trust</div>
              <h1>Instability Map</h1>
              <p className="subline">Workflow history, reruns, runner skew, and fail/pass swings turned into readable CI trust pressure.</p>
            </div>
            <div className="actions">
              <span className="chip amber">warn</span>
              <span className="chip signal">25 run lookback</span>
              <button className="btn primary" type="button">Scan workflow</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Instability" title="CI signal map" action={<span className="chip signal">react port</span>}>
              <InstabilityMap />
            </Panel>
            <FlakyQueuePanel />
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
        <div className="eyebrow">// FlakeSting v2 extraction queue</div>
        <h1>Scan History</h1>
        <p className="subline">Comparable CI scans and whether trust pressure is rising or improving.</p>
      </div>
      <Panel eyebrow="Recent" title="Workflow scans">
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
      <div className="eyebrow">// FlakeSting v2 extraction queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("instability");

  return (
    <>
      <DeckBar
        activeTab={activeTab}
        brandName="FlakeSting frontend v2"
        navLabel="FlakeSting v2 surfaces"
        onTabChange={setActiveTab}
        tabs={TABS}
      />
      {activeTab === "instability" && <InstabilitySurface />}
      {activeTab === "history" && <HistorySurface />}
      {activeTab === "checks" && (
        <Placeholder
          title="Checks"
          body="This becomes the shared v2 GitHub Actions and token readiness surface."
        />
      )}
    </>
  );
}

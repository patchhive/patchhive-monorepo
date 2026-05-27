import { useState } from "react";
import {
  DeckBar,
  MetricBand,
  Panel,
  PlaceholderSurface,
  ProductRail,
  SuiteRadar,
  SuiteTopline,
} from "@patchhivehq/ui-v2";
import {
  EVIDENCE,
  METRICS,
  QUEUE,
  RADAR_ECHOES,
  RADAR_REPOS,
  RADAR_WINDOWS,
} from "./data.js";

const TABS = [
  { id: "atlas", label: "Atlas board" },
  { id: "ledger", label: "Ops ledger" },
  { id: "floor", label: "Watch floor" },
];

const TOPLINE_CELLS = [
  { label: "SignalHive", value: "Scout array 07", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Read only" },
  { label: "GitHub", value: "Connected", tone: "sig" },
  { label: "Schedules", value: "3 active", tone: "warn" },
  { label: "Last sweep", value: "T-03:00" },
];

const RAIL_SECTIONS = [
  {
    title: "Presets",
    items: [
      { label: "nightly-rust", active: true, pin: true },
      { label: "ts-api-watch", value: "42" },
      { label: "python-backlog", value: "18" },
      { label: "maintainerbot", value: "9" },
    ],
  },
  {
    title: "Schedules",
    items: [
      { label: "daily-rust-q", active: true, badge: "on", badgeTone: "green" },
      { label: "weekly-ts", badge: "on", badgeTone: "green" },
      { label: "py-monthly", badge: "hold", badgeTone: "amber" },
    ],
  },
];

const RAIL_STATS = {
  title: "Last sweep",
  items: [
    { label: "Top target", value: "tokio-rs/tokio" },
    { label: "Signals", value: "142", large: true, tone: "warn" },
    { label: "Elapsed", value: "04m 12s" },
  ],
};

function RadarScope() {
  return (
    <SuiteRadar
      ariaLabel="Live maintenance signal radar"
      detailLabel="Selected repo scan"
      echoes={RADAR_ECHOES}
      feed={[
        { text: "Tokio crossed high-pressure range with stale backlog and duplicate clusters.", tone: "red" },
        { text: "Serde is holding in middle range; derive macro cluster remains visible.", tone: "amber" },
        { text: "Next.js is drifting outward as stale pressure falls after cleanup." },
      ]}
      gainLabel="Signal gain"
      itemQueryParam="repo"
      items={RADAR_REPOS.map((repo) => ({
        ...repo,
        id: repo.repo,
        detail: repo.repo,
        gain: repo.gain,
        gainMeta: "selected repo fusion",
        stats: [
          { label: "Score", value: repo.score },
          { label: "Stale", value: repo.stale },
          { label: "Dupes", value: repo.dupes },
          { label: "Markers", value: repo.markers },
          { label: "Trend", value: repo.trend },
        ],
        title: repo.repo,
        vector: repo.vector,
      }))}
      signalLabel="signals"
      vectorLabel="Sweep vector"
      windows={RADAR_WINDOWS}
    />
  );
}

function QueuePanel() {
  return (
    <Panel eyebrow="Queue" title="Ranked targets" action={<button className="btn" type="button">Export</button>}>
      <div className="panelbody repo-list">
        {QUEUE.map((item) => (
          <article className="repo-card" key={item.repo}>
            <div className="repo-head">
              <div>
                <div className="repo-name">{item.repo}</div>
                <div className="repo-meta">
                  {item.tags.map((tag, index) => (
                    <span className={`chip ${index === 1 ? item.scoreTone : index === 0 ? "signal" : ""}`} key={tag}>
                      {tag}
                    </span>
                  ))}
                </div>
              </div>
              <div className={`score ${item.scoreTone}`}>{item.score}</div>
            </div>
            {item.drivers ? (
              <div className="bargrid">
                {item.drivers.map((driver) => (
                  <div className="driver" key={driver.name}>
                    <span>{driver.name}</span>
                    <div className="bar"><span className={driver.tone} style={{ width: driver.width }} /></div>
                    <span className={driver.tone === "red" ? "hot" : "warn"}>{driver.value}</span>
                  </div>
                ))}
              </div>
            ) : (
              <p className="muted">{item.description}</p>
            )}
          </article>
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

      <Panel eyebrow="Guardrails" title="Scope control">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">Allowlist</span><span className="chip green">3 targets</span></div>
          <div className="rowline"><span className="muted">Denylist</span><span className="chip red">1 target</span></div>
          <div className="rowline"><span className="muted">Opt-out</span><span className="chip">0 targets</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function AtlasBoard() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - scan array</div>
              <h1>Signal Atlas</h1>
              <p className="subline">Read-only GitHub reconnaissance with visible scope, trend pressure, and evidence trails.</p>
            </div>
            <div className="actions">
              <span className="chip signal">8 repos max</span>
              <span className="chip">rust - typescript - python</span>
              <button className="btn primary" type="button">Run sweep</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <Panel
              eyebrow="Cartography"
              title="Field intensity map"
              action={<span className="chip signal">field radar</span>}
            >
              <RadarScope />
            </Panel>
            <QueuePanel />
          </div>
        </main>
        <SidePanels />
      </div>
    </>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("atlas");

  return (
    <>
      <DeckBar
        activeTab={activeTab}
        brandName="SignalHive frontend v2"
        navLabel="SignalHive v2 surfaces"
        onTabChange={setActiveTab}
        tabs={TABS}
      />
      {activeTab === "atlas" && <AtlasBoard />}
      {activeTab === "ledger" && (
        <PlaceholderSurface
          title="Ops Ledger"
          body="The ledger direction will become the v2 table, filter, and score-driver pattern."
        />
      )}
      {activeTab === "floor" && (
        <PlaceholderSurface
          title="Watch Floor"
          body="The watch-floor direction will become the v2 schedule, diagnostic, and suite-status pattern."
        />
      )}
    </>
  );
}

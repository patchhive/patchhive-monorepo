import { useEffect, useMemo, useState } from "react";
import {
  DeckBar,
  MetricBand,
  Panel,
  PlaceholderSurface,
  ProductRail,
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
  const params = useMemo(() => new URLSearchParams(window.location.search), []);
  const [windowDays, setWindowDays] = useState(() => {
    const raw = Number(params.get("window") || 7);
    return RADAR_WINDOWS[raw] ? raw : 7;
  });
  const visibleRepos = useMemo(
    () => RADAR_REPOS.filter((repo) => repo.minWindow <= windowDays),
    [windowDays],
  );
  const [selectedRepo, setSelectedRepo] = useState(() => {
    const requested = params.get("repo");
    return RADAR_REPOS.find((repo) => repo.repo === requested) || RADAR_REPOS[0];
  });

  useEffect(() => {
    if (selectedRepo.minWindow > windowDays) {
      setSelectedRepo(visibleRepos[0] || RADAR_REPOS[0]);
    }
  }, [selectedRepo, visibleRepos, windowDays]);

  const activeWindow = RADAR_WINDOWS[windowDays];
  const visibleEchoes = RADAR_ECHOES.filter((echo) => echo.minWindow <= windowDays);

  return (
    <div className="signal-map" data-window={windowDays}>
      <div className="radar-screen" aria-label="Live maintenance signal radar">
        <span className="radar-bearing n">000</span>
        <span className="radar-bearing e">090</span>
        <span className="radar-bearing s">180</span>
        <span className="radar-bearing w">270</span>
        <span className="range-label r1">{activeWindow.outer}</span>
        <span className="range-label r2">{activeWindow.mid}</span>
        <span className="range-label r3">{activeWindow.inner}</span>
        <span className="radar-density" />
        <span className="radar-sweep" />
        <span className="radar-line" />
        <span className="radar-trace trace-a" />
        <span className="radar-trace trace-b" />
        <span className="radar-trace trace-c" />
        {visibleRepos.map((repo) => (
          <button
            aria-label={`Show ${repo.repo} scan`}
            className={`node ${repo.tone}${selectedRepo.repo === repo.repo ? " active" : ""}`}
            data-label={repo.label}
            key={repo.repo}
            onClick={() => setSelectedRepo(repo)}
            style={{ ...repo.position, "--ping-delay": repo.pingDelay }}
            type="button"
          />
        ))}
        {visibleEchoes.map((echo, index) => (
          <span
            className={`echo ${echo.tone}`}
            key={`${echo.position.left}-${index}`}
            style={echo.position}
          />
        ))}
      </div>

      <div className="radar-readout">
        <div className="readout-card">
          <span className="label">Sweep vector</span>
          <span className="readout-value">{selectedRepo.vector}</span>
          <span className="micro">{activeWindow.label}</span>
        </div>
        <div className="readout-card">
          <span className="label">Signal gain</span>
          <span className="readout-value warn">{selectedRepo.gain}</span>
          <span className="micro">selected repo fusion</span>
        </div>
        <div className="readout-card selected-scan">
          <span className="label">Selected repo scan</span>
          <span className="readout-value">{selectedRepo.repo}</span>
          <div className="selected-grid">
            <div className="selected-stat"><span className="micro">Score</span><strong>{selectedRepo.score}</strong></div>
            <div className="selected-stat"><span className="micro">Stale</span><strong>{selectedRepo.stale}</strong></div>
            <div className="selected-stat"><span className="micro">Dupes</span><strong>{selectedRepo.dupes}</strong></div>
            <div className="selected-stat"><span className="micro">Markers</span><strong>{selectedRepo.markers}</strong></div>
            <div className="selected-stat"><span className="micro">Trend</span><strong>{selectedRepo.trend}</strong></div>
          </div>
          <span className="micro">{selectedRepo.summary}</span>
        </div>
        <div className="readout-feed">
          <div className="readout-line red">Tokio crossed high-pressure range with stale backlog and duplicate clusters.</div>
          <div className="readout-line amber">Serde is holding in middle range; derive macro cluster remains visible.</div>
          <div className="readout-line">Next.js is drifting outward as stale pressure falls after cleanup.</div>
        </div>
      </div>
      <div className="range-panel">
        <span className="chip signal">{activeWindow.count}</span>
        <div className="range-switch" aria-label="Radar history window">
          {Object.keys(RADAR_WINDOWS).map((days) => (
            <button
              className={`range-btn${windowDays === Number(days) ? " active" : ""}`}
              key={days}
              onClick={() => setWindowDays(Number(days))}
              type="button"
            >
              {days}d
            </button>
          ))}
        </div>
      </div>
    </div>
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
          <div className="atlas-layout">
            <Panel
              eyebrow="Cartography"
              title="Field intensity map"
              action={<span className="chip signal">react port</span>}
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

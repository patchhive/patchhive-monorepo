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
  { id: "scout", label: "Scout" },
  { id: "history", label: "Scan history" },
  { id: "checks", label: "Checks" },
];

const TOPLINE_CELLS = [
  { label: "RefactorScout", value: "Map 04", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Local read" },
  { label: "Scope", value: "Allowed root", tone: "sig" },
  { label: "Safety", value: "8 high", tone: "ok" },
  { label: "Last scan", value: "T-03:10" },
];

const RAIL_SECTIONS = [
  {
    title: "Roots",
    items: [
      { label: "/workspaces/patchhive", active: true, pin: true },
      { label: "max files", value: "250" },
      { label: "remote fs", value: "blocked" },
      { label: "ignored dirs", value: "9" },
    ],
  },
  {
    title: "Heuristics",
    items: [
      { label: "high safety", active: true, badge: "8", badgeTone: "green" },
      { label: "medium safety", badge: "17", badgeTone: "amber" },
      { label: "oversized files", badge: "6", badgeTone: "signal" },
      { label: "literal repeats", badge: "24", badgeTone: "signal" },
    ],
  },
];

const RAIL_STATS = {
  title: "Active repo",
  items: [
    { label: "Repository", value: "patchhive/patchhive" },
    { label: "Decision", value: "SCOUT", large: true, tone: "ok" },
    { label: "Focus", value: "safe first move" },
  ],
};

const METRICS = [
  { label: "High safety", value: "8", tone: "ok", sub: "low blast radius" },
  { label: "Medium", value: "17", tone: "warn", sub: "needs owner eye" },
  { label: "Large files", value: "6", tone: "sig", sub: "clear split points" },
  { label: "Repeat strings", value: "24", tone: "sig", sub: "constant candidates" },
  { label: "Files scanned", value: "250", tone: "ok", sub: "cap respected" },
];

const LEADS = [
  {
    id: "RS-01",
    title: "frontend panel split",
    bucket: "high",
    tone: "green",
    state: "high safety",
    value: "component split",
    position: { left: "60%", top: "28%" },
    summary: "One panel carries repeated row rendering and can be extracted without changing product behavior.",
  },
  {
    id: "RS-02",
    title: "repeated API path strings",
    bucket: "string",
    tone: "signal",
    state: "high safety",
    value: "24 repeats",
    position: { left: "37%", top: "38%" },
    summary: "Repeated endpoint literals can move into local constants and reduce accidental drift.",
  },
  {
    id: "RS-03",
    title: "oversized scan function",
    bucket: "medium",
    tone: "amber",
    state: "medium safety",
    value: "186 lines",
    position: { left: "72%", top: "62%" },
    summary: "The function is large and cohesive enough to split, but the data-flow needs a human pass.",
  },
  {
    id: "RS-04",
    title: "generated folder ignored",
    bucket: "ignore",
    tone: "green",
    state: "ignore",
    value: "safe skip",
    position: { left: "29%", top: "70%" },
    summary: "Generated output is noisy and correctly excluded from the cleanup queue.",
  },
  {
    id: "RS-05",
    title: "route helper extraction",
    bucket: "high",
    tone: "signal",
    state: "high safety",
    value: "shared helper",
    position: { left: "55%", top: "76%" },
    summary: "Two route modules repeat request parsing that can become a small local helper.",
  },
];

const LINKS = [
  { from: "RS-01", to: "RS-02", style: { left: "39%", top: "33%", width: "118px", transform: "rotate(-13deg)" } },
  { from: "RS-01", to: "RS-03", style: { left: "60%", top: "42%", width: "132px", transform: "rotate(69deg)" } },
  { from: "RS-02", to: "RS-04", style: { left: "31%", top: "51%", width: "122px", transform: "rotate(82deg)" } },
  { from: "RS-04", to: "RS-05", style: { left: "32%", top: "74%", width: "142px", transform: "rotate(8deg)" } },
];

const FILTERS = [
  { id: "all", label: "all" },
  { id: "high", label: "high" },
  { id: "medium", label: "medium" },
  { id: "string", label: "strings" },
  { id: "ignore", label: "ignore" },
];

const LEAD_QUEUE = [
  { rank: "01", title: "frontend panel split", meta: "extract repeated row rendering from one panel", tone: "green", label: "high" },
  { rank: "02", title: "repeated API path strings", meta: "move 24 endpoint literals into constants", tone: "signal", label: "safe" },
  { rank: "03", title: "route helper extraction", meta: "two modules repeat request parsing", tone: "signal", label: "high" },
  { rank: "04", title: "oversized scan function", meta: "split after owner checks data-flow", tone: "amber", label: "medium" },
];

const EVIDENCE = [
  { title: "Low blast radius", meta: "8 leads avoid business logic and persistence boundaries", label: "safe", tone: "green" },
  { title: "Explicit root respected", meta: "scan stayed inside configured local allowlist", label: "local", tone: "signal" },
  { title: "Medium leads held back", meta: "17 items need owner review before scheduling", label: "watch", tone: "amber" },
];

const HISTORY = [
  { title: "patchhive / local scan", meta: "high-safety leads rose from 6 to 8", label: "better", tone: "green" },
  { title: "repo-memory / panels", meta: "large component split still pending", label: "queue", tone: "signal" },
  { title: "trust-gate / routes", meta: "request parsing helper remains medium-safety", label: "watch", tone: "amber" },
];

function RefactorMap() {
  return (
    <SuiteRadar
      ariaLabel="RefactorScout opportunity radar"
      detailLabel="Suggested first move"
      feed={[
        { text: "High-safety leads avoid persistence and business logic boundaries.", tone: "green" },
        { text: "Repeated API path strings are a safe constants extraction." },
        { text: "Oversized scan function stays medium until owner reviews data flow.", tone: "amber" },
      ]}
      gainLabel="Safety"
      items={LEADS.map((lead) => ({
        ...lead,
        detail: lead.title,
        gain: lead.state,
        gainMeta: lead.value,
        label: lead.id,
        stats: [
          { label: "Bucket", value: lead.bucket },
          { label: "Safety", value: lead.state },
          { label: "Move", value: lead.value },
          { label: "Blast", value: lead.tone === "amber" ? "medium" : "low" },
          { label: "Action", value: lead.tone === "green" ? "schedule" : "review" },
        ],
        vector: lead.id,
        vectorTone: lead.tone === "amber" ? "warn" : "",
      }))}
      signalLabel="leads"
      vectorLabel="Selected lead"
    />
  );
}

function LeadQueuePanel() {
  return (
    <Panel eyebrow="Queue" title="Refactor leads" action={<span className="chip green">8 high</span>}>
      <div className="panelbody repo-list queue-grid">
        {LEAD_QUEUE.map((item) => (
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
      <Panel eyebrow="Evidence" title="Why it is safe">
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
          <div className="rowline"><span className="muted">RepoMemory</span><span className="chip signal">context</span></div>
          <div className="rowline"><span className="muted">TrustGate</span><span className="chip green">safe</span></div>
          <div className="rowline"><span className="muted">RepoReaper</span><span className="chip amber">later</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function ScoutSurface() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - conservative cleanup</div>
              <h1>Refactor Opportunity</h1>
              <p className="subline">Local repo paths, explicit scan caps, and explainable heuristics turned into safe cleanup leads.</p>
            </div>
            <div className="actions">
              <span className="chip green">high safety</span>
              <span className="chip signal">local read</span>
              <button className="btn primary" type="button">Scan path</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Scout" title="Opportunity map" action={<span className="chip signal">scout radar</span>}>
              <RefactorMap />
            </Panel>
            <LeadQueuePanel />
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
        <div className="eyebrow">// RefactorScout v2 extraction queue</div>
        <h1>Scan History</h1>
        <p className="subline">Saved local scans with high-safety lead movement and ignored-path evidence.</p>
      </div>
      <Panel eyebrow="Recent" title="Local scans">
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
      <div className="eyebrow">// RefactorScout v2 extraction queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("scout");

  return (
    <>
      <DeckBar
        activeTab={activeTab}
        brandName="RefactorScout frontend v2"
        navLabel="RefactorScout v2 surfaces"
        onTabChange={setActiveTab}
        tabs={TABS}
      />
      {activeTab === "scout" && <ScoutSurface />}
      {activeTab === "history" && <HistorySurface />}
      {activeTab === "checks" && (
        <Placeholder
          title="Checks"
          body="This becomes the shared v2 local filesystem allowlist, remote scan guardrail, and backend readiness surface."
        />
      )}
    </>
  );
}

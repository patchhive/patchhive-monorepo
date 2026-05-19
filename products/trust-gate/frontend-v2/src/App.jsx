import { useState } from "react";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductRail,
  SuiteTopline,
} from "@patchhivehq/ui-v2";

const TABS = [
  { id: "review", label: "Risk review" },
  { id: "rules", label: "Rule packs" },
  { id: "history", label: "Decision log" },
];

const TOPLINE_CELLS = [
  { label: "TrustGate", value: "Policy gate 02", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Review only" },
  { label: "GitHub", value: "Optional", tone: "warn" },
  { label: "FailGuard", value: "Armed", tone: "sig" },
  { label: "Last decision", value: "T-11:00" },
];

const RAIL_SECTIONS = [
  {
    title: "Rule packs",
    items: [
      { label: "ai-diff-baseline", active: true, pin: true },
      { label: "sensitive-paths", value: "12" },
      { label: "missing-tests", value: "7" },
      { label: "scope-caps", value: "4" },
    ],
  },
  {
    title: "Sinks",
    items: [
      { label: "github checks", badge: "ready", badgeTone: "green" },
      { label: "pr comment", badge: "ready", badgeTone: "green" },
      { label: "failguard", active: true, badge: "on", badgeTone: "amber" },
    ],
  },
];

const RAIL_STATS = {
  title: "Current diff",
  items: [
    { label: "Repository", value: "patchhive/signalhive" },
    { label: "Risk", value: "WARN", large: true, tone: "warn" },
    { label: "Elapsed", value: "00m 42s" },
  ],
};

const METRICS = [
  { label: "Recommendation", value: "WARN", tone: "warn", sub: "review required" },
  { label: "Risk score", value: "68", tone: "hot", sub: "+12 vs baseline" },
  { label: "Files touched", value: "9", tone: "sig", sub: "2 sensitive" },
  { label: "Rule hits", value: "14", tone: "warn", sub: "4 high-signal" },
  { label: "Tests found", value: "3", tone: "ok", sub: "coverage present" },
];

const RULE_HITS = [
  { title: "Sensitive path touched", meta: "backend/src/auth.rs - auth boundary", tone: "red", label: "blockable" },
  { title: "Generated diff exceeds normal size", meta: "624 changed lines - scope cap is 500", tone: "amber", label: "warn" },
  { title: "Tests changed with implementation", meta: "unit and route tests present", tone: "green", label: "offset" },
];

const FILES = [
  { path: "backend/src/auth.rs", risk: "high", change: "+92 -21", tone: "red" },
  { path: "backend/src/routes/review.rs", risk: "medium", change: "+211 -44", tone: "amber" },
  { path: "frontend/src/App.jsx", risk: "low", change: "+88 -16", tone: "green" },
  { path: "backend/tests/review.rs", risk: "offset", change: "+42 -0", tone: "green" },
];

function DecisionGauge() {
  return (
    <div className="signal-map trust-map" data-window="14">
      <div className="trust-gauge">
        <div className="trust-ring">
          <span className="trust-decision">WARN</span>
          <span className="trust-score">68</span>
          <span className="micro">policy confidence 0.74</span>
        </div>
      </div>
      <div className="radar-readout">
        <div className="readout-card">
          <span className="label">Primary reason</span>
          <span className="readout-value warn">scope</span>
          <span className="micro">change size plus auth path</span>
        </div>
        <div className="readout-card">
          <span className="label">FailGuard</span>
          <span className="readout-value warn">queued</span>
          <span className="micro">candidate lesson prepared</span>
        </div>
        <div className="readout-card selected-scan">
          <span className="label">Review summary</span>
          <span className="readout-value">patchhive/signalhive</span>
          <span className="micro">
            The diff is probably reviewable, but it crosses an auth boundary and exceeds the normal scope cap. Require maintainer review before any downstream automation.
          </span>
        </div>
      </div>
    </div>
  );
}

function RuleHitPanel() {
  return (
    <Panel eyebrow="Policy" title="Rule hits" action={<span className="chip amber">14 hits</span>}>
      <div className="panelbody repo-list">
        {RULE_HITS.map((item) => (
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
  );
}

function FileRiskPanel() {
  return (
    <Panel eyebrow="Diff" title="File risk matrix" action={<button className="btn" type="button">Export</button>}>
      <div className="panelbody repo-list">
        {FILES.map((file) => (
          <div className="ledger-row" key={file.path}>
            <div className="rank">{file.risk}</div>
            <div>
              <div className="repo-name">{file.path}</div>
              <div className="muted">{file.change}</div>
            </div>
            <span className={`chip ${file.tone}`}>{file.risk}</span>
          </div>
        ))}
      </div>
    </Panel>
  );
}

function ReviewSurface() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - risk gate</div>
              <h1>Trust Review</h1>
              <p className="subline">Diff risk made visible before maintainers or downstream automation move a change forward.</p>
            </div>
            <div className="actions">
              <span className="chip amber">warn</span>
              <span className="chip">9 files</span>
              <button className="btn primary" type="button">Review diff</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout">
            <Panel eyebrow="Decision" title="Safety recommendation" action={<span className="chip amber">warn</span>}>
              <DecisionGauge />
            </Panel>
            <RuleHitPanel />
          </div>
        </main>
        <aside className="side">
          <FileRiskPanel />
          <Panel eyebrow="Output" title="Publish posture">
            <div className="panelbody repo-list">
              <div className="rowline"><span className="muted">GitHub status</span><span className="chip amber">pending review</span></div>
              <div className="rowline"><span className="muted">Maintained comment</span><span className="chip green">ready</span></div>
              <div className="rowline"><span className="muted">FailGuard candidate</span><span className="chip amber">prepared</span></div>
            </div>
          </Panel>
        </aside>
      </div>
    </>
  );
}

function Placeholder({ title, body }) {
  return (
    <div className="placeholder-shell">
      <div className="eyebrow">// TrustGate v2 extraction queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("review");

  return (
    <>
      <DeckBar
        activeTab={activeTab}
        brandName="TrustGate frontend v2"
        navLabel="TrustGate v2 surfaces"
        onTabChange={setActiveTab}
        tabs={TABS}
      />
      {activeTab === "review" && <ReviewSurface />}
      {activeTab === "rules" && <Placeholder title="Rule Packs" body="This will become the shared v2 rule and policy editor surface." />}
      {activeTab === "history" && <Placeholder title="Decision Log" body="This will become the shared v2 history and evidence timeline pattern." />}
    </>
  );
}

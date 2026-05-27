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
  { id: "triage", label: "Triage" },
  { id: "history", label: "Scan history" },
  { id: "checks", label: "Checks" },
];

const TOPLINE_CELLS = [
  { label: "VulnTriage", value: "Queue 09", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Read only" },
  { label: "GitHub", value: "Security read", tone: "sig" },
  { label: "Severity", value: "7 high+", tone: "warn" },
  { label: "Last scan", value: "T-00:48" },
];

const RAIL_SECTIONS = [
  {
    title: "Feeds",
    items: [
      { label: "Code scanning", active: true, pin: true },
      { label: "Dependabot alerts", value: "18" },
      { label: "Owner hints", value: "9" },
      { label: "Dismissed noise", value: "12" },
    ],
  },
  {
    title: "Buckets",
    items: [
      { label: "fix now", active: true, badge: "5", badgeTone: "red" },
      { label: "plan next", badge: "12", badgeTone: "amber" },
      { label: "watch", badge: "18", badgeTone: "green" },
      { label: "needs access", badge: "2", badgeTone: "signal" },
    ],
  },
];

const RAIL_STATS = {
  title: "Active repo",
  items: [
    { label: "Repository", value: "patchhive/repo-reaper" },
    { label: "Decision", value: "HOLD", large: true, tone: "hot" },
    { label: "Focus", value: "reachable paths" },
  ],
};

const METRICS = [
  { label: "Fix now", value: "5", tone: "hot", sub: "reachable or high" },
  { label: "Plan next", value: "12", tone: "warn", sub: "owner follow-up" },
  { label: "Watch", value: "18", tone: "ok", sub: "low exposure" },
  { label: "High/Critical", value: "7", tone: "hot", sub: "severity pressure" },
  { label: "Reachable", value: "4", tone: "sig", sub: "runtime proxy" },
];

const FINDINGS = [
  {
    id: "VT-01",
    title: "SQL injection sink",
    bucket: "fix",
    tone: "red",
    state: "fix now",
    value: "reachable",
    position: { left: "69%", top: "33%" },
    summary: "Code scanning points at a request path with user input and no clear sanitizer evidence.",
  },
  {
    id: "VT-02",
    title: "transitive crypto advisory",
    bucket: "dep",
    tone: "red",
    state: "fix now",
    value: "high alert",
    position: { left: "57%", top: "70%" },
    summary: "Dependency alert affects a runtime package, and the upgrade path is already available.",
  },
  {
    id: "VT-03",
    title: "token-like string",
    bucket: "plan",
    tone: "amber",
    state: "plan next",
    value: "secret proxy",
    position: { left: "36%", top: "31%" },
    summary: "The finding needs owner confirmation, but the path looks operational rather than test-only.",
  },
  {
    id: "VT-04",
    title: "dev dependency advisory",
    bucket: "watch",
    tone: "green",
    state: "watch",
    value: "dev only",
    position: { left: "29%", top: "67%" },
    summary: "Severity is real, but current evidence keeps it outside production runtime paths.",
  },
  {
    id: "VT-05",
    title: "path traversal warning",
    bucket: "plan",
    tone: "amber",
    state: "plan next",
    value: "needs owner",
    position: { left: "76%", top: "58%" },
    summary: "The alert is near archive extraction code and should be reviewed before the next release.",
  },
];

const LINKS = [
  { from: "VT-01", to: "VT-05", style: { left: "68%", top: "42%", width: "112px", transform: "rotate(73deg)" } },
  { from: "VT-01", to: "VT-03", style: { left: "40%", top: "31%", width: "170px", transform: "rotate(3deg)" } },
  { from: "VT-02", to: "VT-04", style: { left: "34%", top: "69%", width: "142px", transform: "rotate(5deg)" } },
  { from: "VT-05", to: "VT-02", style: { left: "61%", top: "64%", width: "90px", transform: "rotate(162deg)" } },
];

const FILTERS = [
  { id: "all", label: "all" },
  { id: "fix", label: "fix" },
  { id: "plan", label: "plan" },
  { id: "watch", label: "watch" },
  { id: "dep", label: "dep" },
];

const FIX_QUEUE = [
  { rank: "01", title: "SQL injection sink", meta: "reachable request path; no sanitizer evidence", tone: "red", label: "fix now" },
  { rank: "02", title: "transitive crypto advisory", meta: "runtime package with patched version available", tone: "red", label: "fix now" },
  { rank: "03", title: "path traversal warning", meta: "near archive extraction before release", tone: "amber", label: "plan" },
  { rank: "04", title: "dev dependency advisory", meta: "real CVE, but outside runtime path", tone: "green", label: "watch" },
];

const EVIDENCE = [
  { title: "Reachability proxy is high", meta: "4 findings sit on runtime or request paths", label: "high", tone: "red" },
  { title: "Owner hints resolved", meta: "9 findings map to clear code owners", label: "routed", tone: "signal" },
  { title: "Watch bucket is defensible", meta: "18 findings lack runtime exposure evidence", label: "watch", tone: "green" },
];

const HISTORY = [
  { title: "repo-reaper / security scan", meta: "fix-now bucket stayed at 5 after owner routing", label: "hold", tone: "red" },
  { title: "dep-triage / advisory scan", meta: "dependency alerts dropped by 3", label: "better", tone: "green" },
  { title: "trust-gate / code scanning", meta: "path traversal warning still needs review", label: "plan", tone: "amber" },
];

function VulnerabilityMap() {
  return (
    <SuiteRadar
      ariaLabel="VulnTriage vulnerability pressure radar"
      detailLabel="Triage reason"
      feed={[
        { text: "Reachability proxy keeps SQL injection at the top of the queue.", tone: "red" },
        { text: "Path traversal warning should be reviewed before release.", tone: "amber" },
        { text: "Dev dependency advisory remains defensible watch noise." },
      ]}
      gainLabel="Decision"
      items={FINDINGS.map((finding) => ({
        ...finding,
        detail: finding.title,
        gain: finding.state,
        gainMeta: finding.value,
        label: finding.id,
        stats: [
          { label: "Bucket", value: finding.bucket },
          { label: "State", value: finding.state },
          { label: "Value", value: finding.value },
          { label: "Action", value: finding.tone === "red" ? "fix" : finding.tone === "green" ? "watch" : "plan" },
          { label: "Owner", value: "routed" },
        ],
        vector: finding.id,
        vectorTone: finding.tone === "amber" || finding.tone === "red" ? "warn" : "",
      }))}
      signalLabel="findings"
      vectorLabel="Selected finding"
    />
  );
}

function FixQueuePanel() {
  return (
    <Panel eyebrow="Queue" title="Security decisions" action={<span className="chip red">5 fix</span>}>
      <div className="panelbody repo-list queue-grid">
        {FIX_QUEUE.map((item) => (
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
      <Panel eyebrow="Evidence" title="Why it ranks high">
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
          <div className="rowline"><span className="muted">ReleaseSentry</span><span className="chip red">hold</span></div>
          <div className="rowline"><span className="muted">TrustGate</span><span className="chip amber">rules</span></div>
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
              <div className="eyebrow">// Module - security triage</div>
              <h1>Vulnerability Pressure</h1>
              <p className="subline">Code scanning alerts, Dependabot advisories, owner hints, and reachability proxies ranked into engineering work.</p>
            </div>
            <div className="actions">
              <span className="chip red">hold release</span>
              <span className="chip signal">read-only</span>
              <button className="btn primary" type="button">Scan findings</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Triage" title="Finding map" action={<span className="chip signal">react port</span>}>
              <VulnerabilityMap />
            </Panel>
            <FixQueuePanel />
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
        <div className="eyebrow">// VulnTriage v2 extraction queue</div>
        <h1>Scan History</h1>
        <p className="subline">Saved security snapshots with bucket movement, owner routing, and release pressure.</p>
      </div>
      <Panel eyebrow="Recent" title="Security scans">
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
      <div className="eyebrow">// VulnTriage v2 extraction queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("triage");

  return (
    <>
      <DeckBar
        activeTab={activeTab}
        brandName="VulnTriage frontend v2"
        navLabel="VulnTriage v2 surfaces"
        onTabChange={setActiveTab}
        tabs={TABS}
      />
      {activeTab === "triage" && <TriageSurface />}
      {activeTab === "history" && <HistorySurface />}
      {activeTab === "checks" && (
        <Placeholder
          title="Checks"
          body="This becomes the shared v2 GitHub security permissions, code scanning, Dependabot alert, and backend readiness surface."
        />
      )}
    </>
  );
}

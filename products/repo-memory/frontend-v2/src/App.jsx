import { useState } from "react";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductRail,
  SuiteTopline,
} from "@patchhivehq/ui-v2";

const TABS = [
  { id: "core", label: "Memory core" },
  { id: "failguard", label: "FailGuard" },
  { id: "packs", label: "Prompt packs" },
];

const TOPLINE_CELLS = [
  { label: "RepoMemory", value: "Memory spine 03", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Context first" },
  { label: "GitHub", value: "Connected", tone: "sig" },
  { label: "FailGuard", value: "7 queued", tone: "warn" },
  { label: "Last ingest", value: "T-19:00" },
];

const RAIL_SECTIONS = [
  {
    title: "Repositories",
    items: [
      { label: "patchhive/reporeaper", active: true, pin: true },
      { label: "patchhive/signalhive", value: "42" },
      { label: "patchhive/trustgate", value: "31" },
      { label: "patchhive/hivecore", value: "18" },
    ],
  },
  {
    title: "Curation",
    items: [
      { label: "pinned policy", active: true, badge: "27", badgeTone: "green" },
      { label: "soft context", badge: "112", badgeTone: "signal" },
      { label: "needs review", badge: "14", badgeTone: "amber" },
      { label: "suppressed", badge: "9" },
    ],
  },
];

const RAIL_STATS = {
  title: "Active repo",
  items: [
    { label: "Repository", value: "patchhive/reporeaper" },
    { label: "Context coverage", value: "92%", large: true, tone: "green" },
    { label: "Prompt pack", value: "fresh" },
  ],
};

const METRICS = [
  { label: "Memories", value: "184", tone: "sig", sub: "+23 from ingest" },
  { label: "Pinned policies", value: "27", tone: "ok", sub: "guardrail ready" },
  { label: "FailGuard queue", value: "7", tone: "warn", sub: "3 high priority" },
  { label: "Prompt packs", value: "4", tone: "sig", sub: "2 refreshed" },
  { label: "Drift", value: "+12", tone: "warn", sub: "review themes changed" },
];

const FILTERS = [
  { id: "all", label: "all" },
  { id: "policy", label: "policy" },
  { id: "failure", label: "failure" },
  { id: "review", label: "review" },
];

const MEMORY_NODES = [
  {
    id: "M-143",
    title: "Auth boundary expectations",
    lane: "policy",
    type: "policy",
    confidence: "0.91",
    evidence: "18 merged PRs",
    tone: "green",
    position: { left: "51%", top: "25%" },
    summary: "Keep auth middleware small, explicit, and backed by route-level tests before other products consume the service.",
  },
  {
    id: "M-118",
    title: "Retry semantics are product-owned",
    lane: "review",
    type: "review",
    confidence: "0.82",
    evidence: "9 review threads",
    tone: "signal",
    position: { left: "73%", top: "43%" },
    summary: "Reviewers repeatedly reject hidden retries unless the UI surfaces retry count, last failure, and operator control.",
  },
  {
    id: "M-097",
    title: "Scope creep in broad refactors",
    lane: "failure",
    type: "failure",
    confidence: "0.88",
    evidence: "5 rejected patches",
    tone: "amber",
    position: { left: "43%", top: "67%" },
    summary: "Large cleanup patches drift into behavior changes. Split mechanical cleanup from product behavior before RepoReaper acts.",
  },
  {
    id: "M-071",
    title: "Config drift breaks local launch",
    lane: "failure",
    type: "failure",
    confidence: "0.79",
    evidence: "3 FailGuard lessons",
    tone: "red",
    position: { left: "25%", top: "47%" },
    summary: "Docker ports, frontend URLs, and API defaults need matching updates or HiveCore reports false product failures.",
  },
  {
    id: "M-052",
    title: "Reviewer wants evidence first",
    lane: "review",
    type: "review",
    confidence: "0.86",
    evidence: "14 comments",
    tone: "signal",
    position: { left: "62%", top: "78%" },
    summary: "Patch explanations land better when risk evidence and test output appear before implementation narrative.",
  },
];

const MEMORY_LINKS = [
  { from: "M-143", to: "M-118", style: { left: "52%", top: "31%", width: "128px", transform: "rotate(28deg)" } },
  { from: "M-118", to: "M-097", style: { left: "54%", top: "54%", width: "145px", transform: "rotate(142deg)" } },
  { from: "M-097", to: "M-071", style: { left: "30%", top: "57%", width: "102px", transform: "rotate(210deg)" } },
  { from: "M-097", to: "M-052", style: { left: "46%", top: "72%", width: "82px", transform: "rotate(20deg)" } },
  { from: "M-071", to: "M-143", style: { left: "31%", top: "36%", width: "122px", transform: "rotate(-22deg)" } },
];

const MEMORY_QUEUE = [
  { rank: "01", title: "Promote config drift lesson", meta: "FailGuard candidate - affects launcher and HiveCore", tone: "amber", label: "review" },
  { rank: "02", title: "Pin auth boundary policy", meta: "High confidence - TrustGate should consume this", tone: "green", label: "pin" },
  { rank: "03", title: "Refresh retry prompt pack", meta: "Review pressure changed after 9 threads", tone: "signal", label: "pack" },
  { rank: "04", title: "Suppress stale fixture style note", meta: "No longer appears in recent PR history", tone: "", label: "soft" },
];

const PROMPT_PACK = [
  { title: "Repo contract", meta: "Auth, startup checks, service tokens", label: "fresh", tone: "green" },
  { title: "Review posture", meta: "Evidence first, small patches, test output", label: "ready", tone: "signal" },
  { title: "Known failure modes", meta: "Config drift, hidden retries, broad refactors", label: "warn", tone: "amber" },
];

const FAILGUARD = [
  { title: "Rejected patch became policy candidate", meta: "RepoReaper Smith - confidence below threshold", label: "high", tone: "red" },
  { title: "TrustGate warn submitted lesson", meta: "Sensitive path without explicit route test", label: "review", tone: "amber" },
  { title: "Operator captured local launch failure", meta: "Port mismatch after frontend v2 test", label: "queued", tone: "signal" },
];

function MemoryLattice() {
  const [filter, setFilter] = useState("all");
  const [activeNode, setActiveNode] = useState(MEMORY_NODES[0]);
  const visibleNodes = MEMORY_NODES.filter((node) => filter === "all" || node.lane === filter);
  const visibleIds = new Set(visibleNodes.map((node) => node.id));
  const visibleLinks = MEMORY_LINKS.filter((link) => visibleIds.has(link.from) && visibleIds.has(link.to));

  const changeFilter = (nextFilter) => {
    setFilter(nextFilter);
    const firstNode = MEMORY_NODES.find((node) => nextFilter === "all" || node.lane === nextFilter);
    if (firstNode) {
      setActiveNode(firstNode);
    }
  };

  return (
    <div className="signal-map memory-map" data-window="14">
      <div className="memory-lattice" aria-label="Repo memory lattice">
        <span className="memory-core">
          <span className="memory-core-title">RepoMemory</span>
          <span className="memory-core-count">184</span>
          <span className="micro">curated memories</span>
        </span>
        {visibleLinks.map((link) => (
          <span className="memory-link" key={`${link.from}-${link.to}`} style={link.style} />
        ))}
        {visibleNodes.map((node) => (
          <button
            aria-label={`Show memory ${node.id}`}
            className={`memory-node ${node.tone}${activeNode.id === node.id ? " active" : ""}`}
            data-label={node.id}
            key={node.id}
            onClick={() => setActiveNode(node)}
            style={node.position}
            type="button"
          />
        ))}
        <span className="memory-strand strand-a" />
        <span className="memory-strand strand-b" />
        <span className="memory-strand strand-c" />
      </div>

      <div className="radar-readout">
        <div className="readout-card">
          <span className="label">Selected memory</span>
          <span className={`readout-value ${activeNode.tone === "amber" ? "warn" : ""}`}>{activeNode.id}</span>
          <span className="micro">{activeNode.type} lane</span>
        </div>
        <div className="readout-card">
          <span className="label">Confidence</span>
          <span className="readout-value">{activeNode.confidence}</span>
          <span className="micro">{activeNode.evidence}</span>
        </div>
        <div className="readout-card selected-scan">
          <span className="label">Memory detail</span>
          <span className="readout-value">{activeNode.title}</span>
          <div className="selected-grid memory-selected-grid">
            <div className="selected-stat"><span className="micro">Kind</span><strong>{activeNode.type}</strong></div>
            <div className="selected-stat"><span className="micro">State</span><strong>curated</strong></div>
            <div className="selected-stat"><span className="micro">Uses</span><strong>TrustGate</strong></div>
            <div className="selected-stat"><span className="micro">Pack</span><strong>ready</strong></div>
          </div>
          <span className="micro">{activeNode.summary}</span>
        </div>
        <div className="readout-feed">
          <div className="readout-line amber">FailGuard candidates are feeding pinned policy, not automatic action.</div>
          <div className="readout-line">Prompt pack is fresh enough for TrustGate and RepoReaper context handoff.</div>
          <div className="readout-line red">Config drift remains the highest-risk repeated failure pattern.</div>
        </div>
      </div>

      <div className="range-panel">
        <span className="chip signal">{visibleNodes.length} nodes</span>
        <div className="range-switch" aria-label="Memory lane filter">
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
    </div>
  );
}

function MemoryQueuePanel() {
  return (
    <Panel eyebrow="Curation" title="Review queue" action={<span className="chip amber">14 pending</span>}>
      <div className="panelbody repo-list">
        {MEMORY_QUEUE.map((item) => (
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

function PromptPackPanel() {
  return (
    <Panel eyebrow="Context" title="Prompt pack" action={<button className="btn" type="button">Export</button>}>
      <div className="panelbody repo-list">
        {PROMPT_PACK.map((item) => (
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

function Placeholder({ title, body }) {
  return (
    <div className="placeholder-shell">
      <div className="eyebrow">// RepoMemory v2 extraction queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

function PromptPackSurface() {
  return (
    <div className="placeholder-shell">
      <div>
        <div className="eyebrow">// RepoMemory v2 extraction queue</div>
        <h1>Prompt Packs</h1>
        <p className="subline">Reusable repo context bundles for TrustGate, RepoReaper, and future HiveCore handoffs.</p>
      </div>
      <div className="placeholder-panel">
        <PromptPackPanel />
      </div>
    </div>
  );
}

function SidePanels() {
  return (
    <aside className="side">
      <Panel eyebrow="FailGuard" title="Lesson pressure">
        <div className="panelbody repo-list">
          {FAILGUARD.map((item) => (
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

      <Panel eyebrow="Consumers" title="Context handoff">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">TrustGate</span><span className="chip green">armed</span></div>
          <div className="rowline"><span className="muted">RepoReaper</span><span className="chip signal">ready</span></div>
          <div className="rowline"><span className="muted">HiveCore</span><span className="chip amber">partial</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function MemoryCore() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - durable context</div>
              <h1>Memory Lattice</h1>
              <p className="subline">Merged history, review pain, and FailGuard lessons distilled into reusable repo context.</p>
            </div>
            <div className="actions">
              <span className="chip signal">context-first</span>
              <span className="chip">read only</span>
              <button className="btn primary" type="button">Run ingest</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout">
            <Panel eyebrow="Graph" title="Repo knowledge map" action={<span className="chip signal">react port</span>}>
              <MemoryLattice />
            </Panel>
            <MemoryQueuePanel />
          </div>
        </main>
        <SidePanels />
      </div>
    </>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("core");

  return (
    <>
      <DeckBar
        activeTab={activeTab}
        brandName="RepoMemory frontend v2"
        navLabel="RepoMemory v2 surfaces"
        onTabChange={setActiveTab}
        tabs={TABS}
      />
      {activeTab === "core" && <MemoryCore />}
      {activeTab === "failguard" && (
        <Placeholder
          title="FailGuard Review"
          body="This becomes the shared v2 bad-outcome review and promotion surface."
        />
      )}
      {activeTab === "packs" && <PromptPackSurface />}
    </>
  );
}

import { useMemo, useState } from "react";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductRail,
  SuiteTopline,
} from "@patchhivehq/ui-v2";

const TABS = [
  { id: "suite", label: "Suite board" },
  { id: "launch", label: "Launch stack" },
  { id: "defaults", label: "Defaults" },
  { id: "contracts", label: "Contracts" },
];

const TOPLINE_CELLS = [
  { label: "HiveCore", value: "Control plane 01", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Products", value: "11 / 12", tone: "warn" },
  { label: "Launcher", value: "Ready", tone: "sig" },
  { label: "Contract drift", value: "2 flags", tone: "warn" },
  { label: "Last poll", value: "T-00:42" },
];

const PRODUCTS = [
  {
    slug: "signal-hive",
    code: "SH",
    title: "SignalHive",
    accent: "#67bbe7",
    state: "online",
    stateTone: "green",
    position: { left: "50%", top: "10%" },
    health: "read-only scans healthy",
    url: "localhost:5192",
    run: "nightly-rust sweep",
    drift: "none",
    contract: "health, startup, runs",
    handoff: "TrustGate risk packet ready",
  },
  {
    slug: "trust-gate",
    code: "TG",
    title: "TrustGate",
    accent: "#c794ff",
    state: "online",
    stateTone: "green",
    position: { left: "72%", top: "18%" },
    health: "policy checks armed",
    url: "localhost:5193",
    run: "diff review idle",
    drift: "none",
    contract: "health, capabilities, run detail",
    handoff: "RepoMemory lesson sink armed",
  },
  {
    slug: "repo-memory",
    code: "RM",
    title: "RepoMemory",
    accent: "#65d98e",
    state: "online",
    stateTone: "green",
    position: { left: "86%", top: "38%" },
    health: "memory spine warm",
    url: "localhost:5194",
    run: "FailGuard queue review",
    drift: "none",
    contract: "health, startup, runs",
    handoff: "Prompt pack fresh",
  },
  {
    slug: "repo-reaper",
    code: "RR",
    title: "RepoReaper",
    accent: "#ff637c",
    state: "guarded",
    stateTone: "amber",
    position: { left: "82%", top: "66%" },
    health: "write actions dry gated",
    url: "localhost:5173",
    run: "candidate hunt paused",
    drift: "run detail missing",
    contract: "health, startup",
    handoff: "waiting on TrustGate",
  },
  {
    slug: "review-bee",
    code: "RB",
    title: "ReviewBee",
    accent: "#ffd36a",
    state: "online",
    stateTone: "green",
    position: { left: "62%", top: "86%" },
    health: "thread reads fresh",
    url: "localhost:5195",
    run: "review pressure map",
    drift: "none",
    contract: "health, runs, capabilities",
    handoff: "MergeKeeper checklist queued",
  },
  {
    slug: "merge-keeper",
    code: "MK",
    title: "MergeKeeper",
    accent: "#62e1d3",
    state: "online",
    stateTone: "green",
    position: { left: "38%", top: "86%" },
    health: "readiness checks clear",
    url: "localhost:5196",
    run: "PR readiness poll",
    drift: "none",
    contract: "health, run list",
    handoff: "ReleaseSentry release gate",
  },
  {
    slug: "flake-sting",
    code: "FS",
    title: "FlakeSting",
    accent: "#ff9b52",
    state: "setup",
    stateTone: "amber",
    position: { left: "18%", top: "66%" },
    health: "Actions token needed",
    url: "localhost:5197",
    run: "waiting for workflow history",
    drift: "capabilities missing",
    contract: "health only",
    handoff: "CI trust signal pending",
  },
  {
    slug: "dep-triage",
    code: "DT",
    title: "DepTriage",
    accent: "#c8db62",
    state: "online",
    stateTone: "green",
    position: { left: "14%", top: "38%" },
    health: "dependency queue stable",
    url: "localhost:5198",
    run: "Dependabot triage",
    drift: "none",
    contract: "health, runs",
    handoff: "ReleaseSentry blocker feed",
  },
  {
    slug: "vuln-triage",
    code: "VT",
    title: "VulnTriage",
    accent: "#ff7aa1",
    state: "watch",
    stateTone: "amber",
    position: { left: "28%", top: "18%" },
    health: "security reads partial",
    url: "localhost:5199",
    run: "alert reachability pass",
    drift: "run detail missing",
    contract: "health, startup",
    handoff: "TrustGate policy update",
  },
  {
    slug: "refactor-scout",
    code: "RS",
    title: "RefactorScout",
    accent: "#70dfbd",
    state: "online",
    stateTone: "green",
    position: { left: "32%", top: "50%" },
    health: "local scan ready",
    url: "localhost:5200",
    run: "hotspot scan idle",
    drift: "none",
    contract: "health, runs",
    handoff: "RepoMemory context candidate",
  },
  {
    slug: "release-sentry",
    code: "RSY",
    title: "ReleaseSentry",
    accent: "#fff08a",
    state: "online",
    stateTone: "green",
    position: { left: "68%", top: "50%" },
    health: "ship evidence ready",
    url: "localhost:5201",
    run: "release gate watch",
    drift: "none",
    contract: "health, capabilities, runs",
    handoff: "MergeKeeper evidence consumed",
  },
];

const METRICS = [
  { label: "Products online", value: "9", tone: "ok", sub: "2 watch, 1 setup" },
  { label: "Launch state", value: "Ready", tone: "sig", sub: "launcher linked" },
  { label: "Contract drift", value: "2", tone: "warn", sub: "run detail gaps" },
  { label: "Active runs", value: "4", tone: "sig", sub: "all read-only" },
  { label: "Token gaps", value: "1", tone: "warn", sub: "Actions history" },
];

const RAIL_SECTIONS = [
  {
    title: "Suite stack",
    items: [
      { label: "local-control", active: true, pin: true },
      { label: "discovery layer", value: "online" },
      { label: "trust layer", value: "armed" },
      { label: "action layer", value: "guarded" },
    ],
  },
  {
    title: "Profiles",
    items: [
      { label: "solo-local", active: true, badge: "on", badgeTone: "green" },
      { label: "suite-demo", badge: "v2", badgeTone: "signal" },
      { label: "autonomy-safe", badge: "dry", badgeTone: "amber" },
    ],
  },
];

const RAIL_STATS = {
  title: "Control plane",
  items: [
    { label: "Primary link", value: "launcher:8210" },
    { label: "Suite", value: "11/12", large: true, tone: "warn" },
    { label: "Mode", value: "operator gated" },
  ],
};

const ATTENTION = [
  { title: "FlakeSting needs Actions history token", meta: "CI trust signal is partial until workflow reads are available", label: "setup", tone: "amber" },
  { title: "RepoReaper remains guarded", meta: "write actions require TrustGate and dry-run confirmation", label: "guarded", tone: "red" },
  { title: "Two products lack run detail", meta: "HiveCore should expose this as contract drift, not hide it", label: "drift", tone: "amber" },
];

const LAUNCH_QUEUE = [
  { rank: "01", title: "Start missing local stack pieces", meta: "launcher can fill gaps without touching product code", tone: "green", label: "ready" },
  { rank: "02", title: "Open SignalHive reconnaissance", meta: "first handoff into TrustGate and RepoMemory", tone: "signal", label: "open" },
  { rank: "03", title: "Hold RepoReaper write actions", meta: "guarded until policy and memory are current", tone: "amber", label: "hold" },
  { rank: "04", title: "Refresh contract scan", meta: "poll health, startup, capabilities, runs, detail", tone: "", label: "poll" },
];

const CONTRACT_FLAGS = [
  { title: "Run detail", meta: "RepoReaper and VulnTriage need /runs/:id detail parity", label: "2 gaps", tone: "amber" },
  { title: "Capabilities", meta: "FlakeSting should publish supported read scopes", label: "1 gap", tone: "amber" },
  { title: "Startup checks", meta: "All visible v2 products report expected setup state", label: "clear", tone: "green" },
];

const DEFAULTS = [
  { title: "GitHub identity", meta: "PatchHive bot account, autonomous attribution required", label: "locked", tone: "green" },
  { title: "AI routing", meta: "PATCHHIVE_AI_URL before raw provider endpoints", label: "shared", tone: "signal" },
  { title: "Write action posture", meta: "dry-run and TrustGate review before PR creation", label: "guarded", tone: "amber" },
];

function toneClass(tone) {
  return tone ? ` ${tone}` : "";
}

function SuiteConstellation({ selectedProduct, onSelect }) {
  return (
    <Panel eyebrow="Control" title="Suite constellation" action={<span className="chip signal">12 nodes</span>}>
      <div className="constellation-shell">
        <div className="suite-orbit" aria-label="PatchHive suite control constellation">
          <span className="orbit-ring ring-a" />
          <span className="orbit-ring ring-b" />
          <span className="orbit-axis axis-a" />
          <span className="orbit-axis axis-b" />
          <button
            className="hive-node core active"
            onClick={() => onSelect(null)}
            style={{ "--node-color": "#7bd8e8" }}
            type="button"
          >
            HC
          </button>
          {PRODUCTS.map((product) => (
            <button
              className={`hive-node${selectedProduct?.slug === product.slug ? " active" : ""}`}
              data-state={product.state}
              key={product.slug}
              onClick={() => onSelect(product)}
              style={{ ...product.position, "--node-color": product.accent }}
              type="button"
            >
              <span>{product.code}</span>
            </button>
          ))}
        </div>
        <div className="constellation-readout">
          <div className="readout-card">
            <span className="label">Selected system</span>
            <span className="readout-value">{selectedProduct?.title || "HiveCore"}</span>
            <span className="micro">{selectedProduct?.health || "suite lifecycle and contract drift control"}</span>
          </div>
          <div className="readout-card">
            <span className="label">State</span>
            <span className={`readout-value${toneClass(selectedProduct?.stateTone || "sig")}`}>
              {selectedProduct?.state || "coordinating"}
            </span>
            <span className="micro">{selectedProduct?.url || "localhost control plane"}</span>
          </div>
          <div className="readout-card selected-scan">
            <span className="label">Control detail</span>
            <span className="readout-value">{selectedProduct?.run || "suite board poll"}</span>
            <div className="selected-grid">
              <div className="selected-stat">
                <span className="micro">Contract</span>
                <strong>{selectedProduct?.contract || "all products"}</strong>
              </div>
              <div className="selected-stat">
                <span className="micro">Drift</span>
                <strong>{selectedProduct?.drift || "2 flags"}</strong>
              </div>
              <div className="selected-stat">
                <span className="micro">Handoff</span>
                <strong>{selectedProduct?.handoff || "pipeline visible"}</strong>
              </div>
              <div className="selected-stat">
                <span className="micro">Owner</span>
                <strong>HiveCore</strong>
              </div>
            </div>
            <span className="micro">
              {selectedProduct
                ? `${selectedProduct.title} reports into HiveCore without losing its standalone product boundary.`
                : "HiveCore shows where the suite is healthy, where it drifts, and which products are ready for handoff."}
            </span>
          </div>
        </div>
      </div>
    </Panel>
  );
}

function LaunchQueuePanel() {
  return (
    <Panel eyebrow="Launch" title="Action queue" action={<span className="chip amber">operator gated</span>}>
      <div className="panelbody repo-list queue-grid">
        {LAUNCH_QUEUE.map((item) => (
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
      <Panel eyebrow="Attention" title="Needs action">
        <div className="panelbody repo-list">
          {ATTENTION.map((item) => (
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
      <Panel eyebrow="Contracts" title="Drift report">
        <div className="panelbody repo-list">
          {CONTRACT_FLAGS.map((item) => (
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
    </aside>
  );
}

function SuiteBoard() {
  const [selectedProduct, setSelectedProduct] = useState(PRODUCTS[0]);

  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - suite control</div>
              <h1>HiveCore Command</h1>
              <p className="subline">PatchHive product health, launcher state, contract drift, and cross-product handoffs in one control plane.</p>
            </div>
            <div className="actions">
              <span className="chip signal">launcher ready</span>
              <span className="chip amber">2 drift flags</span>
              <button className="btn primary" type="button">Poll suite</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <SuiteConstellation selectedProduct={selectedProduct} onSelect={setSelectedProduct} />
            <LaunchQueuePanel />
          </div>
        </main>
        <SidePanels />
      </div>
    </>
  );
}

function LaunchStack() {
  const services = useMemo(
    () => [
      { rank: "8210", title: "patchhive-launcher", meta: "host-control daemon linked to HiveCore", tone: "green", label: "ready" },
      { rank: "8000", title: "HiveCore backend", meta: "health, startup checks, product registry", tone: "green", label: "online" },
      { rank: "5199", title: "HiveCore frontend v2", meta: "control plane prototype surface", tone: "signal", label: "view" },
      { rank: "dry", title: "write-action interlock", meta: "RepoReaper stays guarded until trust handoff clears", tone: "amber", label: "armed" },
    ],
    [],
  );

  return (
    <div className="placeholder-shell">
      <div>
        <div className="eyebrow">// HiveCore v2 launch stack</div>
        <h1>Launch Stack</h1>
        <p className="subline">Local stack pieces, launcher authority, and guarded action posture.</p>
      </div>
      <Panel eyebrow="Stack" title="Local services">
        <div className="panelbody repo-list queue-grid">
          {services.map((item) => (
            <div className="ledger-row" key={item.title}>
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
    </div>
  );
}

function DefaultsSurface() {
  return (
    <div className="placeholder-shell">
      <div>
        <div className="eyebrow">// HiveCore v2 shared defaults</div>
        <h1>Defaults</h1>
        <p className="subline">Suite-wide settings that should eventually propagate into every specialist product.</p>
      </div>
      <Panel eyebrow="Policy" title="Shared defaults">
        <div className="panelbody repo-list">
          {DEFAULTS.map((item) => (
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

function ContractsSurface() {
  return (
    <div className="placeholder-shell">
      <div>
        <div className="eyebrow">// HiveCore v2 contract monitor</div>
        <h1>Contracts</h1>
        <p className="subline">Health, startup, capabilities, run lists, and run detail parity across the suite.</p>
      </div>
      <Panel eyebrow="Drift" title="Contract flags">
        <div className="panelbody repo-list">
          {CONTRACT_FLAGS.map((item) => (
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

export default function App() {
  const [activeTab, setActiveTab] = useState("suite");

  return (
    <>
      <DeckBar
        activeTab={activeTab}
        brandName="HiveCore frontend v2"
        navLabel="HiveCore v2 surfaces"
        onTabChange={setActiveTab}
        productKey="hive-core"
        tabs={TABS}
      />
      {activeTab === "suite" && <SuiteBoard />}
      {activeTab === "launch" && <LaunchStack />}
      {activeTab === "defaults" && <DefaultsSurface />}
      {activeTab === "contracts" && <ContractsSurface />}
    </>
  );
}

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
  { label: "Radar blips", value: "10 / 11", tone: "warn" },
  { label: "Launcher", value: "Ready", tone: "sig" },
  { label: "Issues", value: "2 yellow", tone: "warn" },
  { label: "Last poll", value: "T-00:42" },
];

const PRODUCTS = [
  {
    slug: "signal-hive",
    code: "SH",
    title: "SignalHive",
    accent: "#67bbe7",
    status: "good",
    state: "good",
    stateTone: "green",
    position: { left: "50%", top: "16%" },
    health: "read-only scans healthy",
    url: "localhost:5192",
    run: "nightly-rust sweep",
    drift: "none",
    contract: "health, startup, runs",
    handoff: "TrustGate risk packet ready",
    actions: [
      { label: "Run sweep", meta: "Start a fresh reconnaissance pass", tone: "signal" },
      { label: "Edit schedule", meta: "Adjust scan cadence and scope", tone: "" },
      { label: "Send to TrustGate", meta: "Queue selected findings for risk review", tone: "amber" },
    ],
  },
  {
    slug: "trust-gate",
    code: "TG",
    title: "TrustGate",
    accent: "#c794ff",
    status: "good",
    state: "good",
    stateTone: "green",
    position: { left: "70%", top: "24%" },
    health: "policy checks armed",
    url: "localhost:5193",
    run: "diff review idle",
    drift: "none",
    contract: "health, capabilities, run detail",
    handoff: "RepoMemory lesson sink armed",
    actions: [
      { label: "Review diff", meta: "Run policy gate on current patch", tone: "signal" },
      { label: "Sync policies", meta: "Pull latest RepoMemory rules", tone: "" },
      { label: "Publish check", meta: "Prepare GitHub status output", tone: "green" },
    ],
  },
  {
    slug: "repo-memory",
    code: "RM",
    title: "RepoMemory",
    accent: "#65d98e",
    status: "good",
    state: "good",
    stateTone: "green",
    position: { left: "78%", top: "42%" },
    health: "memory spine warm",
    url: "localhost:5194",
    run: "FailGuard queue review",
    drift: "none",
    contract: "health, startup, runs",
    handoff: "Prompt pack fresh",
    actions: [
      { label: "Promote lessons", meta: "Review queued FailGuard memories", tone: "amber" },
      { label: "Export pack", meta: "Generate fresh repo context", tone: "signal" },
      { label: "Sync TrustGate", meta: "Push pinned policy memories", tone: "green" },
    ],
  },
  {
    slug: "repo-reaper",
    code: "RR",
    title: "RepoReaper",
    accent: "#ff637c",
    status: "issues",
    state: "issues",
    stateTone: "amber",
    position: { left: "76%", top: "65%" },
    health: "write actions dry gated",
    url: "localhost:5173",
    run: "candidate hunt paused",
    drift: "run detail missing",
    contract: "health, startup",
    handoff: "waiting on TrustGate",
    actions: [
      { label: "Dry run hunt", meta: "Find candidates without opening PRs", tone: "signal" },
      { label: "Hold writes", meta: "Keep outbound PR actions locked", tone: "amber" },
      { label: "Open reaper view", meta: "Inspect patch pipeline details", tone: "" },
    ],
  },
  {
    slug: "review-bee",
    code: "RB",
    title: "ReviewBee",
    accent: "#ffd36a",
    status: "good",
    state: "good",
    stateTone: "green",
    position: { left: "60%", top: "80%" },
    health: "thread reads fresh",
    url: "localhost:5195",
    run: "review pressure map",
    drift: "none",
    contract: "health, runs, capabilities",
    handoff: "MergeKeeper checklist queued",
    actions: [
      { label: "Refresh PR", meta: "Pull latest review thread state", tone: "signal" },
      { label: "Build checklist", meta: "Regenerate action groups", tone: "green" },
      { label: "Send MergeKeeper", meta: "Pass unresolved pressure forward", tone: "amber" },
    ],
  },
  {
    slug: "merge-keeper",
    code: "MK",
    title: "MergeKeeper",
    accent: "#62e1d3",
    status: "good",
    state: "good",
    stateTone: "green",
    recentlyLive: true,
    position: { left: "42%", top: "80%" },
    health: "readiness checks clear",
    url: "localhost:5196",
    run: "PR readiness poll",
    drift: "none",
    contract: "health, run list",
    handoff: "ReleaseSentry release gate",
    actions: [
      { label: "Check readiness", meta: "Poll review, checks, and blockers", tone: "signal" },
      { label: "Hold merge", meta: "Mark PR as not ready", tone: "amber" },
      { label: "Send release gate", meta: "Forward clear PR evidence", tone: "green" },
    ],
  },
  {
    slug: "flake-sting",
    code: "FS",
    title: "FlakeSting",
    accent: "#ff9b52",
    status: "not started",
    state: "not started",
    stateTone: "",
    started: false,
    position: { left: "20%", top: "64%" },
    health: "Actions token needed",
    url: "localhost:5197",
    run: "waiting for workflow history",
    drift: "capabilities missing",
    contract: "health only",
    handoff: "CI trust signal pending",
    actions: [
      { label: "Start product", meta: "Launch service and add radar blip", tone: "signal" },
      { label: "Add token", meta: "Enable GitHub Actions reads", tone: "amber" },
      { label: "Open setup", meta: "Configure flaky-test evidence", tone: "" },
    ],
  },
  {
    slug: "dep-triage",
    code: "DT",
    title: "DepTriage",
    accent: "#c8db62",
    status: "good",
    state: "good",
    stateTone: "green",
    position: { left: "22%", top: "42%" },
    health: "dependency queue stable",
    url: "localhost:5198",
    run: "Dependabot triage",
    drift: "none",
    contract: "health, runs",
    handoff: "ReleaseSentry blocker feed",
    actions: [
      { label: "Run triage", meta: "Rank dependency update pressure", tone: "signal" },
      { label: "Open queue", meta: "Inspect update decisions", tone: "" },
      { label: "Send blockers", meta: "Feed release readiness", tone: "amber" },
    ],
  },
  {
    slug: "vuln-triage",
    code: "VT",
    title: "VulnTriage",
    accent: "#ff7aa1",
    status: "issues",
    state: "issues",
    stateTone: "amber",
    position: { left: "30%", top: "24%" },
    health: "security reads partial",
    url: "localhost:5199",
    run: "alert reachability pass",
    drift: "run detail missing",
    contract: "health, startup",
    handoff: "TrustGate policy update",
    actions: [
      { label: "Refresh alerts", meta: "Read security findings again", tone: "signal" },
      { label: "Fix contract", meta: "Add run detail support", tone: "amber" },
      { label: "Send policies", meta: "Queue TrustGate rule updates", tone: "green" },
    ],
  },
  {
    slug: "refactor-scout",
    code: "RS",
    title: "RefactorScout",
    accent: "#70dfbd",
    status: "good",
    state: "good",
    stateTone: "green",
    position: { left: "34%", top: "54%" },
    health: "local scan ready",
    url: "localhost:5200",
    run: "hotspot scan idle",
    drift: "none",
    contract: "health, runs",
    handoff: "RepoMemory context candidate",
    actions: [
      { label: "Scan repo", meta: "Find conservative refactor leads", tone: "signal" },
      { label: "Export leads", meta: "Save opportunity report", tone: "" },
      { label: "Send memory", meta: "Store recurring hotspot context", tone: "green" },
    ],
  },
  {
    slug: "release-sentry",
    code: "RSY",
    title: "ReleaseSentry",
    accent: "#fff08a",
    status: "down",
    state: "down",
    stateTone: "red",
    position: { left: "66%", top: "54%" },
    health: "health endpoint unreachable",
    url: "localhost:5201",
    run: "release gate offline",
    drift: "service down",
    contract: "health, capabilities, runs",
    handoff: "MergeKeeper evidence consumed",
    actions: [
      { label: "Restart service", meta: "Ask launcher to recover health", tone: "red" },
      { label: "View logs", meta: "Inspect failed release gate", tone: "amber" },
      { label: "Hold release", meta: "Keep ship decision blocked", tone: "red" },
    ],
  },
];

const METRICS = [
  { label: "Green blips", value: "7", tone: "ok", sub: "healthy products" },
  { label: "Yellow blips", value: "2", tone: "warn", sub: "needs attention" },
  { label: "Red blips", value: "1", tone: "hot", sub: "down product" },
  { label: "Not started", value: "1", tone: "sig", sub: "hidden from radar" },
  { label: "Launcher", value: "Ready", tone: "sig", sub: "can fill gaps" },
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
    { label: "Radar", value: "10/11", large: true, tone: "warn" },
    { label: "Mode", value: "operator gated" },
  ],
};

const ATTENTION = [
  { title: "ReleaseSentry is down", meta: "Red radar blip means health endpoint is currently unreachable", label: "down", tone: "red" },
  { title: "FlakeSting has not started", meta: "Not-started products stay off the radar until launcher brings them online", label: "hidden", tone: "signal" },
  { title: "RepoReaper remains guarded", meta: "write actions require TrustGate and dry-run confirmation", label: "guarded", tone: "red" },
  { title: "VulnTriage needs run detail", meta: "Yellow radar blip means the product is reachable but has contract drift", label: "issue", tone: "amber" },
];

const LAUNCH_QUEUE = [
  { rank: "01", title: "Start FlakeSting", meta: "not-started products do not appear on the radar until launched", tone: "signal", label: "start" },
  { rank: "02", title: "Open SignalHive reconnaissance", meta: "first handoff into TrustGate and RepoMemory", tone: "signal", label: "open" },
  { rank: "03", title: "Restart ReleaseSentry", meta: "red radar blip should clear after health endpoint responds", tone: "red", label: "down" },
  { rank: "04", title: "Hold RepoReaper write actions", meta: "yellow until policy and memory handoff clears", tone: "amber", label: "hold" },
];

const CONTRACT_FLAGS = [
  { title: "Run detail", meta: "RepoReaper and VulnTriage need /runs/:id detail parity", label: "2 gaps", tone: "amber" },
  { title: "Service health", meta: "ReleaseSentry is visible as red until health recovers", label: "down", tone: "red" },
  { title: "Startup checks", meta: "All visible v2 products report expected setup state", label: "clear", tone: "green" },
];

const DEFAULTS = [
  { title: "GitHub identity", meta: "PatchHive bot account, autonomous attribution required", label: "locked", tone: "green" },
  { title: "AI routing", meta: "PATCHHIVE_AI_URL before raw provider endpoints", label: "shared", tone: "signal" },
  { title: "Write action posture", meta: "dry-run and TrustGate review before PR creation", label: "guarded", tone: "amber" },
];

const SUITE_ACTIONS = [
  { label: "Poll suite", meta: "Refresh all product health and contract state", tone: "signal" },
  { label: "Start missing", meta: "Launch products that are not on radar", tone: "green" },
  { label: "Lock writes", meta: "Keep action products in guarded mode", tone: "amber" },
];

function toneClass(tone) {
  return tone ? ` ${tone}` : "";
}

function SuiteProductRadar({ selectedProduct, onSelect }) {
  const visibleProducts = PRODUCTS.filter((product) => product.started !== false);
  const activeProduct = selectedProduct?.started === false ? visibleProducts[0] : selectedProduct;

  return (
    <Panel eyebrow="Control" title="Product status radar" action={<span className="chip signal">10 blips</span>}>
      <div className="constellation-shell">
        <div className="suite-orbit hive-radar-frame">
          <div className="status-key" aria-label="Radar status legend">
            <span><i className="green" /> good</span>
            <span><i className="amber" /> issues</span>
            <span><i className="red" /> down</span>
            <span><i /> not started: hidden</span>
          </div>
          <div className="radar-screen hive-radar-disc" aria-label="PatchHive product status radar">
            <span className="radar-bearing n">000</span>
            <span className="radar-bearing e">090</span>
            <span className="radar-bearing s">180</span>
            <span className="radar-bearing w">270</span>
            <span className="radar-density" />
            <span className="radar-sweep" />
            <span className="radar-line" />
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
            {visibleProducts.map((product) => (
              <button
                className={`hive-node hive-product-blip ${product.stateTone}${product.recentlyLive ? " joining" : ""}${activeProduct?.slug === product.slug ? " active" : ""}`}
                data-code={product.code}
                data-state={product.state}
                key={product.slug}
                onClick={() => onSelect(product)}
                style={{ ...product.position }}
                type="button"
              >
                <span>{product.title}</span>
              </button>
            ))}
          </div>
        </div>
        <div className="constellation-readout">
          <div className="readout-card">
            <span className="label">Selected system</span>
            <span className="readout-value">{activeProduct?.title || "HiveCore"}</span>
            <span className="micro">{activeProduct?.health || "suite lifecycle and contract drift control"}</span>
          </div>
          <div className="readout-card">
            <span className="label">State</span>
            <span className={`readout-value${toneClass(activeProduct?.stateTone || "sig")}`}>
              {activeProduct?.state || "coordinating"}
            </span>
            <span className="micro">{activeProduct?.url || "localhost control plane"}</span>
          </div>
          <div className="readout-card selected-scan">
            <div className="readout-headline">
              <div>
                <span className="label">Product detail</span>
                <span className="readout-value">{activeProduct?.run || "suite board poll"}</span>
              </div>
              {activeProduct && (
                <a className="btn" href={`http://${activeProduct.url}`} rel="noreferrer" target="_blank">
                  Open UI
                </a>
              )}
            </div>
            <div className="selected-grid">
              <div className="selected-stat">
                <span className="micro">Contract</span>
                <strong>{activeProduct?.contract || "all products"}</strong>
              </div>
              <div className="selected-stat">
                <span className="micro">Drift</span>
                <strong>{activeProduct?.drift || "2 flags"}</strong>
              </div>
              <div className="selected-stat">
                <span className="micro">Handoff</span>
                <strong>{activeProduct?.handoff || "pipeline visible"}</strong>
              </div>
              <div className="selected-stat">
                <span className="micro">Owner</span>
                <strong>HiveCore</strong>
              </div>
            </div>
            <span className="micro">
              {activeProduct
                ? `${activeProduct.title} controls stay in HiveCore while the specialist dashboard stays one click away.`
                : "HiveCore is the cockpit: radar status, launch controls, config changes, and guarded actions stay in one place."}
            </span>
          </div>
        </div>
      </div>
    </Panel>
  );
}

function CockpitControlsPanel({ selectedProduct }) {
  const activeProduct = selectedProduct?.started === false ? null : selectedProduct;
  const actions = activeProduct?.actions || SUITE_ACTIONS;

  return (
    <Panel
      eyebrow={activeProduct ? "Controls" : "Suite"}
      title={activeProduct ? `${activeProduct.title} actions` : "Cockpit actions"}
      action={<span className={`chip ${activeProduct?.stateTone || "signal"}`}>{activeProduct?.state || "suite"}</span>}
    >
      <div className="panelbody cockpit-control-grid">
        {actions.map((action) => (
          <button className={`cockpit-action ${action.tone || ""}`} key={action.label} type="button">
            <span>{action.label}</span>
            <small>{action.meta}</small>
          </button>
        ))}
        {activeProduct && (
          <a className="cockpit-action" href={`http://${activeProduct.url}`} rel="noreferrer" target="_blank">
            <span>Open dashboard</span>
            <small>Jump to the specialist view</small>
          </a>
        )}
      </div>
      <div className="panelbody repo-list cockpit-queue">
        {LAUNCH_QUEUE.slice(0, 3).map((item) => (
          <div className="feed-item" key={item.rank}>
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
  const [selectedProduct, setSelectedProduct] = useState(() => PRODUCTS.find((product) => product.started !== false));

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
              <p className="subline">PatchHive product health, launcher state, contract drift, and product controls in one cockpit.</p>
            </div>
            <div className="actions">
              <span className="chip signal">launcher ready</span>
              <span className="chip amber">2 drift flags</span>
              <button className="btn primary" type="button">Poll suite</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <SuiteProductRadar selectedProduct={selectedProduct} onSelect={setSelectedProduct} />
            <CockpitControlsPanel selectedProduct={selectedProduct} />
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

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductV2Shell,
  ProductRail,
  SuiteTopline,
  applySuiteAccent,
  usePersistentProductTab,
} from "@patchhivehq/ui-v2";
import { API } from "./config.js";

const TABS = [
  { id: "suite", label: "Suite board" },
  { id: "launch", label: "Launch stack" },
  { id: "defaults", label: "Defaults" },
  { id: "contracts", label: "Contracts" },
];

const TOPLINE_CELLS = [
  { label: "HiveCore", value: "Control plane", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Radar blips", value: "10 / 11", tone: "warn" },
  { label: "Launcher", value: "Ready", tone: "sig" },
  { label: "Issues", value: "2 yellow", tone: "warn" },
  { label: "Last poll", value: "T-00:42" },
];

const PRODUCT_PRESENTATION = [
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

const PRODUCT_PRESENTATION_BY_SLUG = Object.fromEntries(PRODUCT_PRESENTATION.map((product) => [product.slug, product]));

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
      { label: "suite-local", badge: "on", badgeTone: "signal" },
      { label: "autonomy-safe", badge: "dry", badgeTone: "amber" },
    ],
  },
];

const RAIL_STATS = {
  title: "Unified backend",
  items: [
    { label: "Primary link", value: "patchhive-backend" },
    { label: "Radar", value: "registry", large: true, tone: "signal" },
    { label: "Mode", value: "suite runtime" },
  ],
};

const ATTENTION = [
  { title: "Product engines are pending", meta: "The unified backend owns the registry now; engines move in one product at a time.", label: "pending", tone: "amber" },
  { title: "HiveCore is first client", meta: "This UI now reads the unified backend control-plane contract directly.", label: "live", tone: "signal" },
  { title: "Gateway routes are next", meta: "Existing product APIs can be proxied before engines are fully migrated.", label: "bridge", tone: "amber" },
  { title: "RepoReaper moves late", meta: "Write-capable execution waits for auth, approvals, and safety contracts.", label: "guarded", tone: "red" },
];

const LAUNCH_QUEUE = [
  { rank: "01", title: "Wire HiveCore", meta: "make the control-plane UI read patchhive-backend first", tone: "signal", label: "now" },
  { rank: "02", title: "Add gateway routes", meta: "proxy existing product APIs through the unified backend", tone: "amber", label: "next" },
  { rank: "03", title: "Move SignalHive engine", meta: "first read-only product engine to run in-process", tone: "signal", label: "first" },
  { rank: "04", title: "Add auth and storage", meta: "operator auth, run index, event log, and shared config", tone: "amber", label: "core" },
];

const CONTRACT_FLAGS = [
  { title: "Unified registry", meta: "HiveCore reads product names, roles, status, and route prefixes from patchhive-backend.", label: "live", tone: "green" },
  { title: "Product engines", meta: "Enabled products report pending until their real engines are migrated or proxied.", label: "pending", tone: "amber" },
  { title: "Action dispatch", meta: "Mutating product actions stay disabled until backend approval contracts exist.", label: "guarded", tone: "red" },
];

const DEFAULTS = [
  { title: "GitHub identity", meta: "PatchHive bot account, autonomous attribution required", label: "locked", tone: "green" },
  { title: "AI routing", meta: "PATCHHIVE_AI_URL before raw provider endpoints", label: "shared", tone: "signal" },
  { title: "Write action posture", meta: "dry-run and TrustGate review before PR creation", label: "guarded", tone: "amber" },
];

const SUITE_ACTIONS = [
  { label: "Poll suite", meta: "Refresh all product health and contract state", tone: "signal" },
  { label: "Sync registry", meta: "Reload enabled products from patchhive-backend", tone: "green" },
  { label: "Lock writes", meta: "Keep action products in guarded mode", tone: "amber" },
];

function toneClass(tone) {
  return tone ? ` ${tone}` : "";
}

async function fetchJson(fetch_, url, opts = {}) {
  const res = await fetch_(url, opts);
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw new Error(data?.error?.message || data?.error || data?.message || `Request failed: ${res.status}`);
  }
  return data;
}

async function fetchEnvelope(fetch_, path, opts = {}) {
  const payload = await fetchJson(fetch_, `${API}${path}`, opts);
  return payload?.data ?? payload;
}

function productTone(status = "") {
  if (status === "online" || status === "integrated") return "green";
  if (status === "degraded" || status === "engine-pending" || status === "gateway-pending") return "amber";
  if (status === "disabled" || status === "unconfigured" || status === "not-started") return "signal";
  if (status === "offline" || status === "down") return "red";
  return "";
}

function statusTone(status = "") {
  if (status === "ok" || status === "online") return "ok";
  if (status === "degraded") return "warn";
  if (status === "down" || status === "offline") return "hot";
  return "sig";
}

function liveProductsFrom(setup, products) {
  if (setup?.products?.length) {
    return setup.products.map((item) => normalizeRuntimeProduct(item.runtime || item));
  }
  return Array.isArray(products) ? products.map(normalizeRuntimeProduct) : [];
}

function normalizeRuntimeProduct(product = {}) {
  const slug = product.slug || product.key;
  const presentation = PRODUCT_PRESENTATION_BY_SLUG[slug] || {};
  const status = product.status || (product.enabled === false ? "disabled" : "engine-pending");
  return {
    ...presentation,
    ...product,
    slug,
    code: product.icon || presentation.code || slug?.slice(0, 2).toUpperCase() || "--",
    title: product.title || product.name || presentation.title || slug || "Unknown product",
    role: product.role || presentation.role || "",
    status,
    enabled: product.enabled !== false,
    api_url: product.api_url || product.route_prefix || `/api/products/${slug}`,
    service_token_configured: Boolean(product.service_token_configured),
    contract_drift_count: Number(product.contract_drift_count || 0),
  };
}

function radarStateForStatus(status = "") {
  if (status === "online" || status === "integrated") {
    return { state: "good", stateTone: "green" };
  }
  if (status === "offline" || status === "down") {
    return { state: "down", stateTone: "red" };
  }
  if (status === "disabled" || status === "unconfigured" || status === "not-started") {
    return { state: "not started", stateTone: "", started: false };
  }
  return { state: "issues", stateTone: "amber" };
}

function buildRadarProducts(setup, products) {
  return liveProductsFrom(setup, products)
    .filter((product) => product.slug && product.slug !== "hive-core")
    .map((product) => {
      const presentation = PRODUCT_PRESENTATION_BY_SLUG[product.slug] || {};
      const radarState = radarStateForStatus(product.status);
      const driftCount = Number(product.contract_drift_count || 0);
      return {
        ...presentation,
        ...product,
        ...radarState,
        code: product.code || presentation.code,
        position: presentation.position || { left: "50%", top: "50%" },
        health: product.role || presentation.health || "unified backend product",
        run: product.status === "online" ? "control route online" : "engine migration pending",
        drift: driftCount ? `${driftCount} flags` : "none",
        contract: product.api_url || `/api/products/${product.slug}`,
        handoff: product.enabled ? "unified backend route registered" : "disabled by PATCHHIVE_PRODUCTS",
        url: presentation.url,
        actions: presentation.actions || [
          { label: "Inspect route", meta: "Review unified backend product metadata", tone: "signal" },
          { label: "Hold actions", meta: "Wait for backend approval contracts", tone: "amber" },
        ],
      };
    });
}

function buildLiveTopline(health, setup, products) {
  const liveProducts = liveProductsFrom(setup, products);
  const downstream = liveProducts.filter((product) => product.slug !== "hive-core" && product.enabled !== false);
  const active = liveProducts.filter((product) => product.status !== "disabled" && product.enabled !== false).length;
  const paired = downstream.filter((product) => product.service_token_configured).length;
  const drift = liveProducts.reduce((total, product) => total + Number(product.contract_drift_count || 0), 0);
  return [
    { label: "HiveCore", value: health?.status || "unknown", tone: statusTone(health?.status) },
    { label: "Backend", value: "unified", tone: "sig" },
    { label: "Mode", value: health?.mode || "suite runtime", tone: "sig" },
    { label: "Enabled", value: `${active} / ${liveProducts.length || 12}`, tone: active ? "ok" : "warn" },
    { label: "Tokens", value: `${paired} / ${downstream.length || 11}`, tone: paired === downstream.length && downstream.length ? "ok" : "warn" },
    { label: "Drift", value: String(drift), tone: drift ? "warn" : "ok" },
  ];
}

function buildLiveMetrics(health, setup, products) {
  const liveProducts = liveProductsFrom(setup, products);
  const online = liveProducts.filter((product) => product.status === "online").length;
  const pending = liveProducts.filter((product) => product.status === "degraded" || product.status === "engine-pending" || product.status === "gateway-pending").length;
  const offline = liveProducts.filter((product) => product.status === "offline").length;
  const disabled = liveProducts.filter((product) => product.status === "disabled" || product.enabled === false).length;
  const drift = liveProducts.reduce((total, product) => total + Number(product.contract_drift_count || 0), 0);
  return [
    { label: "Online", value: String(online), tone: "ok", sub: "unified routes ready" },
    { label: "Pending", value: String(pending), tone: "warn", sub: "engines or gateways pending" },
    { label: "Offline", value: String(offline), tone: offline ? "hot" : "ok", sub: "down products" },
    { label: "Disabled", value: String(disabled), tone: "sig", sub: "hidden from radar" },
    { label: "Drift", value: String(drift), tone: drift ? "warn" : "ok", sub: health?.mode || "suite runtime" },
  ];
}

function StatusBanner({ tone = "signal", children }) {
  if (!children) return null;
  return <div className={`status-banner ${tone}`}>{children}</div>;
}

function SuiteProductRadar({ products, selectedProduct, onSelect }) {
  const visibleProducts = products.filter((product) => product.started !== false);
  const activeProduct = selectedProduct?.started === false ? visibleProducts[0] : selectedProduct;

  return (
    <Panel eyebrow="Control" title="Product status radar" action={<span className="chip signal">{visibleProducts.length} blips</span>}>
      <div className="constellation-shell">
        <div className="status-key" aria-label="Radar status legend">
          <span><i className="green" /> good</span>
          <span><i className="amber" /> issues</span>
          <span><i className="red" /> down</span>
          <span><i /> not started: hidden</span>
        </div>
        <div className="suite-orbit hive-radar-frame">
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
              className={`hive-node core${activeProduct ? "" : " active"}`}
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
              {activeProduct?.url && (
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

function CockpitControlsPanel({ onPairFirstStack, onRefresh, running, selectedProduct }) {
  const activeProduct = selectedProduct?.started === false ? null : selectedProduct;
  const actions = activeProduct?.actions || SUITE_ACTIONS;

  return (
    <Panel
      eyebrow={activeProduct ? "Controls" : "Suite"}
      title={activeProduct ? `${activeProduct.title} actions` : "Cockpit actions"}
      action={<span className={`chip ${activeProduct?.stateTone || "signal"}`}>{activeProduct?.state || "suite"}</span>}
    >
      <div className="panelbody cockpit-control-grid">
        {actions.map((action) => {
          const actionHandler = !activeProduct && action.label === "Poll suite"
            ? onRefresh
            : !activeProduct && action.label === "Sync registry"
              ? onPairFirstStack
              : undefined;
          return (
          <button
            className={`cockpit-action ${action.tone || ""}`}
            disabled={running}
            key={action.label}
            onClick={actionHandler}
            type="button"
          >
            <span>{action.label}</span>
            <small>{action.meta}</small>
          </button>
          );
        })}
        {activeProduct?.url && (
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

function SuiteBoard({
  actionMessage,
  error,
  health,
  onPairFirstStack,
  onRefresh,
  products,
  running,
  setup,
}) {
  const radarProducts = useMemo(() => buildRadarProducts(setup, products), [setup, products]);
  const [selectedSlug, setSelectedSlug] = useState(null);
  const selectedProduct = useMemo(
    () => selectedSlug ? radarProducts.find((product) => product.slug === selectedSlug) || null : null,
    [radarProducts, selectedSlug],
  );
  const selectProduct = useCallback((product) => setSelectedSlug(product?.slug || null), []);

  return (
    <>
      <SuiteTopline cells={setup || health ? buildLiveTopline(health, setup, products) : TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - suite control</div>
              <h1>HiveCore Command</h1>
              <p className="subline">PatchHive product registry, runtime status, contract drift, and guarded controls from the unified backend.</p>
            </div>
            <div className="actions">
              <span className="chip signal">unified backend</span>
              <span className="chip amber">{liveProductsFrom(setup, products).reduce((total, product) => total + Number(product.contract_drift_count || 0), 0)} drift flags</span>
              <button className="btn primary" disabled={running} onClick={onRefresh} type="button">
                {running ? "Polling" : "Poll suite"}
              </button>
            </div>
          </div>
          {error && <StatusBanner tone="red">{error}</StatusBanner>}
          {actionMessage && <StatusBanner tone={actionMessage.tone}>{actionMessage.text}</StatusBanner>}
          <MetricBand metrics={setup || health ? buildLiveMetrics(health, setup, products) : METRICS} />
          <div className="atlas-layout suite-four-layout">
            <SuiteProductRadar products={radarProducts} selectedProduct={selectedProduct} onSelect={selectProduct} />
            <CockpitControlsPanel
              onPairFirstStack={onPairFirstStack}
              onRefresh={onRefresh}
              running={running}
              selectedProduct={selectedProduct}
            />
          </div>
        </main>
        <SidePanels />
      </div>
    </>
  );
}

function HiveTabFrame({ children, health, products, setup }) {
  return (
    <>
      <SuiteTopline cells={setup || health ? buildLiveTopline(health, setup, products) : TOPLINE_CELLS} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace hive-tab-workspace">
          {children}
        </main>
      </div>
    </>
  );
}

function LaunchStack({
  actionMessage,
  error,
  health,
  onPairFirstStack,
  onRefresh,
  products,
  running,
  setup,
}) {
  const setupProducts = setup?.products || [];
  const runtimeProducts = setupProducts.length
    ? setupProducts.map((item) => item.runtime)
    : products;
  const downstreamCount = runtimeProducts.filter((product) => product.slug !== "hive-core").length;
  const enabledCount = runtimeProducts.filter((product) => product.slug !== "hive-core" && product.enabled !== false).length;

  return (
    <HiveTabFrame health={health} products={products} setup={setup}>
      <div>
        <div className="eyebrow">// HiveCore launch stack</div>
        <h1>Launch Stack</h1>
        <p className="subline">Local stack pieces, launcher authority, and guarded action posture.</p>
      </div>
      <div className="actions">
        <button className="btn" disabled={running} onClick={onRefresh} type="button">
          Refresh
        </button>
        <button className="btn primary" disabled={running} onClick={onPairFirstStack} type="button">
          {running ? "Syncing" : "Sync registry"}
        </button>
      </div>
      {error && <StatusBanner tone="red">{error}</StatusBanner>}
      {actionMessage && <StatusBanner tone={actionMessage.tone}>{actionMessage.text}</StatusBanner>}
      <Panel eyebrow="Stack" title="Local services">
        <div className="panelbody report-grid hive-stack-grid">
          <div><span className="label">HiveCore</span><strong>{health?.status || "unknown"}</strong></div>
          <div><span className="label">DB</span><strong>{health?.db_ok ? "ok" : "check"}</strong></div>
          <div><span className="label">Overrides</span><strong>{health?.product_override_count ?? "none"}</strong></div>
          <div><span className="label">Backend</span><strong>{health?.mode || "unified"}</strong></div>
          <div><span className="label">Launcher</span><strong>{setup?.launcher?.available ? "bridged" : "pending"}</strong></div>
          <div><span className="label">Enabled</span><strong>{enabledCount} / {downstreamCount || 11}</strong></div>
        </div>
      </Panel>
      <Panel eyebrow="Registry" title="Unified product registry">
        <div className="panelbody hive-launch-grid">
          {setupProducts.length === 0 && (
            <div className="empty-v2">
              <span className="micro">// Empty</span>
              <strong>No setup data loaded</strong>
              <span>Refresh HiveCore to load the unified backend product registry.</span>
            </div>
          )}
          {setupProducts.map((item) => {
            const runtime = item.runtime;
            const canInspect = runtime.slug !== "hive-core" && runtime.enabled !== false;
            return (
              <article className={`launch-card ${productTone(runtime.status)}`} key={runtime.slug}>
                <div className="launch-card-head">
                  <div className="rank launch-rank">
                    {runtime.slug === "hive-core" ? "HC" : runtime.icon || runtime.slug.slice(0, 2).toUpperCase()}
                  </div>
                  <span className={`chip ${productTone(runtime.status)}`}>{runtime.status}</span>
                </div>
                <div className="launch-card-body">
                  <div className="launch-title">{runtime.title}</div>
                  <div className="launch-url">{runtime.api_url}</div>
                  <div className="launch-token-state">
                    {runtime.enabled === false ? "disabled by PATCHHIVE_PRODUCTS" : "registered in patchhive-backend"}
                  </div>
                  {runtime.status !== "online" && runtime.enabled !== false && <div className="launch-warning">product engine or gateway route is still pending</div>}
                  {item.auth_status_error && <div className="launch-warning">{item.auth_status_error}</div>}
                </div>
                <div className="launch-card-footer">
                  <span className={`chip ${runtime.enabled === false ? "signal" : productTone(runtime.status)}`}>
                    {runtime.enabled === false ? "disabled" : runtime.status}
                  </span>
                  <button
                    className="btn"
                    disabled={running || !canInspect}
                    onClick={onRefresh}
                    type="button"
                  >
                    Inspect
                  </button>
                </div>
              </article>
            );
          })}
        </div>
      </Panel>
      {setup?.actions?.length > 0 && (
        <Panel eyebrow="Result" title="Latest pairing actions">
          <div className="panelbody repo-list">
            {setup.actions.map((action) => (
              <div className="feed-item" key={action}>
                <div className="feed-title">{action}</div>
                <span className="chip signal">log</span>
              </div>
            ))}
          </div>
        </Panel>
      )}
      <Panel eyebrow="Fleet" title="Runtime products">
        <div className="panelbody repo-list">
          {runtimeProducts.map((product) => (
            <div className="feed-item" key={product.slug}>
              <div>
                <div className="feed-title">{product.title}</div>
                <div className="feed-meta">{product.role || product.repo}</div>
              </div>
              <span className={`chip ${productTone(product.status)}`}>{product.status}</span>
            </div>
          ))}
        </div>
      </Panel>
    </HiveTabFrame>
  );
}

function DefaultsSurface({ health, products, setup }) {
  return (
    <HiveTabFrame health={health} products={products} setup={setup}>
      <div>
        <div className="eyebrow">// HiveCore shared defaults</div>
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
    </HiveTabFrame>
  );
}

function ContractsSurface({ health, products, setup }) {
  return (
    <HiveTabFrame health={health} products={products} setup={setup}>
      <div>
        <div className="eyebrow">// HiveCore contract monitor</div>
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
    </HiveTabFrame>
  );
}

export default function App() {
  const fetch_ = useCallback((url, opts = {}) => fetch(url, opts), []);
  const [activeTab, setActiveTab] = usePersistentProductTab("hive-core", TABS, "suite");
  const [health, setHealth] = useState(null);
  const [setup, setSetup] = useState(null);
  const [products, setProducts] = useState([]);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const [actionMessage, setActionMessage] = useState(null);

  useEffect(() => {
    applySuiteAccent("hive-core");
  }, []);

  const refreshControlPlane = useCallback(async ({ quiet = false } = {}) => {
    if (!quiet) {
      setRunning(true);
      setActionMessage(null);
    }
    setError("");
    const results = await Promise.allSettled([
      fetchJson(fetch_, `${API}/health`),
      fetchEnvelope(fetch_, "/setup/first-stack"),
      fetchEnvelope(fetch_, "/products"),
    ]);

    const [healthResult, setupResult, productsResult] = results;
    if (healthResult.status === "fulfilled") setHealth(healthResult.value);
    if (setupResult.status === "fulfilled") setSetup(setupResult.value);
    if (productsResult.status === "fulfilled") setProducts(Array.isArray(productsResult.value) ? productsResult.value : []);

    const failed = results.find((result) => result.status === "rejected");
    if (failed) {
      setError(failed.reason?.message || "HiveCore could not load one or more control-plane resources.");
    }
    if (!quiet) setRunning(false);
  }, [fetch_]);

  useEffect(() => {
    refreshControlPlane({ quiet: true });
    const timer = setInterval(() => refreshControlPlane({ quiet: true }), 15000);
    return () => clearInterval(timer);
  }, [refreshControlPlane]);

  const pairFirstStack = async () => {
    setRunning(true);
    setError("");
    setActionMessage(null);
    try {
      const data = await fetchEnvelope(fetch_, "/setup/first-stack/pair", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });
      setSetup(data);
      await refreshControlPlane({ quiet: true });
      const count = data.actions?.length || 0;
      setActionMessage({ tone: "green", text: count ? `Pairing finished with ${count} action log entries.` : "Pairing check finished." });
      setActiveTab("launch");
    } catch (err) {
      setError(err.message || "HiveCore could not pair running products.");
    } finally {
      setRunning(false);
    }
  };

  return (
    <ProductV2Shell productKey="hive-core" productName="HiveCore">
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="HiveCore"
        navLabel="HiveCore navigation"
        onTabChange={setActiveTab}
        productKey="hive-core"
        tabs={TABS}
      />
      {activeTab === "suite" && (
        <SuiteBoard
          actionMessage={actionMessage}
          error={error}
          health={health}
          onPairFirstStack={pairFirstStack}
          onRefresh={() => refreshControlPlane()}
          products={products}
          running={running}
          setup={setup}
        />
      )}
      {activeTab === "launch" && (
        <LaunchStack
          actionMessage={actionMessage}
          error={error}
          health={health}
          onPairFirstStack={pairFirstStack}
          onRefresh={() => refreshControlPlane()}
          products={products}
          running={running}
          setup={setup}
        />
      )}
      {activeTab === "defaults" && <DefaultsSurface health={health} products={products} setup={setup} />}
      {activeTab === "contracts" && <ContractsSurface health={health} products={products} setup={setup} />}
    </ProductV2Shell>
  );
}

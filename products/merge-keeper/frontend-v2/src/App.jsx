import { useMemo, useState } from "react";
import { createApiFetcher, useApiKeyAuth, useProductRuntime } from "@patchhivehq/product-shell/auth";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductV2AuthGate,
  ProductV2Shell,
  ProductRail,
  SuiteRadar,
  SuiteTopline,
} from "@patchhivehq/ui-v2";
import { API } from "./config.js";

const TABS = [
  { id: "readiness", label: "Readiness" },
  { id: "history", label: "Decision log" },
  { id: "checks", label: "Checks" },
];

const TOPLINE_CELLS = [
  { label: "MergeKeeper", value: "Merge array 05", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Readiness" },
  { label: "GitHub", value: "PR state", tone: "sig" },
  { label: "Suite input", value: "3 sources", tone: "warn" },
  { label: "Last call", value: "T-04:00" },
];

const RAIL_SECTIONS = [
  {
    title: "Pull requests",
    items: [
      { label: "patchhive/repo-reaper#42", active: true, pin: true },
      { label: "review-bee#19", value: "ready" },
      { label: "trust-gate#27", value: "hold" },
      { label: "signal-hive#33", value: "clear" },
    ],
  },
  {
    title: "Inputs",
    items: [
      { label: "review pressure", active: true, badge: "warn", badgeTone: "amber" },
      { label: "risk gate", badge: "clear", badgeTone: "green" },
      { label: "ci health", badge: "pass", badgeTone: "green" },
      { label: "memory rules", badge: "watch", badgeTone: "signal" },
    ],
  },
];

const RAIL_STATS = {
  title: "Active PR",
  items: [
    { label: "Repository", value: "patchhive/repo-reaper" },
    { label: "Decision", value: "HOLD", large: true, tone: "warn" },
    { label: "Report", value: "published" },
  ],
};

const METRICS = [
  { label: "Readiness", value: "HOLD", tone: "warn", sub: "2 blockers remain" },
  { label: "Review pressure", value: "5", tone: "warn", sub: "from ReviewBee" },
  { label: "Checks", value: "12/12", tone: "ok", sub: "all required green" },
  { label: "Risk", value: "LOW", tone: "ok", sub: "TrustGate clear" },
  { label: "Time waiting", value: "3h", tone: "sig", sub: "review response" },
];

const SIGNALS = [
  {
    id: "MK-01",
    title: "ReviewBee unresolved action group",
    source: "review",
    tone: "amber",
    state: "hold",
    value: "5 open",
    position: { left: "49%", top: "22%" },
    summary: "Two review asks still look merge-relevant: auth regression test and retry fallback copy.",
  },
  {
    id: "MK-02",
    title: "Required checks passing",
    source: "ci",
    tone: "green",
    state: "ready",
    value: "12/12",
    position: { left: "72%", top: "48%" },
    summary: "All required checks are green and no flaky rerun pressure is visible.",
  },
  {
    id: "MK-03",
    title: "TrustGate risk clear",
    source: "risk",
    tone: "green",
    state: "clear",
    value: "low",
    position: { left: "57%", top: "74%" },
    summary: "Diff avoids sensitive paths and stays under the configured scope cap.",
  },
  {
    id: "MK-04",
    title: "RepoMemory expectation warning",
    source: "memory",
    tone: "signal",
    state: "watch",
    value: "policy",
    position: { left: "28%", top: "61%" },
    summary: "Repo history prefers explicit startup-check notes when product routes change.",
  },
  {
    id: "MK-05",
    title: "Approval state incomplete",
    source: "review",
    tone: "red",
    state: "blocked",
    value: "1 missing",
    position: { left: "31%", top: "34%" },
    summary: "One required reviewer has not re-approved since the latest push.",
  },
];

const LINKS = [
  { from: "MK-01", to: "MK-05", style: { left: "34%", top: "27%", width: "122px", transform: "rotate(-18deg)" } },
  { from: "MK-01", to: "MK-02", style: { left: "51%", top: "35%", width: "132px", transform: "rotate(33deg)" } },
  { from: "MK-02", to: "MK-03", style: { left: "61%", top: "60%", width: "102px", transform: "rotate(112deg)" } },
  { from: "MK-04", to: "MK-03", style: { left: "34%", top: "68%", width: "160px", transform: "rotate(14deg)" } },
];

const FILTERS = [
  { id: "all", label: "all" },
  { id: "review", label: "review" },
  { id: "ci", label: "ci" },
  { id: "risk", label: "risk" },
];

const BLOCKERS = [
  { rank: "01", title: "Missing re-approval after latest push", meta: "required reviewer has not re-approved", tone: "red", label: "block" },
  { rank: "02", title: "Auth regression test still requested", meta: "ReviewBee action group remains open", tone: "amber", label: "hold" },
  { rank: "03", title: "Retry fallback copy needs confirmation", meta: "config clarity requested in review", tone: "amber", label: "hold" },
  { rank: "04", title: "Required CI checks passing", meta: "all required checks are green", tone: "green", label: "clear" },
];

const EVIDENCE = [
  { title: "ReviewBee pressure imported", meta: "5 open asks collapsed from review threads", label: "warn", tone: "amber" },
  { title: "TrustGate clear", meta: "policy risk low, no sensitive-path hit", label: "clear", tone: "green" },
  { title: "RepoMemory expectation present", meta: "startup-check note recommended", label: "watch", tone: "signal" },
];

const HISTORY = [
  { title: "repo-reaper#41", meta: "ready after ReviewBee checklist cleared", label: "ready", tone: "green" },
  { title: "trust-gate#27", meta: "held on policy warning and missing reviewer", label: "hold", tone: "amber" },
  { title: "signal-hive#33", meta: "checks green and no unresolved reviews", label: "ready", tone: "green" },
];

function ReadinessMap() {
  return (
    <SuiteRadar
      ariaLabel="MergeKeeper readiness radar"
      detailLabel="Readiness reason"
      feed={[
        { text: "ReviewBee pressure is the current merge blocker.", tone: "amber" },
        { text: "TrustGate and required checks are clear.", tone: "green" },
        { text: "RepoMemory recommends a startup-check note before merge." },
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
          { label: "Decision", value: signal.tone === "red" ? "block" : signal.tone === "amber" ? "hold" : "clear" },
          { label: "Consumer", value: "merge" },
        ],
        vector: signal.id,
        vectorTone: signal.tone === "amber" || signal.tone === "red" ? "warn" : "",
      }))}
      signalLabel="signals"
      vectorLabel="Selected signal"
    />
  );
}

function BlockerPanel() {
  return (
    <Panel eyebrow="Decision" title="Merge blockers" action={<span className="chip amber">hold</span>}>
      <div className="panelbody repo-list queue-grid">
        {BLOCKERS.map((item) => (
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
      <Panel eyebrow="Evidence" title="Suite inputs">
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
      <Panel eyebrow="Output" title="Publish posture">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">GitHub check</span><span className="chip amber">hold</span></div>
          <div className="rowline"><span className="muted">Maintained comment</span><span className="chip green">fresh</span></div>
          <div className="rowline"><span className="muted">HiveCore signal</span><span className="chip signal">ready</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function ReadinessSurface() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - merge readiness</div>
              <h1>Readiness Scope</h1>
              <p className="subline">GitHub state, review pressure, policy risk, and repo memory collapsed into one merge call.</p>
            </div>
            <div className="actions">
              <span className="chip amber">hold</span>
              <span className="chip signal">3 suite inputs</span>
              <button className="btn primary" type="button">Assess PR</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Readiness" title="Merge pressure map" action={<span className="chip signal">merge radar</span>}>
              <ReadinessMap />
            </Panel>
            <BlockerPanel />
          </div>
        </main>
        <SidePanels />
      </div>
    </>
  );
}

function HistorySurface() {
  return (
    <div className="product-page-shell">
      <div>
        <div className="eyebrow">// MergeKeeper readiness queue</div>
        <h1>Decision Log</h1>
        <p className="subline">Saved readiness calls and the evidence that changed them.</p>
      </div>
      <Panel eyebrow="Recent" title="Readiness history">
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
    <div className="product-page-shell">
      <div className="eyebrow">// MergeKeeper readiness queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("readiness");
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "merge-keeper_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

  if (!ready) {
    return (
      <ProductV2AuthGate
        apiBase={API}
        auth={auth}
        keyPrefix="merge-keeper-"
        productKey="merge-keeper"
        productName="MergeKeeper"
      />
    );
  }

  return (
    <ProductV2Shell authConfigured={authConfigured} runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="MergeKeeper"
        navLabel="MergeKeeper navigation"
        onTabChange={setActiveTab}
        productKey="merge-keeper"
        tabs={TABS}
      />
      {activeTab === "readiness" && <ReadinessSurface />}
      {activeTab === "history" && <HistorySurface />}
      {activeTab === "checks" && (
        <Placeholder
          title="Checks"
          body="GitHub, webhook, and integration readiness surface for MergeKeeper."
        />
      )}
    </ProductV2Shell>
  );
}

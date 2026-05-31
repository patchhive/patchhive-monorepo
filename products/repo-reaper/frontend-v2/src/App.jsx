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
  { id: "mission", label: "Mission deck" },
  { id: "dryrun", label: "Dry Stalk" },
  { id: "prs", label: "PR monitor" },
];

const TOPLINE_CELLS = [
  { label: "RepoReaper", value: "Autonomy rig 01", tone: "sig" },
  { label: "System", value: "Armed", tone: "warn" },
  { label: "Mode", value: "Guarded write" },
  { label: "GitHub", value: "Bot identity", tone: "sig" },
  { label: "RepoMemory", value: "Context on", tone: "green" },
  { label: "Last run", value: "T-08:00" },
];

const RAIL_SECTIONS = [
  {
    title: "Run modes",
    items: [
      { label: "full hunt", active: true, pin: true },
      { label: "dry stalk", value: "safe" },
      { label: "watch mode", badge: "off", badgeTone: "amber" },
      { label: "manual issue", value: "ready" },
    ],
  },
  {
    title: "Gates",
    items: [
      { label: "smith confidence", active: true, badge: "0.83", badgeTone: "green" },
      { label: "test sandbox", badge: "docker", badgeTone: "signal" },
      { label: "budget cap", badge: "$2.50", badgeTone: "amber" },
      { label: "trustgate handoff", badge: "next", badgeTone: "signal" },
    ],
  },
];

const RAIL_STATS = {
  title: "Active target",
  items: [
    { label: "Repository", value: "acme/api-gateway" },
    { label: "Run confidence", value: "86%", large: true, tone: "green" },
    { label: "Cost", value: "$0.1742" },
  ],
};

const METRICS = [
  { label: "Candidates", value: "42", tone: "sig", sub: "8 above threshold" },
  { label: "Patch confidence", value: "86", tone: "ok", sub: "smith approved" },
  { label: "Validation", value: "3/3", tone: "ok", sub: "tests passing" },
  { label: "Rejected", value: "5", tone: "warn", sub: "logged with feedback" },
  { label: "Run cost", value: "$0.17", tone: "sig", sub: "$0.41 lifetime today" },
];

const AGENTS = [
  {
    id: "scout",
    label: "Scout",
    phase: "hunt",
    status: "complete",
    tone: "green",
    confidence: "0.88",
    cost: "$0.021",
    summary: "Found eight fixable bug issues after topic, language, stale activity, and repo health filters.",
  },
  {
    id: "judge",
    label: "Judge",
    phase: "scope",
    status: "complete",
    tone: "green",
    confidence: "0.84",
    cost: "$0.033",
    summary: "Selected route parser and regression tests as the smallest useful patch surface.",
  },
  {
    id: "reaper",
    label: "Reaper",
    phase: "patch",
    status: "complete",
    tone: "signal",
    confidence: "0.79",
    cost: "$0.071",
    summary: "Generated a guarded parser fix and updated the failing request normalization path.",
  },
  {
    id: "smith",
    label: "Smith",
    phase: "review",
    status: "approved",
    tone: "green",
    confidence: "0.86",
    cost: "$0.041",
    summary: "Trimmed a broad helper change, kept the patch narrow, and raised review confidence above the gate.",
  },
  {
    id: "gatekeeper",
    label: "Gatekeeper",
    phase: "validate",
    status: "ready",
    tone: "amber",
    confidence: "0.81",
    cost: "$0.008",
    summary: "Validation passed in sandbox. PR is ready to publish after final operator review.",
  },
];

const CANDIDATES = [
  { rank: "01", repo: "acme/api-gateway", issue: "Route params dropped on nested proxy paths", score: "91", tone: "green" },
  { rank: "02", repo: "helium-labs/taskboard", issue: "Saved filters lose date range", score: "84", tone: "signal" },
  { rank: "03", repo: "oxide-ci/log-tap", issue: "Retry summary hides failing step", score: "77", tone: "amber" },
];

const VALIDATION = [
  { title: "Patch apply", meta: "clean apply after Smith refinement", label: "pass", tone: "green" },
  { title: "Unit tests", meta: "cargo test route_params --locked", label: "pass", tone: "green" },
  { title: "Sandbox", meta: "Docker test execution, host tests disabled", label: "safe", tone: "signal" },
  { title: "Attribution", meta: "PatchHive autonomous PR body ready", label: "ready", tone: "amber" },
];

const REJECTIONS = [
  { title: "Broad fixture rewrite", meta: "Smith rejected scope drift before validation", label: "logged", tone: "amber" },
  { title: "Flaky upstream test", meta: "Gatekeeper held PR and kept FailGuard note", label: "held", tone: "red" },
  { title: "Missing issue reproduction", meta: "Scout confidence below minimum threshold", label: "skip", tone: "" },
];

const PR_OUTCOMES = [
  { title: "PR body", meta: "Autonomous attribution and evidence summary", label: "ready", tone: "green" },
  { title: "Branch", meta: "patchhive/reaper-route-param-fix", label: "staged", tone: "signal" },
  { title: "Maintainer signal", meta: "Small patch, tests included, no workflow writes", label: "strong", tone: "green" },
];

function AgentPipeline() {
  const positions = [
    { left: "31%", top: "35%" },
    { left: "49%", top: "23%" },
    { left: "72%", top: "48%" },
    { left: "58%", top: "73%" },
    { left: "27%", top: "65%" },
  ];

  return (
    <SuiteRadar
      ariaLabel="RepoReaper agent pipeline radar"
      detailLabel="Agent report"
      feed={[
        { text: "Smith trimmed broad helper drift before validation.", tone: "green" },
        { text: "Gatekeeper passed sandbox validation and is waiting on operator review.", tone: "amber" },
        { text: "Candidate queue remains under budget and above confidence threshold." },
      ]}
      gainLabel="Confidence"
      items={AGENTS.map((agent, index) => ({
        ...agent,
        detail: agent.status,
        gain: agent.confidence,
        gainMeta: `cost ${agent.cost}`,
        label: `${String(index + 1).padStart(2, "0")} ${agent.label}`,
        position: positions[index],
        stats: [
          { label: "Agent", value: agent.label },
          { label: "Phase", value: agent.phase },
          { label: "Status", value: agent.status },
          { label: "Cost", value: agent.cost },
          { label: "Gate", value: agent.confidence },
        ],
        summary: agent.summary,
        title: agent.label,
        vector: agent.phase,
        vectorTone: agent.tone === "amber" ? "warn" : "",
      }))}
      signalLabel="agents"
      vectorLabel="Active phase"
    />
  );
}

function CandidatePanel() {
  return (
    <Panel eyebrow="Scout" title="Candidate queue" action={<span className="chip signal">8 viable</span>}>
      <div className="panelbody repo-list candidate-grid">
        {CANDIDATES.map((item) => (
          <article className="repo-card" key={item.rank}>
            <div className="repo-head">
              <div>
                <div className="repo-name">{item.repo}</div>
                <p className="muted">{item.issue}</p>
              </div>
              <div className={`score ${item.tone === "green" ? "ok" : item.tone}`}>{item.score}</div>
            </div>
            <div className="repo-meta">
              <span className="chip">rank {item.rank}</span>
              <span className={`chip ${item.tone}`}>fixable</span>
              <span className="chip signal">bug</span>
            </div>
          </article>
        ))}
      </div>
    </Panel>
  );
}

function ValidationPanel() {
  return (
    <Panel eyebrow="Gatekeeper" title="Validation gates" action={<span className="chip green">3 passed</span>}>
      <div className="panelbody repo-list">
        {VALIDATION.map((item) => (
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

function SidePanels() {
  return (
    <aside className="side">
      <ValidationPanel />
      <Panel eyebrow="Rejected" title="Smith feedback">
        <div className="panelbody repo-list">
          {REJECTIONS.map((item) => (
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

function PrOutcomePanel() {
  return (
    <Panel eyebrow="Output" title="PR delivery posture">
      <div className="panelbody repo-list queue-grid">
        {PR_OUTCOMES.map((item) => (
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

function MissionDeck() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - autonomous patch run</div>
              <h1>Mission Deck</h1>
              <p className="subline">Outbound contribution made reviewable: candidates, agents, confidence, validation, and PR posture in one run view.</p>
            </div>
            <div className="actions">
              <span className="chip amber">guarded write</span>
              <span className="chip signal">repo memory on</span>
              <button className="btn primary" type="button">Start hunt</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Pipeline" title="Agent execution chain" action={<span className="chip amber">operator review</span>}>
              <AgentPipeline />
            </Panel>
            <CandidatePanel />
          </div>
          <PrOutcomePanel />
        </main>
        <SidePanels />
      </div>
    </>
  );
}

function Placeholder({ title, body }) {
  return (
    <div className="product-page-shell">
      <div className="eyebrow">// RepoReaper patch queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("mission");
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "repo-reaper_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

  if (!ready) {
    return (
      <ProductV2AuthGate
        apiBase={API}
        auth={auth}
        keyPrefix="rr-"
        productKey="repo-reaper"
        productName="RepoReaper"
      />
    );
  }

  return (
    <>
      <ProductV2Shell authConfigured={authConfigured} runtime={runtime}>
        <DeckBar
          activeTab={activeTab}
          brandEyebrow="PatchHive"
          brandName="RepoReaper"
          navLabel="RepoReaper navigation"
          onTabChange={setActiveTab}
          productKey="repo-reaper"
          tabs={TABS}
        />
        {activeTab === "mission" && <MissionDeck />}
        {activeTab === "dryrun" && (
          <Placeholder
            title="Dry Stalk"
            body="Safe-run surface for inspecting hunt quality before live pull request creation."
          />
        )}
        {activeTab === "prs" && (
          <Placeholder
            title="PR Monitor"
            body="Outbound contribution history, confidence, and maintainer response surface."
          />
        )}
      </ProductV2Shell>
    </>
  );
}

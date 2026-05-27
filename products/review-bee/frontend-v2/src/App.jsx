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
  { id: "threads", label: "Thread map" },
  { id: "history", label: "Review history" },
  { id: "checks", label: "Checks" },
];

const TOPLINE_CELLS = [
  { label: "ReviewBee", value: "Review array 04", tone: "sig" },
  { label: "System", value: "Online", tone: "ok" },
  { label: "Mode", value: "Review only" },
  { label: "GitHub", value: "PR reads", tone: "sig" },
  { label: "Status", value: "Fresh", tone: "green" },
  { label: "Last refresh", value: "T-06:00" },
];

const RAIL_SECTIONS = [
  {
    title: "Pull requests",
    items: [
      { label: "patchhive/repo-reaper#42", active: true, pin: true },
      { label: "signal-hive#18", value: "clear" },
      { label: "trust-gate#27", value: "watch" },
      { label: "repo-memory#31", value: "2 open" },
    ],
  },
  {
    title: "Review state",
    items: [
      { label: "unresolved", active: true, badge: "5", badgeTone: "amber" },
      { label: "duplicates", badge: "8", badgeTone: "signal" },
      { label: "resolved", badge: "11", badgeTone: "green" },
      { label: "noise", badge: "6" },
    ],
  },
];

const RAIL_STATS = {
  title: "Active PR",
  items: [
    { label: "Repository", value: "patchhive/repo-reaper" },
    { label: "Pressure", value: "ATTN", large: true, tone: "warn" },
    { label: "Comment", value: "fresh" },
  ],
};

const METRICS = [
  { label: "Threads read", value: "28", tone: "sig", sub: "14 review comments" },
  { label: "Open asks", value: "5", tone: "warn", sub: "2 blocking" },
  { label: "Duplicate asks", value: "8", tone: "sig", sub: "collapsed to 3" },
  { label: "Resolved", value: "11", tone: "ok", sub: "appears addressed" },
  { label: "Checklist", value: "7", tone: "warn", sub: "maintained comment" },
];

const THREADS = [
  {
    id: "T-102",
    title: "Add route test for auth edge case",
    cluster: "tests",
    tone: "red",
    state: "blocking",
    count: "3 comments",
    position: { left: "48%", top: "22%" },
    summary: "Three reviewers asked for a direct route test before this patch can merge.",
  },
  {
    id: "T-097",
    title: "Clarify Retry_COUNT fallback",
    cluster: "config",
    tone: "amber",
    state: "open",
    count: "4 comments",
    position: { left: "72%", top: "46%" },
    summary: "Repeated feedback points at unclear retry behavior in the config panel and README note.",
  },
  {
    id: "T-083",
    title: "Remove stale helper rename",
    cluster: "scope",
    tone: "amber",
    state: "open",
    count: "2 comments",
    position: { left: "33%", top: "48%" },
    summary: "A nonessential helper rename is making the diff harder to review and should be split out.",
  },
  {
    id: "T-068",
    title: "Status copy is addressed",
    cluster: "copy",
    tone: "green",
    state: "resolved",
    count: "5 comments",
    position: { left: "60%", top: "73%" },
    summary: "The status copy thread appears handled by the latest pushed commit.",
  },
  {
    id: "T-051",
    title: "Question only, no action",
    cluster: "noise",
    tone: "signal",
    state: "tracked",
    count: "1 comment",
    position: { left: "24%", top: "70%" },
    summary: "This is useful reviewer context, but it does not need to become a checklist item.",
  },
];

const THREAD_LINKS = [
  { from: "T-102", to: "T-097", style: { left: "50%", top: "29%", width: "130px", transform: "rotate(38deg)" } },
  { from: "T-097", to: "T-083", style: { left: "40%", top: "48%", width: "190px", transform: "rotate(178deg)" } },
  { from: "T-083", to: "T-051", style: { left: "25%", top: "57%", width: "88px", transform: "rotate(104deg)" } },
  { from: "T-068", to: "T-097", style: { left: "61%", top: "60%", width: "92px", transform: "rotate(-54deg)" } },
];

const FILTERS = [
  { id: "all", label: "all" },
  { id: "tests", label: "tests" },
  { id: "config", label: "config" },
  { id: "scope", label: "scope" },
];

const CHECKLIST = [
  { rank: "01", title: "Add route-level auth regression test", meta: "blocking - 3 related comments", tone: "red", label: "open" },
  { rank: "02", title: "Explain retry fallback behavior", meta: "config docs and UI copy", tone: "amber", label: "open" },
  { rank: "03", title: "Split helper rename from fix", meta: "scope cleanup requested twice", tone: "amber", label: "open" },
  { rank: "04", title: "Confirm status text change", meta: "appears resolved in latest commit", tone: "green", label: "done" },
];

const EVIDENCE = [
  { title: "Maintained comment is fresh", meta: "last update includes latest review event", label: "fresh", tone: "green" },
  { title: "Two asks are merge-blocking", meta: "test coverage and config clarity", label: "attention", tone: "amber" },
  { title: "Noise filtered out", meta: "six comments kept out of checklist", label: "quiet", tone: "signal" },
];

const HISTORY = [
  { title: "repo-reaper#41", meta: "clear after 2 checklist revisions", label: "clear", tone: "green" },
  { title: "signal-hive#18", meta: "one stale feedback cluster remains", label: "watch", tone: "amber" },
  { title: "trust-gate#27", meta: "thread refresh from webhook event", label: "fresh", tone: "signal" },
];

function ThreadMap() {
  return (
    <SuiteRadar
      ariaLabel="ReviewBee thread radar"
      detailLabel="Thread summary"
      feed={[
        { text: "Maintained comment is fresh and includes the latest review event." },
        { text: "Two asks are still merge-blocking.", tone: "amber" },
        { text: "Six low-action comments stay out of the checklist." },
      ]}
      gainLabel="State"
      items={THREADS.map((thread) => ({
        ...thread,
        detail: thread.title,
        gain: thread.state,
        gainMeta: thread.count,
        label: thread.id,
        stats: [
          { label: "Cluster", value: thread.cluster },
          { label: "State", value: thread.state },
          { label: "Count", value: thread.count },
          { label: "Action", value: thread.tone === "green" ? "done" : "open" },
          { label: "Queue", value: "checklist" },
        ],
        vector: thread.id,
        vectorTone: thread.tone === "amber" || thread.tone === "red" ? "warn" : "",
      }))}
      signalLabel="threads"
      vectorLabel="Selected thread"
    />
  );
}

function ChecklistPanel() {
  return (
    <Panel eyebrow="Checklist" title="Action groups" action={<span className="chip amber">5 open</span>}>
      <div className="panelbody repo-list queue-grid">
        {CHECKLIST.map((item) => (
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
      <Panel eyebrow="Evidence" title="Review pressure">
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
      <Panel eyebrow="Output" title="Comment posture">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">Maintained comment</span><span className="chip green">fresh</span></div>
          <div className="rowline"><span className="muted">Webhook refresh</span><span className="chip signal">armed</span></div>
          <div className="rowline"><span className="muted">MergeKeeper input</span><span className="chip amber">pending</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function ThreadSurface() {
  return (
    <>
      <SuiteTopline cells={TOPLINE_CELLS} />
      <div className="main-grid">
        <ProductRail sections={RAIL_SECTIONS} stats={RAIL_STATS} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - review resolution</div>
              <h1>Thread Map</h1>
              <p className="subline">Review comments collapsed into action groups, unresolved pressure, and a maintained PR checklist.</p>
            </div>
            <div className="actions">
              <span className="chip amber">attention</span>
              <span className="chip signal">comment fresh</span>
              <button className="btn primary" type="button">Refresh PR</button>
            </div>
          </div>
          <MetricBand metrics={METRICS} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Threads" title="Resolution map" action={<span className="chip signal">thread radar</span>}>
              <ThreadMap />
            </Panel>
            <ChecklistPanel />
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
        <div className="eyebrow">// ReviewBee v2 extraction queue</div>
        <h1>Review History</h1>
        <p className="subline">Saved review runs and unresolved pressure over time.</p>
      </div>
      <Panel eyebrow="Recent" title="Review runs">
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
      <div className="eyebrow">// ReviewBee v2 extraction queue</div>
      <h1>{title}</h1>
      <p className="subline">{body}</p>
    </div>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = useState("threads");

  return (
    <>
      <DeckBar
        activeTab={activeTab}
        brandName="ReviewBee frontend v2"
        navLabel="ReviewBee v2 surfaces"
        onTabChange={setActiveTab}
        tabs={TABS}
      />
      {activeTab === "threads" && <ThreadSurface />}
      {activeTab === "history" && <HistorySurface />}
      {activeTab === "checks" && (
        <Placeholder
          title="Checks"
          body="This becomes the shared v2 startup and GitHub readiness check surface."
        />
      )}
    </>
  );
}

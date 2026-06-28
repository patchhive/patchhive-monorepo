import { useEffect, useMemo, useRef, useState } from "react";
import { createApiFetcher, useApiKeyAuth, useProductRuntime } from "@patchhivehq/product-shell/auth";
import {
  DeckBar,
  HistoryDetailGrid,
  MetricBand,
  Panel,
  ProductV2AuthGate,
  ProductV2Shell,
  ProductRail,
  SuiteRadar,
  SuiteTopline,
  radarWindowFromTimestamp,
  usePersistentProductTab,
} from "@patchhivehq/ui-v2";
import { API } from "./config.js";

const TABS = [
  { id: "core", label: "Memory core" },
  { id: "history", label: "Ingest history" },
  { id: "failguard", label: "FailGuard" },
  { id: "packs", label: "Prompt packs" },
];

const POSITIONS = [
  { left: "51%", top: "25%" },
  { left: "73%", top: "43%" },
  { left: "43%", top: "67%" },
  { left: "25%", top: "47%" },
  { left: "62%", top: "78%" },
  { left: "33%", top: "32%" },
  { left: "69%", top: "62%" },
  { left: "48%", top: "52%" },
];

const DEFAULT_INGEST_FORM = {
  repo: "",
  merged_pr_limit: "18",
  issue_limit: "24",
  since_days: "180",
};

const DEFAULT_CANDIDATE_FORM = {
  repo: "",
  title: "",
  outcome: "",
  lesson: "",
  prevention: "",
};

const PROMPT_PACK_RUN_STORAGE_KEY = "repo-memory_prompt_pack_run";

function asCount(value) {
  const number = Number(value || 0);
  return Number.isFinite(number) ? number : 0;
}

function timeAgo(value) {
  if (!value) {
    return "never";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return String(value);
  }
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function githubReady(health) {
  return Boolean(health?.github_ready || health?.github?.token_configured);
}

function confidenceTone(confidence) {
  const value = Number(confidence || 0);
  if (value >= 82) return "green";
  if (value >= 65) return "signal";
  if (value >= 45) return "amber";
  return "red";
}

function metricTone(confidence) {
  const tone = confidenceTone(confidence);
  if (tone === "green") return "ok";
  if (tone === "amber") return "warn";
  if (tone === "red") return "hot";
  return "sig";
}

function kindTone(kind, disposition) {
  if (disposition === "policy") return "green";
  if (disposition === "suppressed") return "red";
  if (kind === "failure_pattern") return "amber";
  if (kind === "testing_expectation" || kind === "review_rule") return "green";
  return "signal";
}

function shortKind(kind = "") {
  return kind.replaceAll("_", " ") || "memory";
}

function formatConfidence(value) {
  const number = Number(value || 0);
  if (!Number.isFinite(number)) return "0";
  return String(Math.round(number));
}

function rememberPromptPackRun(id) {
  if (!id || typeof window === "undefined") {
    return;
  }
  window.localStorage.setItem(PROMPT_PACK_RUN_STORAGE_KEY, id);
}

function rememberedPromptPackRun() {
  if (typeof window === "undefined") {
    return "";
  }
  return window.localStorage.getItem(PROMPT_PACK_RUN_STORAGE_KEY) || "";
}

function forgetPromptPackRun() {
  if (typeof window === "undefined") {
    return;
  }
  window.localStorage.removeItem(PROMPT_PACK_RUN_STORAGE_KEY);
}

async function parseJsonResponse(response, fallbackError) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || data.message || fallbackError);
  }
  return data;
}

function memoryList(overview, memories) {
  const featured = overview?.featured_memories || [];
  return featured.length ? featured : memories;
}

function openCandidates(candidates) {
  return candidates.filter((candidate) => candidate.status === "open" || !candidate.status);
}

function buildTopline(health, overview, candidates, history) {
  const counts = overview?.counts || health?.counts || {};
  const latest = history[0];
  return [
    { label: "RepoMemory", value: "Memory spine", tone: "sig" },
    { label: "System", value: health?.status || "checking", tone: health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: "Context first" },
    { label: "GitHub", value: githubReady(health) ? "connected" : "token missing", tone: githubReady(health) ? "sig" : "warn" },
    { label: "FailGuard", value: `${openCandidates(candidates).length} queued`, tone: openCandidates(candidates).length ? "warn" : "ok" },
    { label: "Last ingest", value: latest?.created_at ? timeAgo(latest.created_at) : counts.runs ? "loaded" : "none" },
  ];
}

function buildRail(overview, memories, candidates, history, selectedRepo) {
  const repos = overview?.repos || [];
  const pinned = memories.filter((memory) => memory.pinned).length;
  const policy = memories.filter((memory) => memory.disposition === "policy").length;
  const suppressed = memories.filter((memory) => memory.disposition === "suppressed").length;
  return {
    sections: [
      {
        title: "Repositories",
        items: repos.length
          ? repos.slice(0, 4).map((repo) => ({
            active: repo.repo === selectedRepo,
            label: repo.repo,
            value: String(repo.memory_count || 0),
          }))
          : [{ label: "no ingests yet", active: true, value: "0" }],
      },
      {
        title: "Curation",
        items: [
          { label: "pinned policy", active: true, badge: String(policy || pinned), badgeTone: "green" },
          { label: "soft context", badge: String(Math.max(0, memories.length - policy - suppressed)), badgeTone: "signal" },
          { label: "needs review", badge: String(openCandidates(candidates).length), badgeTone: openCandidates(candidates).length ? "amber" : "green" },
          { label: "suppressed", badge: String(suppressed), badgeTone: suppressed ? "red" : "signal" },
        ],
      },
    ],
    stats: {
      title: "Active repo",
      items: [
        { label: "Repository", value: selectedRepo || repos[0]?.repo || "none" },
        { label: "Memories", value: String(memories.filter((memory) => !selectedRepo || memory.repo === selectedRepo).length), large: true, tone: "sig" },
        { label: "Runs", value: String(history.filter((item) => !selectedRepo || item.repo === selectedRepo).length) },
      ],
    },
  };
}

function buildMetrics(health, overview, memories, candidates) {
  const counts = overview?.counts || health?.counts || {};
  const policy = memories.filter((memory) => memory.disposition === "policy").length;
  const pinned = memories.filter((memory) => memory.pinned).length;
  const failures = memories.filter((memory) => memory.kind === "failure_pattern").length;
  return [
    { label: "Memories", value: String(asCount(counts.memories || memories.length)), tone: "sig", sub: `${asCount(counts.runs)} ingests` },
    { label: "Pinned policies", value: String(policy || pinned), tone: "ok", sub: "guardrail ready" },
    { label: "FailGuard queue", value: String(openCandidates(candidates).length), tone: openCandidates(candidates).length ? "warn" : "ok", sub: `${candidates.length} total` },
    { label: "Repos", value: String(asCount(counts.repos)), tone: "sig", sub: "known history" },
    { label: "Failure patterns", value: String(failures), tone: failures ? "warn" : "ok", sub: "learned lessons" },
  ];
}

function repoInitials(repo = "", fallback = "RM") {
  const parts = repo.split("/").filter(Boolean);
  if (parts.length >= 2) {
    return `${parts[0].charAt(0)}${parts[1].charAt(0)}`.toUpperCase();
  }
  return (repo || fallback).slice(0, 2).toUpperCase();
}

function runSummary(run) {
  const memoriesCreated = asCount(run.memories_created);
  const partialReads = asCount(run.partial_read_warnings);
  if (memoriesCreated > 0) {
    const partialNote = partialReads ? ` ${partialReads} GitHub evidence read${partialReads === 1 ? " was" : "s were"} partial, so rerun with stronger access before treating this as complete coverage.` : "";
    return `${run.repo} produced ${memoriesCreated} durable ${memoriesCreated === 1 ? "memory" : "memories"} on its latest ingest.${partialNote}`;
  }
  if (partialReads > 0) {
    return `${run.repo} was scanned with partial GitHub evidence: ${partialReads} read${partialReads === 1 ? " was" : "s were"} unavailable, so RepoMemory did not have enough repeated context to promote durable memories yet.`;
  }
  return `${run.repo} was scanned, but no repeated convention or failure pattern crossed RepoMemory's confidence threshold yet.`;
}

function buildRadarItems(overview, memories, history = []) {
  if (!history.length) {
    return [{
      detail: "No memory ingests yet",
      gain: "standby",
      gainMeta: "run ingest",
      id: "repo-memory-ready",
      label: "RM",
      position: { left: "50%", top: "44%" },
      stats: [
        { label: "Mode", value: "context" },
        { label: "Ingest", value: "ready" },
        { label: "GitHub", value: "required" },
        { label: "Prompt", value: "empty" },
        { label: "Action", value: "run" },
      ],
      summary: "Run an ingest against an owner/repo to populate RepoMemory's live memory radar.",
      title: "RepoMemory ready",
      tone: "signal",
      vector: "READY",
    }];
  }

  return history.map((run, index) => {
    const minWindow = radarWindowFromTimestamp(run.created_at);
    if (!minWindow) {
      return null;
    }
    const memoriesCreated = asCount(run.memories_created);
    return {
      detail: run.repo,
      gain: memoriesCreated ? String(memoriesCreated) : "early",
      gainMeta: memoriesCreated ? "memories created" : "0 memories",
      gainTone: memoriesCreated ? "ok" : "sig",
      id: `ingest-${run.id}`,
      label: repoInitials(run.repo, `R${index + 1}`),
      minWindow,
      position: POSITIONS[index % POSITIONS.length],
      stats: [
        { label: "Run", value: "saved" },
        { label: "Memories", value: String(memoriesCreated) },
        { label: "Conventions", value: String(asCount(run.conventions)) },
        { label: "Failures", value: String(asCount(run.failures)) },
        { label: "Partial", value: String(asCount(run.partial_read_warnings)) },
        { label: "Age", value: timeAgo(run.created_at) },
      ],
      summary: asCount(run.partial_read_warnings) ? runSummary(run) : run.top_memory || runSummary(run),
      title: run.repo || `Ingest ${index + 1}`,
      tone: memoriesCreated ? "green" : "signal",
      vector: memoriesCreated ? "INGEST" : "EARLY SIGNAL",
    };
  }).filter(Boolean);
}

function buildRadarFeed(overview, memories, candidates, history = []) {
  const entries = memoryList(overview, memories);
  const latestRun = history[0];
  if (!entries.length) {
    if (latestRun) {
      const memoriesCreated = asCount(latestRun.memories_created);
      return [
        { text: `${latestRun.repo} saved ${timeAgo(latestRun.created_at)} with ${memoriesCreated} durable ${memoriesCreated === 1 ? "memory" : "memories"}.`, tone: memoriesCreated ? "green" : "signal" },
        { text: asCount(latestRun.partial_read_warnings) ? runSummary(latestRun) : latestRun.top_memory || runSummary(latestRun), tone: memoriesCreated ? "green" : "signal" },
        { text: `${openCandidates(candidates).length} FailGuard candidates are waiting for operator review.`, tone: openCandidates(candidates).length ? "amber" : "green" },
      ];
    }
    return [
      { text: "RepoMemory is waiting for a repo ingest before it can build durable context.", tone: "signal" },
      { text: "FailGuard candidates can still queue lessons for later promotion.", tone: openCandidates(candidates).length ? "amber" : "green" },
      { text: "Prompt packs appear after the first saved ingest.", tone: "signal" },
    ];
  }
  const top = entries[0];
  return [
    latestRun && { text: `${latestRun.repo} latest ingest: ${asCount(latestRun.memories_created)} durable memories, ${timeAgo(latestRun.created_at)}.`, tone: asCount(latestRun.memories_created) ? "green" : "signal" },
    { text: top?.prompt_line || top?.detail || "RepoMemory loaded live memory entries.", tone: kindTone(top?.kind, top?.disposition) },
    { text: `${openCandidates(candidates).length} FailGuard candidates are waiting for operator review.`, tone: openCandidates(candidates).length ? "amber" : "green" },
    { text: `${entries.length} featured memories are available for TrustGate, RepoReaper, and HiveCore handoff.`, tone: "signal" },
  ].filter(Boolean);
}

function StatusBanner({ tone = "signal", children }) {
  if (!children) return null;
  return <div className={`status-banner ${tone}`}>{children}</div>;
}

function Field({ label, children }) {
  return (
    <label className="v2-field">
      <span>{label}</span>
      {children}
    </label>
  );
}

function MemoryLattice({ candidates, history, memories, overview }) {
  const items = useMemo(() => buildRadarItems(overview, memories, history), [overview, memories, history]);
  const feed = useMemo(() => buildRadarFeed(overview, memories, candidates, history), [overview, memories, candidates, history]);
  return (
    <SuiteRadar
      ariaLabel="RepoMemory live memory radar"
      detailLabel="Signal detail"
      feed={feed}
      gainLabel="Signal"
      itemQueryParam="memory"
      items={items}
      signalLabel="signals"
      vectorLabel="Selected signal"
    />
  );
}

function IngestPanel({ error, form, onChange, onRun, running }) {
  const set = (key, value) => onChange((current) => ({ ...current, [key]: value }));
  return (
    <Panel eyebrow="Ingest" title="Repo history intake" action={<span className="chip signal">GitHub read</span>}>
      <form
        className="panelbody control-stack"
        onSubmit={(event) => {
          event.preventDefault();
          onRun();
        }}
      >
        <div className="form-grid">
          <Field label="Repository">
            <input className="v2-input" onChange={(event) => set("repo", event.target.value)} placeholder="owner/repo" value={form.repo} />
          </Field>
          <Field label="Merged PR limit">
            <input className="v2-input" min="5" max="40" onChange={(event) => set("merged_pr_limit", event.target.value)} type="number" value={form.merged_pr_limit} />
          </Field>
          <Field label="Issue limit">
            <input className="v2-input" min="5" max="40" onChange={(event) => set("issue_limit", event.target.value)} type="number" value={form.issue_limit} />
          </Field>
          <Field label="Since days">
            <input className="v2-input" min="30" max="730" onChange={(event) => set("since_days", event.target.value)} type="number" value={form.since_days} />
          </Field>
        </div>
        <div className="actions split-actions">
          <span className="micro">Merged PRs, review feedback, and closed issues become durable memory.</span>
          <button className="btn primary" disabled={running || !form.repo.trim()} type="submit">
            {running ? "Ingesting" : "Run ingest"}
          </button>
        </div>
        {error && <StatusBanner tone="red">{error}</StatusBanner>}
      </form>
    </Panel>
  );
}

function MemoryQueuePanel({ busyCandidate, candidates, memories, onDismissCandidate, onPromoteCandidate }) {
  const open = openCandidates(candidates);
  const queue = open.length ? open : memories.slice(0, 5);
  return (
    <Panel eyebrow="Curation" title={open.length ? "FailGuard review queue" : "Memory queue"} action={<span className={`chip ${open.length ? "amber" : "signal"}`}>{queue.length} items</span>}>
      <div className="panelbody repo-list queue-grid">
        {queue.length ? queue.map((item, index) => {
          const isCandidate = Boolean(item.outcome);
          return (
            <div className="ledger-row" key={item.id || item.memory_ref || index}>
              <div className="rank">{String(index + 1).padStart(2, "0")}</div>
              <div>
                <div className="repo-name">{item.title || item.memory_ref}</div>
                <div className="feed-meta">{isCandidate ? item.outcome : item.prompt_line || item.detail}</div>
                <div className="repo-meta">
                  <span className={`chip ${isCandidate ? "amber" : kindTone(item.kind, item.disposition)}`}>{isCandidate ? item.source_type || "candidate" : shortKind(item.kind)}</span>
                  <span className="chip signal">{item.repo}</span>
                </div>
              </div>
              {isCandidate ? (
                <div className="actions">
                  <button className="btn" disabled={busyCandidate === item.id} onClick={() => onDismissCandidate(item.id)} type="button">Dismiss</button>
                  <button className="btn primary" disabled={busyCandidate === item.id} onClick={() => onPromoteCandidate(item.id)} type="button">Promote</button>
                </div>
              ) : (
                <span className={`chip ${confidenceTone(item.confidence)}`}>{formatConfidence(item.confidence)}</span>
              )}
            </div>
          );
        }) : (
          <div className="empty-v2">
            <strong>No memory queue yet</strong>
            <span>Run an ingest or queue a FailGuard candidate to populate RepoMemory.</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function SidePanels({ candidates, health, memories, overview }) {
  const open = openCandidates(candidates);
  const featured = memoryList(overview, memories).slice(0, 3);
  return (
    <aside className="side">
      <Panel eyebrow="FailGuard" title="Lesson pressure">
        <div className="panelbody repo-list">
          {open.length ? open.slice(0, 3).map((item) => (
            <div className="feed-item" key={item.id}>
              <div>
                <div className="feed-title">{item.title}</div>
                <div className="feed-meta">{item.outcome}</div>
              </div>
              <span className={`chip ${Number(item.confidence || 0) >= 0.75 ? "red" : "amber"}`}>{Math.round(Number(item.confidence || 0) * 100)}%</span>
            </div>
          )) : (
            <div className="rowline"><span className="muted">Open candidates</span><span className="chip green">clear</span></div>
          )}
        </div>
      </Panel>

      <Panel eyebrow="Consumers" title="Context handoff">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">GitHub token</span><span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "ready" : "missing"}</span></div>
          <div className="rowline"><span className="muted">Featured memories</span><span className="chip signal">{featured.length}</span></div>
          <div className="rowline"><span className="muted">Known repos</span><span className="chip signal">{overview?.counts?.repos || 0}</span></div>
        </div>
      </Panel>
    </aside>
  );
}

function MemoryCore({
  busyCandidate,
  candidates,
  error,
  form,
  health,
  history,
  memories,
  onChangeForm,
  onDismissCandidate,
  onPromoteCandidate,
  onRefresh,
  onRunIngest,
  overview,
  running,
}) {
  const selectedRepo = form.repo || history[0]?.repo || overview?.repos?.[0]?.repo || "";
  const rail = useMemo(() => buildRail(overview, memories, candidates, history, selectedRepo), [overview, memories, candidates, history, selectedRepo]);
  const metrics = useMemo(() => buildMetrics(health, overview, memories, candidates), [health, overview, memories, candidates]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, overview, candidates, history)} />
      <div className="main-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - durable context</div>
              <h1>Memory Lattice</h1>
              <p className="subline">Merged history, review pain, and FailGuard lessons distilled into reusable repo context.</p>
            </div>
            <div className="actions">
              <span className={`chip ${githubReady(health) ? "green" : "amber"}`}>{githubReady(health) ? "github ready" : "github missing"}</span>
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
          </div>
          <MetricBand metrics={metrics} />
          <IngestPanel error={error} form={form} onChange={onChangeForm} onRun={onRunIngest} running={running} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Graph" title="Repo knowledge map" action={<span className="chip signal">memory radar</span>}>
              <MemoryLattice candidates={candidates} history={history} memories={memories} overview={overview} />
            </Panel>
            <MemoryQueuePanel
              busyCandidate={busyCandidate}
              candidates={candidates}
              memories={memoryList(overview, memories)}
              onDismissCandidate={onDismissCandidate}
              onPromoteCandidate={onPromoteCandidate}
            />
          </div>
        </main>
        <SidePanels candidates={candidates} health={health} memories={memories} overview={overview} />
      </div>
    </>
  );
}

function SecondaryFrame({ children, candidates, health, history, memories, overview, selectedRepo }) {
  const rail = useMemo(() => buildRail(overview, memories, candidates, history, selectedRepo), [overview, memories, candidates, history, selectedRepo]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, overview, candidates, history)} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">{children}</main>
      </div>
    </>
  );
}

function FailGuardSurface({
  busyCandidate,
  candidateForm,
  candidates,
  error,
  health,
  history,
  memories,
  onChangeCandidateForm,
  onCreateCandidate,
  onDismissCandidate,
  onPromoteCandidate,
  onRefresh,
  overview,
}) {
  const set = (key, value) => onChangeCandidateForm((current) => ({ ...current, [key]: value }));
  return (
    <SecondaryFrame candidates={candidates} health={health} history={history} memories={memories} overview={overview} selectedRepo={candidateForm.repo || history[0]?.repo}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// RepoMemory bad-outcome queue</div>
          <h1>FailGuard Review</h1>
          <p className="subline">Review bad-outcome candidates, then promote useful lessons into durable failure-pattern memory.</p>
        </div>
        <button className="btn" onClick={onRefresh} type="button">Refresh</button>
      </div>
      {error && <StatusBanner tone="red">{error}</StatusBanner>}
      <div className="atlas-layout suite-four-layout">
        <Panel eyebrow="Candidate" title="Queue a lesson">
          <form
            className="panelbody control-stack"
            onSubmit={(event) => {
              event.preventDefault();
              onCreateCandidate();
            }}
          >
            <div className="form-grid">
              <Field label="Repository">
                <input className="v2-input" onChange={(event) => set("repo", event.target.value)} placeholder="owner/repo" value={candidateForm.repo} />
              </Field>
              <Field label="Title">
                <input className="v2-input" onChange={(event) => set("title", event.target.value)} placeholder="What went wrong" value={candidateForm.title} />
              </Field>
            </div>
            <Field label="Outcome">
              <textarea className="v2-input" onChange={(event) => set("outcome", event.target.value)} style={{ minHeight: 88, paddingTop: 10, resize: "vertical" }} value={candidateForm.outcome} />
            </Field>
            <Field label="Lesson">
              <textarea className="v2-input" onChange={(event) => set("lesson", event.target.value)} style={{ minHeight: 88, paddingTop: 10, resize: "vertical" }} value={candidateForm.lesson} />
            </Field>
            <Field label="Prevention">
              <textarea className="v2-input" onChange={(event) => set("prevention", event.target.value)} style={{ minHeight: 88, paddingTop: 10, resize: "vertical" }} value={candidateForm.prevention} />
            </Field>
            <button className="btn primary" disabled={!candidateForm.repo.trim() || !candidateForm.title.trim() || !candidateForm.outcome.trim()} type="submit">
              Queue candidate
            </button>
          </form>
        </Panel>
        <MemoryQueuePanel
          busyCandidate={busyCandidate}
          candidates={candidates}
          memories={memories}
          onDismissCandidate={onDismissCandidate}
          onPromoteCandidate={onPromoteCandidate}
        />
      </div>
    </SecondaryFrame>
  );
}

function HistorySurface({
  candidates,
  error,
  health,
  history,
  loadingPromptPack,
  memories,
  onClearPromptPack,
  onLoadPromptPack,
  onRefresh,
  overview,
  promptPack,
  promptPackRun,
}) {
  const selectedRun = promptPackRun?.id ? history.find((item) => item.id === promptPackRun.id) || null : null;
  return (
    <SecondaryFrame candidates={candidates} health={health} history={history} memories={memories} overview={overview} selectedRepo={selectedRun?.repo}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// RepoMemory ingest ledger</div>
          <h1>Ingest History</h1>
          <p className="subline">Saved repo memory runs, durable context counts, and prompt-pack handoff state.</p>
        </div>
        <div className="actions">
          {promptPack && <button className="btn" onClick={onClearPromptPack} type="button">Clear pack</button>}
          <button className="btn" onClick={onRefresh} type="button">Refresh</button>
          <span className="chip signal">{history.length} saved</span>
        </div>
      </div>
      {error && <StatusBanner tone="red">{error}</StatusBanner>}
      <HistoryDetailGrid>
        <div className="control-stack">
          <Panel eyebrow="History" title="Saved ingests" action={<span className="chip signal">{history.length} runs</span>}>
            <div className="panelbody repo-list queue-grid">
              {history.length ? history.map((item, index) => {
                const active = selectedRun?.id === item.id;
                return (
                  <div className={`ledger-row${active ? " active" : ""}`} key={item.id}>
                    <div className="rank">{active ? "SEL" : String(index + 1).padStart(2, "0")}</div>
                    <div>
                      <div className="repo-name">{item.repo}</div>
                      <div className="feed-meta">{asCount(item.partial_read_warnings) ? runSummary(item) : item.top_memory || runSummary(item)}</div>
                      <div className="repo-meta">
                        <span className="chip signal">{timeAgo(item.created_at)}</span>
                        <span className="chip green">{asCount(item.conventions)} conventions</span>
                        <span className="chip amber">{asCount(item.failures)} failures</span>
                        {asCount(item.partial_read_warnings) > 0 && <span className="chip amber">{asCount(item.partial_read_warnings)} partial</span>}
                        {active && <span className="chip">selected</span>}
                      </div>
                    </div>
                    <button className="btn" disabled={loadingPromptPack} onClick={() => onLoadPromptPack(item.id)} type="button">Load pack</button>
                  </div>
                );
              }) : (
                <div className="empty-v2">
                  <strong>No ingest history</strong>
                  <span>Run an ingest from Memory core to create the first saved RepoMemory run.</span>
                </div>
              )}
            </div>
          </Panel>
          <Panel
            eyebrow="Prompt pack"
            title={promptPackRun?.repo || "Loaded context"}
            action={promptPack ? <button className="btn" onClick={onClearPromptPack} type="button">Clear pack</button> : <span className="chip signal">not loaded</span>}
          >
            <div className="panelbody control-stack">
              <textarea
                className="v2-input"
                readOnly
                style={{ fontFamily: "var(--mono)", lineHeight: 1.45, minHeight: 220, paddingTop: 10, resize: "vertical", whiteSpace: "pre-wrap" }}
                value={promptPack || "Load a saved ingest to inspect the prompt pack from history."}
              />
            </div>
          </Panel>
        </div>
        <Panel
          eyebrow="Run detail"
          title={selectedRun?.repo || "No ingest selected"}
          action={selectedRun ? <button className="btn" disabled={loadingPromptPack} onClick={() => onLoadPromptPack(selectedRun.id)} type="button">Load pack</button> : <span className="chip signal">empty</span>}
        >
          {selectedRun ? (
            <div className="panelbody control-stack">
              <MetricBand
                metrics={[
                  { label: "Memories", value: String(asCount(selectedRun.memories_created)), tone: asCount(selectedRun.memories_created) ? "sig" : "ok", sub: "created" },
                  { label: "Conventions", value: String(asCount(selectedRun.conventions)), tone: "ok", sub: "learned" },
                  { label: "Failures", value: String(asCount(selectedRun.failures)), tone: asCount(selectedRun.failures) ? "warn" : "ok", sub: "patterns" },
                  { label: "Age", value: timeAgo(selectedRun.created_at), tone: "sig", sub: "saved" },
                ]}
              />
              <div className="feed-item">
                <div>
                  <div className="feed-title">Latest memory signal</div>
                  <div className="feed-meta">{asCount(selectedRun.partial_read_warnings) ? runSummary(selectedRun) : selectedRun.top_memory || runSummary(selectedRun)}</div>
                </div>
                <span className={`chip ${asCount(selectedRun.memories_created) ? "green" : "signal"}`}>
                  {asCount(selectedRun.memories_created) ? "durable" : "early"}
                </span>
              </div>
              <div className="rowline"><span className="muted">Run id</span><span className="chip signal">{selectedRun.id}</span></div>
              <div className="rowline"><span className="muted">Created</span><span>{selectedRun.created_at || "unknown"}</span></div>
            </div>
          ) : (
            <div className="empty-v2">
              <strong>No saved run selected</strong>
              <span>Saved ingests appear here after RepoMemory reads repository history.</span>
            </div>
          )}
        </Panel>
      </HistoryDetailGrid>
    </SecondaryFrame>
  );
}

function PromptPackSurface({
  health,
  history,
  loadingPromptPack,
  memories,
  onClearPromptPack,
  onLoadPromptPack,
  overview,
  promptPack,
  promptPackRun,
  candidates,
}) {
  return (
    <SecondaryFrame candidates={candidates} health={health} history={history} memories={memories} overview={overview} selectedRepo={promptPackRun?.repo || history[0]?.repo}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// RepoMemory prompt packs</div>
          <h1>Prompt Packs</h1>
          <p className="subline">Reusable repo context bundles for TrustGate, RepoReaper, and future HiveCore handoffs.</p>
        </div>
        <div className="actions">
          {promptPack && <button className="btn" onClick={onClearPromptPack} type="button">Clear pack</button>}
          <span className="chip signal">{history.length} runs</span>
        </div>
      </div>
      <div className="atlas-layout suite-four-layout">
        <Panel eyebrow="Runs" title="Saved ingests" action={<span className="chip signal">{history.length} saved</span>}>
          <div className="panelbody repo-list queue-grid">
            {history.length ? history.map((item, index) => (
              <div className="ledger-row" key={item.id}>
                <div className="rank">{String(index + 1).padStart(2, "0")}</div>
                <div>
                  <div className="repo-name">{item.repo}</div>
                  <div className="feed-meta">{asCount(item.partial_read_warnings) ? runSummary(item) : item.top_memory || "Saved RepoMemory ingest."}</div>
                  <div className="repo-meta">
                    <span className="chip signal">{timeAgo(item.created_at)}</span>
                    <span className="chip green">{asCount(item.conventions)} conventions</span>
                    <span className="chip amber">{asCount(item.failures)} failures</span>
                    {asCount(item.partial_read_warnings) > 0 && <span className="chip amber">{asCount(item.partial_read_warnings)} partial</span>}
                  </div>
                </div>
                <button className="btn" disabled={loadingPromptPack} onClick={() => onLoadPromptPack(item.id)} type="button">Load pack</button>
              </div>
            )) : (
              <div className="empty-v2">
                <strong>No prompt packs yet</strong>
                <span>Run an ingest to generate the first prompt pack.</span>
              </div>
            )}
          </div>
        </Panel>
        <Panel
          eyebrow="Context"
          title={promptPackRun?.repo || "Prompt pack"}
          action={promptPack ? <button className="btn" onClick={onClearPromptPack} type="button">Clear pack</button> : <span className="chip signal">empty</span>}
        >
          <div className="panelbody control-stack">
            <textarea
              className="v2-input"
              readOnly
              style={{ fontFamily: "var(--mono)", lineHeight: 1.45, minHeight: 300, paddingTop: 10, resize: "vertical", whiteSpace: "pre-wrap" }}
              value={promptPack || "Load a saved ingest to view its generated prompt pack."}
            />
          </div>
        </Panel>
      </div>
    </SecondaryFrame>
  );
}

export default function App() {
  const [activeTab, setActiveTab] = usePersistentProductTab("repo-memory", TABS, "core");
  const [busyCandidate, setBusyCandidate] = useState("");
  const [candidateForm, setCandidateForm] = useState(DEFAULT_CANDIDATE_FORM);
  const [candidates, setCandidates] = useState([]);
  const [error, setError] = useState("");
  const [history, setHistory] = useState([]);
  const [ingestForm, setIngestForm] = useState(DEFAULT_INGEST_FORM);
  const [loadingPromptPack, setLoadingPromptPack] = useState(false);
  const [memories, setMemories] = useState([]);
  const [overview, setOverview] = useState(null);
  const [promptPack, setPromptPack] = useState("");
  const [promptPackRun, setPromptPackRun] = useState(null);
  const [running, setRunning] = useState(false);
  const autoLoadedPromptPackRef = useRef("");
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "repo-memory_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);

  async function fetchJson(path, options, fallbackError) {
    const response = await fetch_(`${API}${path}`, options);
    return parseJsonResponse(response, fallbackError);
  }

  async function refreshMemoryData() {
    if (!ready) {
      return;
    }
    setError("");
    const [overviewResult, historyResult, memoriesResult, candidatesResult] = await Promise.allSettled([
      fetchJson("/overview", undefined, "RepoMemory could not load overview."),
      fetchJson("/history", undefined, "RepoMemory could not load history."),
      fetchJson("/memories", undefined, "RepoMemory could not load memories."),
      fetchJson("/failguard/candidates", undefined, "RepoMemory could not load FailGuard candidates."),
    ]);
    setOverview(overviewResult.status === "fulfilled" ? overviewResult.value : null);
    setHistory(historyResult.status === "fulfilled" ? historyResult.value.history || [] : []);
    setMemories(memoriesResult.status === "fulfilled" ? memoriesResult.value.memories || [] : []);
    setCandidates(candidatesResult.status === "fulfilled" ? candidatesResult.value.candidates || [] : []);
    const failed = [overviewResult, historyResult, memoriesResult, candidatesResult].find((result) => result.status === "rejected");
    if (failed) {
      setError(failed.reason?.message || "RepoMemory could not load one or more backend resources.");
    }
  }

  useEffect(() => {
    refreshMemoryData();
  }, [ready, fetch_]);

  async function runIngest() {
    setRunning(true);
    setError("");
    try {
      const result = await fetchJson(
        "/ingest",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            repo: ingestForm.repo,
            merged_pr_limit: Number(ingestForm.merged_pr_limit) || 18,
            issue_limit: Number(ingestForm.issue_limit) || 24,
            since_days: Number(ingestForm.since_days) || 180,
          }),
        },
        "RepoMemory could not ingest this repository.",
      );
      setPromptPack(result.prompt_pack || "");
      setPromptPackRun({ id: result.id, repo: result.repo });
      rememberPromptPackRun(result.id);
      await refreshMemoryData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "RepoMemory could not ingest this repository.");
    } finally {
      setRunning(false);
    }
  }

  async function loadPromptPack(id) {
    if (!id) return;
    setLoadingPromptPack(true);
    setError("");
    try {
      const result = await fetchJson(`/history/${id}/prompt-pack`, undefined, "RepoMemory could not load that prompt pack.");
      setPromptPack(result.prompt_pack || "");
      setPromptPackRun({ id: result.id, repo: result.repo });
      rememberPromptPackRun(result.id);
    } catch (err) {
      setError(err.message || "RepoMemory could not load that prompt pack.");
    } finally {
      setLoadingPromptPack(false);
    }
  }

  function clearPromptPack() {
    const firstRunId = history[0]?.id || promptPackRun?.id || rememberedPromptPackRun();
    forgetPromptPackRun();
    autoLoadedPromptPackRef.current = firstRunId || "";
    setPromptPack("");
    setPromptPackRun(null);
    setError("");
  }

  useEffect(() => {
    if (!ready || promptPack || loadingPromptPack || !history.length) {
      return;
    }

    const savedId = rememberedPromptPackRun();
    const savedRun = history.find((item) => item.id === savedId);
    const nextRun = savedRun || history[0];
    if (!nextRun?.id || autoLoadedPromptPackRef.current === nextRun.id) {
      return;
    }

    autoLoadedPromptPackRef.current = nextRun.id;
    loadPromptPack(nextRun.id);
  }, [ready, promptPack, loadingPromptPack, history]);

  async function createCandidate() {
    setError("");
    try {
      await fetchJson(
        "/failguard/candidates",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(candidateForm),
        },
        "RepoMemory could not queue that FailGuard candidate.",
      );
      setCandidateForm(DEFAULT_CANDIDATE_FORM);
      await refreshMemoryData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "RepoMemory could not queue that FailGuard candidate.");
    }
  }

  async function promoteCandidate(id) {
    setBusyCandidate(id);
    setError("");
    try {
      await fetchJson(
        `/failguard/candidates/${encodeURIComponent(id)}/promote`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ disposition: "policy", pinned: true }),
        },
        "RepoMemory could not promote that FailGuard candidate.",
      );
      await refreshMemoryData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "RepoMemory could not promote that FailGuard candidate.");
    } finally {
      setBusyCandidate("");
    }
  }

  async function dismissCandidate(id) {
    setBusyCandidate(id);
    setError("");
    try {
      await fetchJson(
        `/failguard/candidates/${encodeURIComponent(id)}/dismiss`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ reason: "Dismissed from RepoMemory v2." }),
        },
        "RepoMemory could not dismiss that FailGuard candidate.",
      );
      await refreshMemoryData();
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "RepoMemory could not dismiss that FailGuard candidate.");
    } finally {
      setBusyCandidate("");
    }
  }

  if (!ready) {
    return (
      <ProductV2AuthGate
        apiBase={API}
        auth={auth}
        keyPrefix="repo-memory-"
        productKey="repo-memory"
        productName="RepoMemory"
      />
    );
  }

  return (
    <ProductV2Shell authConfigured={authConfigured} productKey="repo-memory" productName="RepoMemory" runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="RepoMemory"
        navLabel="RepoMemory navigation"
        onTabChange={setActiveTab}
        productKey="repo-memory"
        tabs={TABS}
      />
      {activeTab === "core" && (
        <MemoryCore
          busyCandidate={busyCandidate}
          candidates={candidates}
          error={error}
          form={ingestForm}
          health={runtime.health || {}}
          history={history}
          memories={memories}
          onChangeForm={setIngestForm}
          onDismissCandidate={dismissCandidate}
          onPromoteCandidate={promoteCandidate}
          onRefresh={() => {
            refreshMemoryData();
            runtime.refresh();
          }}
          onRunIngest={runIngest}
          overview={overview}
          running={running}
        />
      )}
      {activeTab === "history" && (
        <HistorySurface
          candidates={candidates}
          error={error}
          health={runtime.health || {}}
          history={history}
          loadingPromptPack={loadingPromptPack}
          memories={memories}
          onClearPromptPack={clearPromptPack}
          onLoadPromptPack={loadPromptPack}
          onRefresh={() => {
            refreshMemoryData();
            runtime.refresh();
          }}
          overview={overview}
          promptPack={promptPack}
          promptPackRun={promptPackRun}
        />
      )}
      {activeTab === "failguard" && (
        <FailGuardSurface
          busyCandidate={busyCandidate}
          candidateForm={candidateForm}
          candidates={candidates}
          error={error}
          health={runtime.health || {}}
          history={history}
          memories={memories}
          onChangeCandidateForm={setCandidateForm}
          onCreateCandidate={createCandidate}
          onDismissCandidate={dismissCandidate}
          onPromoteCandidate={promoteCandidate}
          onRefresh={() => {
            refreshMemoryData();
            runtime.refresh();
          }}
          overview={overview}
        />
      )}
      {activeTab === "packs" && (
        <PromptPackSurface
          candidates={candidates}
          health={runtime.health || {}}
          history={history}
          loadingPromptPack={loadingPromptPack}
          memories={memories}
          onClearPromptPack={clearPromptPack}
          onLoadPromptPack={loadPromptPack}
          overview={overview}
          promptPack={promptPack}
          promptPackRun={promptPackRun}
        />
      )}
    </ProductV2Shell>
  );
}

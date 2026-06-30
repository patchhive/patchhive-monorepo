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
import { useProviderModelDiscovery } from "@patchhivehq/ai-models/model-discovery";
import { API } from "./config.js";

const TABS = [
  { id: "mission", label: "Mission deck" },
  { id: "dryrun", label: "Dry Stalk" },
  { id: "history", label: "History" },
  { id: "prs", label: "PR monitor" },
  { id: "checks", label: "Checks" },
];

const POSITIONS = [
  { left: "31%", top: "35%" },
  { left: "49%", top: "23%" },
  { left: "72%", top: "48%" },
  { left: "58%", top: "73%" },
  { left: "27%", top: "65%" },
  { left: "42%", top: "55%" },
  { left: "66%", top: "29%" },
  { left: "36%", top: "73%" },
];

const DEFAULT_PARAMS = {
  language: "python",
  min_stars: "50",
  max_repos: "10",
  max_issues: "10",
  concurrency: "3",
  search_query: "",
  cost_budget_usd: "0",
  retry_count: "3",
  labels: "bug",
};

const DEFAULT_DRY_PARAMS = {
  ...DEFAULT_PARAMS,
  max_repos: "5",
  concurrency: "1",
};

const AGENT_ROLES = [
  { value: "scout", label: "Scout", detail: "scores issues and dry-run candidates" },
  { value: "judge", label: "Judge", detail: "chooses code context" },
  { value: "reaper", label: "Reaper", detail: "generates patches" },
  { value: "smith", label: "Smith", detail: "rejects weak patches" },
  { value: "gatekeeper", label: "Gatekeeper", detail: "validates and prepares PRs" },
];

const PROVIDER_MODELS = {
  anthropic: "claude-sonnet-4-6",
  openai: "gpt-5.4-mini",
  gemini: "gemini-2.5-pro",
  groq: "llama-3.3-70b-versatile",
  custom: "gpt-4.1-mini",
  ollama: "llama3.2",
};

const PROVIDER_LABELS = {
  anthropic: "Anthropic",
  openai: "OpenAI / local gateway",
  gemini: "Gemini",
  groq: "Groq",
  custom: "Custom compatible",
  ollama: "Ollama",
};

function asCount(value) {
  const number = Number(value || 0);
  return Number.isFinite(number) ? number : 0;
}

function formatMoney(value) {
  return `$${Number(value || 0).toFixed(4)}`;
}

function timeAgo(value) {
  if (!value) return "never";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function splitCsv(value) {
  return String(value || "")
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function normalizeRunRequest(params) {
  return {
    language: params.language || "python",
    min_stars: Number(params.min_stars) || 50,
    max_repos: Number(params.max_repos) || 10,
    max_issues: Number(params.max_issues) || 10,
    concurrency: Number(params.concurrency) || 1,
    search_query: params.search_query || "",
    cost_budget_usd: Number(params.cost_budget_usd) || 0,
    retry_count: Number(params.retry_count) || 3,
    labels: splitCsv(params.labels || "bug"),
  };
}

function statusTone(status) {
  const value = String(status || "").toLowerCase();
  if (["fixed", "done", "complete", "success", "idle"].includes(value)) return "green";
  if (["running", "working", "queued", "review", "triage", "scan", "fix"].includes(value)) return "amber";
  if (["error", "failed", "crashed", "rejected"].includes(value)) return "red";
  if (["skipped", "held"].includes(value)) return "amber";
  return "signal";
}

function metricToneFromStatus(status) {
  const tone = statusTone(status);
  if (tone === "red") return "hot";
  if (tone === "amber") return "warn";
  if (tone === "green") return "ok";
  return "sig";
}

function githubReady(config, health) {
  return Boolean(config?.BOT_GITHUB_TOKEN_SET || (health?.bot && health.bot !== "(not set)"));
}

function aiReady(config) {
  return Boolean(config?.PROVIDER_API_KEY_SET || config?.PATCHHIVE_AI_URL || config?.AI_LOCAL_STATUS?.ok || config?.AI_LOCAL_STATUS?.status === "ok");
}

function agentsReady(agents, health) {
  return Boolean((Array.isArray(agents) && agents.length) || asCount(health?.agents));
}

function defaultAgentProvider(config) {
  if (config?.PATCHHIVE_AI_URL || config?.AI_LOCAL_STATUS?.ok || config?.AI_LOCAL_STATUS?.status === "ok") return "openai";
  return "openai";
}

function blankAgent(config, role = "scout") {
  const provider = defaultAgentProvider(config);
  return {
    api_key: "",
    base_url: "",
    bot_token: "",
    bot_user: "",
    model: PROVIDER_MODELS[provider] || "",
    name: `PatchHive ${AGENT_ROLES.find((item) => item.value === role)?.label || "Agent"}`,
    provider,
    role,
  };
}

function blankTeamDefaults(config) {
  const provider = defaultAgentProvider(config);
  return {
    api_key: "",
    base_url: "",
    bot_token: "",
    bot_user: "",
    model: PROVIDER_MODELS[provider] || "",
    provider,
  };
}

function applyTeamDefaults(agent, defaults = {}) {
  const provider = defaults.provider || agent.provider || "openai";
  const model = String(defaults.model || "").trim() || agent.model || PROVIDER_MODELS[provider] || "";
  const next = {
    ...agent,
    model,
    provider,
  };
  ["api_key", "base_url", "bot_token", "bot_user"].forEach((key) => {
    const value = String(defaults[key] || "").trim();
    if (value) next[key] = value;
  });
  return next;
}

function applyTeamDefaultsToAgents(agents, defaults) {
  return agents.map((agent) => applyTeamDefaults(agent, defaults));
}

function starterTeam(config) {
  const provider = defaultAgentProvider(config);
  const model = PROVIDER_MODELS[provider] || "";
  return AGENT_ROLES.map((role) => ({
    api_key: "",
    base_url: "",
    bot_token: "",
    bot_user: "",
    id: `${role.value}-${Date.now()}`,
    model,
    name: `PatchHive ${role.label}`,
    provider,
    role: role.value,
  }));
}

async function parseJsonResponse(response, fallbackError) {
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error || data.message || fallbackError);
  }
  return data;
}

function getIssueKey(issue, index) {
  return String(issue.id || issue.issue_url || `${issue.repo || "repo"}-${issue.issue_number || index}`);
}

function normalizeIssues(value) {
  if (Array.isArray(value)) return value;
  if (value && typeof value === "object") return Object.values(value);
  return [];
}

function runAttempts(run) {
  return Array.isArray(run?.attempts) ? run.attempts : [];
}

function newestRun(history) {
  return Array.isArray(history) && history.length ? history[0] : null;
}

function activeTarget(stream, selectedRun, history) {
  const issues = normalizeIssues(stream.issues);
  if (issues.length) return issues[0];
  const attempts = runAttempts(selectedRun);
  if (attempts.length) return attempts[0];
  const latest = newestRun(history);
  return latest || {};
}

function issueTitle(issue) {
  return issue.title || issue.issue_title || issue.repo || issue.id || "Target issue";
}

function issueRepo(issue) {
  return issue.repo || issue.repository || issue.full_name || "";
}

function issueScore(issue) {
  return asCount(issue.fixability_score ?? issue.score ?? issue.confidence);
}

function buildTopline(health, config, stream, history) {
  const latest = newestRun(history) || {};
  return [
    { label: "RepoReaper", value: "Autonomy rig", tone: "sig" },
    { label: "System", value: health?.run_active || stream.running ? "running" : health?.status || "checking", tone: health?.run_active || stream.running ? "warn" : health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: "Guarded write" },
    { label: "GitHub", value: githubReady(config, health) ? "Bot identity" : "token missing", tone: githubReady(config, health) ? "sig" : "warn" },
    { label: "AI", value: aiReady(config) ? "ready" : "key missing", tone: aiReady(config) ? "green" : "warn" },
    { label: "Last run", value: latest.started_at ? timeAgo(latest.started_at) : "none" },
  ];
}

function buildMetrics(health, stream, history, rejected, lifetimeCost, selectedRun) {
  const issues = normalizeIssues(stream.issues);
  const attempts = runAttempts(selectedRun);
  const latest = selectedRun || newestRun(history) || {};
  const candidateCount = issues.length || attempts.length || asCount(latest.total_attempted);
  const fixed = asCount(stream.done?.total_fixed ?? latest.total_fixed);
  const attempted = asCount(stream.done?.total_attempted ?? latest.total_attempted ?? candidateCount);
  const confidences = [...issues, ...attempts].map(issueScore).filter(Boolean);
  const avgConfidence = confidences.length
    ? Math.round(confidences.reduce((sum, value) => sum + value, 0) / confidences.length)
    : 0;
  return [
    { label: "Candidates", value: String(candidateCount), tone: "sig", sub: `${fixed} fixed` },
    { label: "Patch confidence", value: avgConfidence ? String(avgConfidence) : "-", tone: avgConfidence >= 70 ? "ok" : avgConfidence ? "warn" : "sig", sub: "visible attempts" },
    { label: "Validation", value: attempted ? `${fixed}/${attempted}` : "0/0", tone: fixed ? "ok" : attempted ? "warn" : "sig", sub: "PR attempts" },
    { label: "Rejected", value: String((rejected || []).length), tone: rejected?.length ? "warn" : "ok", sub: "Smith feedback" },
    { label: "Run cost", value: formatMoney(stream.runCost || latest.total_cost_usd), tone: "sig", sub: `${formatMoney(lifetimeCost || health?.lifetime_cost)} lifetime` },
  ];
}

function buildRail(health, config, stream, history, selectedRun, watchMode) {
  const latest = selectedRun || newestRun(history) || {};
  const target = activeTarget(stream, selectedRun, history);
  const targetRepo = issueRepo(target);
  const budget = stream.params?.cost_budget_usd || config?.COST_BUDGET_USD || "0";
  return {
    sections: [
      {
        title: "Run modes",
        items: [
          { label: "full hunt", active: true, pin: true },
          { label: "dry stalk", value: "safe" },
          { label: "watch mode", badge: watchMode ? "on" : "off", badgeTone: watchMode ? "green" : "amber" },
          { label: "run active", value: health?.run_active || stream.running ? "yes" : "no" },
        ],
      },
      {
        title: "Gates",
        items: [
          { label: "agents", active: true, badge: String(asCount(health?.agents || stream.agents?.length)), badgeTone: asCount(health?.agents || stream.agents?.length) ? "green" : "amber" },
          { label: "AI ready", badge: aiReady(config) ? "yes" : "no", badgeTone: aiReady(config) ? "green" : "amber" },
          { label: "budget cap", badge: budget && budget !== "0" ? `$${budget}` : "uncapped", badgeTone: budget && budget !== "0" ? "amber" : "signal" },
          { label: "runs saved", badge: String(history.length), badgeTone: "signal" },
        ],
      },
    ],
    stats: {
      title: "Active target",
      items: [
        { label: "Repository", value: targetRepo || latest.id || "none" },
        { label: "Run confidence", value: issueScore(target) ? `${issueScore(target)}%` : latest.status || "ready", large: true, tone: issueScore(target) >= 70 ? "green" : metricToneFromStatus(latest.status) },
        { label: "Cost", value: formatMoney(stream.runCost || latest.total_cost_usd) },
      ],
    },
  };
}

function buildRunRadarItems(history) {
  if (!history.length) {
    return [{
      detail: "No autonomous run yet",
      gain: "standby",
      gainMeta: "guarded writes",
      id: "repo-reaper-ready",
      label: "RR",
      position: { left: "50%", top: "44%" },
      stats: [
        { label: "Mode", value: "guarded" },
        { label: "History", value: "empty" },
        { label: "PRs", value: "none" },
        { label: "Cost", value: "$0.0000" },
        { label: "Action", value: "start" },
      ],
      summary: "Start a guarded hunt to populate RepoReaper's autonomous run radar.",
      title: "RepoReaper ready",
      tone: "signal",
      vector: "READY",
    }];
  }
  return history.map((run, index) => {
    const minWindow = radarWindowFromTimestamp(run.started_at || run.created_at);
    if (!minWindow) {
      return null;
    }
    const fixed = asCount(run.total_fixed);
    const attempted = asCount(run.total_attempted);
    const tone = statusTone(run.status);
    return {
      detail: run.id,
      gain: run.status || "saved",
      gainMeta: attempted ? `${fixed}/${attempted} fixed` : `${fixed} fixed`,
      id: run.id || `run-${index + 1}`,
      label: run.id ? run.id.slice(0, 6).toUpperCase() : `R${index + 1}`,
      minWindow,
      position: POSITIONS[index % POSITIONS.length],
      stats: [
        { label: "Status", value: run.status || "saved" },
        { label: "Fixed", value: String(fixed) },
        { label: "Attempted", value: String(attempted) },
        { label: "Cost", value: formatMoney(run.total_cost_usd) },
        { label: "Age", value: timeAgo(run.started_at || run.created_at) },
      ],
      summary: `${fixed} fixed of ${attempted} attempted in this saved RepoReaper run.`,
      title: run.id || `Autonomous run ${index + 1}`,
      tone,
      vector: "saved run",
      vectorTone: tone === "red" || tone === "amber" ? "warn" : "",
    };
  }).filter(Boolean);
}

function buildRadarFeed(stream, history, config) {
  if (stream.logs?.length) {
    return stream.logs.slice(-3).map((log) => ({
      text: log.msg || log.message || String(log),
      tone: log.type === "error" ? "red" : log.type === "warn" ? "amber" : log.type === "success" ? "green" : "signal",
    }));
  }
  return [
    { text: history.length ? `${history.length} saved RepoReaper runs are available.` : "RepoReaper is waiting for a guarded run.", tone: "signal" },
    { text: githubReady(config, {}) ? "Bot identity is configured for outbound pull requests." : "Configure bot GitHub access before live hunts.", tone: githubReady(config, {}) ? "green" : "amber" },
    { text: aiReady(config) ? "AI provider or local gateway is ready." : "Configure an AI provider before patch generation.", tone: aiReady(config) ? "green" : "amber" },
  ];
}

function StatusBanner({ tone = "signal", children }) {
  if (!children) return null;
  return <div className={`status-banner ${tone}`}>{children}</div>;
}

function RunRadar({ config, history, stream }) {
  const items = useMemo(() => buildRunRadarItems(history), [history]);
  const feed = useMemo(() => buildRadarFeed(stream, history, config), [stream, history, config]);
  return (
    <SuiteRadar
      ariaLabel="RepoReaper autonomous run radar"
      detailLabel="Run report"
      feed={feed}
      gainLabel="Status"
      itemQueryParam="run"
      items={items}
      signalLabel={history.length ? "runs" : "standby"}
      vectorLabel="Selected run"
    />
  );
}

function RunForm({ disabled, disabledReason, error, onChange, onStart, params, running, title }) {
  const set = (key, value) => onChange((current) => ({ ...current, [key]: value }));
  return (
    <Panel eyebrow="Run" title={title} action={<span className="chip amber">guarded</span>}>
      <form
        className="panelbody control-stack"
        onSubmit={(event) => {
          event.preventDefault();
          onStart();
        }}
      >
        <div className="form-grid">
          <label className="v2-field">
            Language
            <input className="v2-input" onChange={(event) => set("language", event.target.value)} value={params.language} />
          </label>
          <label className="v2-field">
            Topic query
            <input className="v2-input" onChange={(event) => set("search_query", event.target.value)} placeholder="optional GitHub search topic" value={params.search_query} />
          </label>
          <label className="v2-field">
            Min stars
            <input className="v2-input" min="0" onChange={(event) => set("min_stars", event.target.value)} type="number" value={params.min_stars} />
          </label>
          <label className="v2-field">
            Max repos
            <input className="v2-input" min="1" max="100" onChange={(event) => set("max_repos", event.target.value)} type="number" value={params.max_repos} />
          </label>
          <label className="v2-field">
            Max issues
            <input className="v2-input" min="1" max="100" onChange={(event) => set("max_issues", event.target.value)} type="number" value={params.max_issues} />
          </label>
          <label className="v2-field">
            Concurrency
            <input className="v2-input" min="1" max="10" onChange={(event) => set("concurrency", event.target.value)} type="number" value={params.concurrency} />
          </label>
          <label className="v2-field">
            Budget USD
            <input className="v2-input" min="0" onChange={(event) => set("cost_budget_usd", event.target.value)} step="0.01" type="number" value={params.cost_budget_usd} />
          </label>
          <label className="v2-field">
            Labels
            <input className="v2-input" onChange={(event) => set("labels", event.target.value)} value={params.labels} />
          </label>
          <div className="v2-field">
            Action
            <button className="btn primary" disabled={disabled || running} type="submit">
              {running ? "Running..." : title}
            </button>
          </div>
        </div>
        {disabledReason && <StatusBanner tone="amber">{disabledReason}</StatusBanner>}
        {error && <StatusBanner tone="red">{error}</StatusBanner>}
      </form>
    </Panel>
  );
}

function CandidatePanel({ issues, selectedRun, onLoadRun }) {
  const visibleIssues = normalizeIssues(issues);
  const attempts = runAttempts(selectedRun);
  if (visibleIssues.length) {
    return (
      <Panel eyebrow="Scout" title="Candidate queue" action={<span className="chip signal">{visibleIssues.length} visible</span>}>
        <div className="panelbody repo-list candidate-grid">
          {visibleIssues.slice(0, 6).map((item, index) => (
            <article className="repo-card" key={getIssueKey(item, index)}>
              <div className="repo-head">
                <div>
                  <div className="repo-name">{issueRepo(item)}</div>
                  <p className="muted">{issueTitle(item)}</p>
                </div>
                <div className={`score ${issueScore(item) >= 70 ? "ok" : "sig"}`}>{issueScore(item) || index + 1}</div>
              </div>
              <div className="repo-meta">
                <span className="chip">rank {String(index + 1).padStart(2, "0")}</span>
                <span className={`chip ${statusTone(item.status)}`}>{item.status || "queued"}</span>
                <span className="chip signal">bug</span>
              </div>
            </article>
          ))}
        </div>
      </Panel>
    );
  }
  if (attempts.length) {
    return (
      <Panel eyebrow="Attempts" title="Run attempts" action={<span className="chip signal">{attempts.length} saved</span>}>
        <div className="panelbody repo-list queue-grid">
          {attempts.slice(0, 6).map((attempt, index) => (
            <div className="ledger-row" key={attempt.id || index}>
              <div className="rank">{attempt.confidence || index + 1}</div>
              <div>
                <div className="repo-name">{attempt.issue_title || `Issue #${attempt.issue_number}`}</div>
                <div className="feed-meta">{attempt.skip_reason || attempt.pr_url || "Saved issue attempt."}</div>
              </div>
              <span className={`chip ${statusTone(attempt.status)}`}>{attempt.status || "attempt"}</span>
            </div>
          ))}
        </div>
      </Panel>
    );
  }
  return (
    <Panel eyebrow="History" title="Recent runs" action={<span className="chip signal">0 saved</span>}>
      <div className="panelbody repo-list queue-grid">
        <div className="empty-v2">
          <strong>No live candidates</strong>
          <span>Start a hunt or load a saved run to populate the queue.</span>
        </div>
      </div>
    </Panel>
  );
}

function ValidationPanel({ selectedRun, stream }) {
  const attempts = runAttempts(selectedRun);
  const issues = normalizeIssues(stream.issues);
  const source = attempts.length ? attempts : issues;
  const fixed = source.filter((item) => item.status === "fixed" || item.pr_url || item.pr?.url).length;
  const rejected = source.filter((item) => item.status === "rejected" || item.reason).length;
  const pending = source.filter((item) => ["queued", "running"].includes(String(item.status))).length;
  const rows = [
    { title: "Patch output", meta: fixed ? `${fixed} fixes have PR output.` : "No successful PR output in the visible run yet.", label: fixed ? "pass" : "wait", tone: fixed ? "green" : "amber" },
    { title: "Smith feedback", meta: rejected ? `${rejected} attempts were rejected or skipped.` : "No rejected attempts in the visible run.", label: rejected ? "logged" : "clear", tone: rejected ? "amber" : "green" },
    { title: "Active work", meta: pending ? `${pending} candidates are still moving.` : "No visible in-flight candidate work.", label: pending ? "active" : "idle", tone: pending ? "amber" : "signal" },
    { title: "Attribution", meta: "PatchHive autonomous PR attribution stays visible in generated pull requests.", label: "required", tone: "signal" },
  ];
  return (
    <Panel eyebrow="Gatekeeper" title="Validation gates" action={<span className={`chip ${fixed ? "green" : "amber"}`}>{fixed} passed</span>}>
      <div className="panelbody repo-list">
        {rows.map((item) => (
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

function SidePanels({ rejected, selectedRun, stream }) {
  return (
    <aside className="side">
      <ValidationPanel selectedRun={selectedRun} stream={stream} />
      <Panel eyebrow="Rejected" title="Smith feedback">
        <div className="panelbody repo-list">
          {rejected?.length ? rejected.slice(0, 4).map((item) => (
            <div className="feed-item" key={item.id}>
              <div>
                <div className="feed-title">{item.issue_title || item.repo}</div>
                <div className="feed-meta">{item.smith_feedback || item.reason}</div>
              </div>
              <span className="chip amber">{item.confidence || "held"}</span>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No Smith rejections</strong>
              <span>Rejected patches will appear here with feedback.</span>
            </div>
          )}
        </div>
      </Panel>
    </aside>
  );
}

function PrOutcomePanel({ prs }) {
  const visible = Array.isArray(prs) ? prs.slice(0, 5) : [];
  return (
    <Panel eyebrow="Output" title="PR delivery posture" action={<span className="chip signal">{visible.length} tracked</span>}>
      <div className="panelbody repo-list queue-grid">
        {visible.length ? visible.map((item) => (
          <div className="feed-item" key={`${item.repo}-${item.pr_number}`}>
            <div>
              <div className="feed-title">{item.repo} #{item.pr_number}</div>
              <div className="feed-meta">{item.review_state || item.state || "open"} · last checked {timeAgo(item.last_checked || item.opened_at)}</div>
            </div>
            <span className={`chip ${item.merged ? "green" : item.state === "closed" ? "red" : "signal"}`}>{item.merged ? "merged" : item.state || "open"}</span>
          </div>
        )) : (
          <div className="empty-v2">
            <strong>No tracked PRs</strong>
            <span>Successful RepoReaper PRs will appear in this monitor.</span>
          </div>
        )}
      </div>
    </Panel>
  );
}

function MissionDeck({
  agents,
  config,
  error,
  health,
  history,
  lifetimeCost,
  onChangeParams,
  onClearRun,
  onLoadRun,
  onRefresh,
  onStartRun,
  params,
  prs,
  rejected,
  selectedRun,
  stream,
  watchMode,
}) {
  const rail = useMemo(() => buildRail(health, config, { ...stream, params }, history, selectedRun, watchMode), [health, config, stream, params, history, selectedRun, watchMode]);
  const metrics = useMemo(() => buildMetrics(health, stream, history, rejected, lifetimeCost, selectedRun), [health, stream, history, rejected, lifetimeCost, selectedRun]);
  const teamReady = agentsReady(agents, health);
  const disabledReason = !teamReady
    ? "Recruit a RepoReaper agent team in Checks before starting a hunt."
    : !githubReady(config, health)
      ? "Configure the PatchHive bot GitHub token before outbound hunts."
      : !aiReady(config)
        ? "Configure an AI provider or PatchHive Local AI before patch generation."
        : "";
  return (
    <>
      <SuiteTopline cells={buildTopline(health, config, stream, history)} />
      <div className="main-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - autonomous patch run</div>
              <h1>Mission Deck</h1>
              <p className="subline">Outbound contribution made reviewable: candidates, agents, confidence, validation, and PR posture in one run view.</p>
            </div>
            <div className="actions">
              <span className={`chip ${githubReady(config, health) ? "green" : "amber"}`}>{githubReady(config, health) ? "bot ready" : "bot missing"}</span>
              {selectedRun && <button className="btn" onClick={onClearRun} type="button">Clear run</button>}
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
          </div>
          <RunForm disabled={!teamReady || !githubReady(config, health) || !aiReady(config)} disabledReason={disabledReason} error={error} onChange={onChangeParams} onStart={onStartRun} params={params} running={stream.running} title="Start hunt" />
          <MetricBand metrics={metrics} />
          <div className="atlas-layout suite-four-layout">
            <Panel eyebrow="Pipeline" title="Autonomous run map" action={<span className={`chip ${stream.running ? "amber" : "signal"}`}>{stream.phase || "standby"}</span>}>
              <RunRadar config={config} history={history} stream={stream} />
            </Panel>
            <CandidatePanel issues={stream.issues} onLoadRun={onLoadRun} selectedRun={selectedRun} />
          </div>
          <PrOutcomePanel prs={prs} />
        </main>
        <SidePanels rejected={rejected} selectedRun={selectedRun} stream={stream} />
      </div>
    </>
  );
}

function DryRunSurface({ agents, config, dry, error, health, history, onChangeParams, onClearRun, onRefresh, onStartDryRun, params, selectedRun, stream, watchMode }) {
  const rail = useMemo(() => buildRail(health, config, { ...stream, params }, history, selectedRun, watchMode), [health, config, stream, params, history, selectedRun, watchMode]);
  const dryIssues = normalizeIssues(dry.issues);
  const teamReady = agentsReady(agents, health);
  const scoringUnavailable = dry.done?.scoring_available === false;
  const reportUnavailable = dry.done?.analysis_available === false && !dry.report;
  const disabledReason = !teamReady
    ? "Dry Stalk still needs a Scout agent for scoring and analysis. Recruit a team in Checks; no repository writes will run here."
    : !githubReady(config, health)
      ? "Configure the PatchHive bot GitHub token before scanning candidate issues."
      : "";
  const metrics = [
    { label: "Would target", value: String(dryIssues.length || asCount(dry.done?.total_would_reap)), tone: scoringUnavailable ? "warn" : "sig", sub: scoringUnavailable ? "unscored" : "no writes" },
    { label: "Phase", value: dry.phase || "standby", tone: dry.running ? "warn" : "sig", sub: "dry stalk" },
    { label: "Report", value: dry.report ? "ready" : reportUnavailable ? "auth needed" : "none", tone: dry.report ? "ok" : reportUnavailable ? "warn" : "sig", sub: "Scout analysis" },
    { label: "Budget", value: "0", tone: "ok", sub: "no patch cost" },
    { label: "Writes", value: "0", tone: "ok", sub: "safe inspection" },
  ];
  return (
    <>
      <SuiteTopline cells={buildTopline(health, config, dry, history)} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// RepoReaper safe preview</div>
              <h1>Dry Stalk</h1>
              <p className="subline">Inspect candidate quality before patch generation, tests, branches, or pull requests are allowed.</p>
            </div>
            <div className="actions">
              <span className="chip green">no writes</span>
              {selectedRun && <button className="btn" onClick={onClearRun} type="button">Clear run</button>}
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
          </div>
          <RunForm disabled={!teamReady || !githubReady(config, health)} disabledReason={disabledReason} error={error} onChange={onChangeParams} onStart={onStartDryRun} params={params} running={dry.running} title="Run Dry Stalk" />
          <MetricBand metrics={metrics} />
          <div className="atlas-layout suite-four-layout">
            <CandidatePanel issues={dryIssues} selectedRun={null} />
            <Panel eyebrow="Scout report" title="Dry-run analysis" action={<span className={`chip ${dry.report ? "green" : reportUnavailable ? "amber" : "signal"}`}>{dry.report ? "ready" : reportUnavailable ? "auth needed" : "waiting"}</span>}>
              <div className="panelbody repo-list">
                {dry.report ? (
                  <div className="feed-item">
                    <div>
                      <div className="feed-title">Scout analysis</div>
                      <div className="feed-meta">{dry.report}</div>
                    </div>
                    <span className="chip green">safe</span>
                  </div>
                ) : (
                  <div className="empty-v2">
                    <strong>{reportUnavailable ? "Scout analysis unavailable" : "No dry-run report"}</strong>
                    <span>{reportUnavailable ? "Check the active agent provider key or switch the team to a configured local/OpenAI-compatible gateway." : "Run Dry Stalk to inspect candidate quality without making changes."}</span>
                  </div>
                )}
                {dry.logs?.slice(-4).map((log, index) => (
                  <div className="feed-item" key={`${log.msg}-${index}`}>
                    <div>
                      <div className="feed-title">{log.type || "log"}</div>
                      <div className="feed-meta">{log.msg || String(log)}</div>
                    </div>
                    <span className={`chip ${log.type === "success" ? "green" : log.type === "warn" ? "amber" : "signal"}`}>{log.type || "log"}</span>
                  </div>
                ))}
              </div>
            </Panel>
          </div>
        </main>
      </div>
    </>
  );
}

function SecondaryFrame({ children, config, health, history, selectedRun, stream, watchMode }) {
  const rail = useMemo(() => buildRail(health, config, stream, history, selectedRun, watchMode), [health, config, stream, history, selectedRun, watchMode]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, config, stream, history)} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">{children}</main>
      </div>
    </>
  );
}

function HistorySurface({ config, health, history, loading, onClearRun, onLoadRun, onRefresh, selectedRun, stream, watchMode }) {
  return (
    <SecondaryFrame config={config} health={health} history={history} selectedRun={selectedRun} stream={stream} watchMode={watchMode}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// RepoReaper patch queue</div>
          <h1>Run History</h1>
          <p className="subline">Saved autonomous runs, attempts, cost, fixes, and held work.</p>
        </div>
        <div className="actions">
          {selectedRun && <button className="btn" onClick={onClearRun} type="button">Clear run</button>}
          <button className="btn" onClick={onRefresh} type="button">{loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      <Panel eyebrow="Recent" title="Autonomous runs" action={<span className="chip signal">{history.length} saved</span>}>
        <div className="panelbody repo-list queue-grid">
          {history.length ? history.map((run) => (
            <div className="ledger-row" key={run.id}>
              <div className="rank">{selectedRun?.id === run.id ? "SEL" : asCount(run.total_fixed)}</div>
              <div>
                <div className="repo-name">{run.id}</div>
                <div className="feed-meta">{asCount(run.total_fixed)} fixed of {asCount(run.total_attempted)} attempted · {formatMoney(run.total_cost_usd)} · {timeAgo(run.started_at)}</div>
                <div className="repo-meta">
                  <span className={`chip ${statusTone(run.status)}`}>{run.status}</span>
                  <span className="chip signal">{run.attempts?.length || 0} attempts</span>
                </div>
              </div>
              <button className="btn" onClick={() => onLoadRun(run.id)} type="button">Load</button>
            </div>
          )) : (
            <div className="empty-v2">
              <strong>No runs saved</strong>
              <span>Start a hunt to create RepoReaper history.</span>
            </div>
          )}
        </div>
      </Panel>
      {selectedRun && (
        <HistoryDetailGrid>
          <CandidatePanel issues={stream.issues} onLoadRun={onLoadRun} selectedRun={selectedRun} />
          <ValidationPanel selectedRun={selectedRun} stream={stream} />
        </HistoryDetailGrid>
      )}
    </SecondaryFrame>
  );
}

function PrMonitorSurface({ config, health, history, onClearRun, onRefresh, prs, selectedRun, stream, watchMode }) {
  return (
    <SecondaryFrame config={config} health={health} history={history} selectedRun={selectedRun} stream={stream} watchMode={watchMode}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// RepoReaper outbound PRs</div>
          <h1>PR Monitor</h1>
          <p className="subline">Outbound contribution history, maintainer response, merged state, and tracked review status.</p>
        </div>
        <div className="actions">
          {selectedRun && <button className="btn" onClick={onClearRun} type="button">Clear run</button>}
          <button className="btn" onClick={onRefresh} type="button">Refresh</button>
        </div>
      </div>
      <PrOutcomePanel prs={prs} />
    </SecondaryFrame>
  );
}

function checkTone(level) {
  if (level === "error") return "red";
  if (level === "warn") return "amber";
  return "green";
}

function AgentTeamPanel({ agents, apiKey, config, onSaveAgents, saving }) {
  const [draft, setDraft] = useState(() => blankAgent(config));
  const [defaults, setDefaults] = useState(() => blankTeamDefaults(config));
  const [freeOnly, setFreeOnly] = useState(false);
  const [showProviderAdvanced, setShowProviderAdvanced] = useState(false);
  const [showManualAgent, setShowManualAgent] = useState(false);
  const team = Array.isArray(agents) ? agents : [];
  const fallbackModels = useMemo(() => config?.providers || undefined, [config?.providers]);
  const setDefault = (key, value) => setDefaults((current) => {
    if (key === "provider") {
      return { ...current, provider: value, model: PROVIDER_MODELS[value] || current.model };
    }
    return { ...current, [key]: value };
  });
  const modelDiscovery = useProviderModelDiscovery({
    apiBase: API,
    authToken: apiKey,
    provider: defaults.provider,
    model: defaults.model,
    onModelChange: (nextModel) => setDefault("model", nextModel),
    providerKey: defaults.api_key,
    baseUrl: defaults.base_url,
    fallbackModels,
    freeOnly,
    localGatewayConfigured: Boolean(config?.PATCHHIVE_AI_URL || config?.AI_LOCAL_STATUS?.ok || config?.AI_LOCAL_STATUS?.status === "ok"),
    globalKeyConfigured: Boolean(config?.PROVIDER_API_KEY_SET),
  });
  const set = (key, value) => setDraft((current) => {
    if (key === "provider") {
      return { ...current, provider: value, model: PROVIDER_MODELS[value] || current.model };
    }
    return { ...current, [key]: value };
  });
  const addAgent = () => {
    if (!draft.name.trim() || !draft.role || !draft.provider || !draft.model.trim()) return;
    onSaveAgents([...team, { ...applyTeamDefaults(draft, defaults), id: draft.id || `${draft.role}-${Date.now()}` }]);
    setDraft(blankAgent(config, draft.role));
  };
  const saveStarterTeam = () => onSaveAgents(applyTeamDefaultsToAgents(starterTeam(config), defaults));
  const applyDefaultsToTeam = () => {
    if (!team.length) return;
    onSaveAgents(applyTeamDefaultsToAgents(team, defaults));
  };
  return (
    <Panel eyebrow="Team" title="Agent team" action={<span className={`chip ${team.length ? "green" : "amber"}`}>{team.length} agents</span>}>
      <div className="panelbody repo-list">
        {!team.length && (
          <div className="empty-v2">
            <strong>No agents configured</strong>
            <span>Recruit a starter team with shared provider defaults before running Dry Stalk or full hunts.</span>
          </div>
        )}
        <div className="feed-item">
          <div>
            <div className="feed-title">Provider defaults</div>
            <div className="feed-meta">Shared provider, model, and optional credentials for the active team.</div>
          </div>
          <span className="chip signal">{PROVIDER_LABELS[defaults.provider] || defaults.provider}</span>
        </div>
        <div className="form-grid compact">
          <label className="v2-field">
            Provider
            <select className="v2-input" onChange={(event) => setDefault("provider", event.target.value)} value={defaults.provider}>
              {Object.entries(PROVIDER_LABELS).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
            </select>
          </label>
          <label className="v2-field">
            Model picker
            <select
              className="v2-input"
              disabled={modelDiscovery.loading || !modelDiscovery.models.length}
              onChange={(event) => setDefault("model", event.target.value)}
              value={modelDiscovery.models.includes(defaults.model) ? defaults.model : ""}
            >
              {!modelDiscovery.models.includes(defaults.model) && <option value="">Manual model below</option>}
              {modelDiscovery.models.map((modelId) => <option key={modelId} value={modelId}>{modelId}</option>)}
            </select>
          </label>
          <label className="v2-field">
            Base URL
            <input className="v2-input" onChange={(event) => setDefault("base_url", event.target.value)} placeholder="custom OpenAI-compatible endpoint" value={defaults.base_url} />
          </label>
          <label className="v2-field">
            Provider key
            <input className="v2-input" onChange={(event) => setDefault("api_key", event.target.value)} placeholder="saved encrypted when key is configured" type="password" value={defaults.api_key} />
          </label>
        </div>
        {showProviderAdvanced && (
          <div className="form-grid compact">
            <label className="v2-field">
              Manual model
              <input className="v2-input" disabled={modelDiscovery.loading} onChange={(event) => setDefault("model", event.target.value)} placeholder="type any model id" value={defaults.model} />
            </label>
            <label className="v2-field">
              Bot token override
              <input className="v2-input" onChange={(event) => setDefault("bot_token", event.target.value)} placeholder="optional per-team override" type="password" value={defaults.bot_token} />
            </label>
            <label className="v2-field">
              Bot user override
              <input className="v2-input" onChange={(event) => setDefault("bot_user", event.target.value)} placeholder="optional bot username" value={defaults.bot_user} />
            </label>
          </div>
        )}
        <div className="repo-meta">
          <button className="btn primary" disabled={saving || !defaults.model.trim()} onClick={saveStarterTeam} type="button">Build starter with defaults</button>
          <button className="btn" disabled={saving || !team.length || !defaults.model.trim()} onClick={applyDefaultsToTeam} type="button">Apply defaults to team</button>
          <button className="btn" disabled={saving || modelDiscovery.loading || !defaults.provider} onClick={() => modelDiscovery.loadModels({ includeProviderKey: true })} type="button">
            {modelDiscovery.loading ? "Pulling..." : "Pull models"}
          </button>
          <button className="btn" disabled={saving || modelDiscovery.testing || !defaults.provider || !defaults.model.trim()} onClick={modelDiscovery.testModel} type="button">
            {modelDiscovery.testing ? "Testing..." : "Test model"}
          </button>
          <button className="btn" onClick={() => setShowProviderAdvanced((visible) => !visible)} type="button">
            {showProviderAdvanced ? "Hide advanced" : "Advanced"}
          </button>
          <button className="btn" onClick={() => setShowManualAgent((visible) => !visible)} type="button">
            {showManualAgent ? "Hide manual agent" : "Manual agent"}
          </button>
          <label className="chip signal" style={{ cursor: "pointer", gap: 6 }}>
            <input checked={freeOnly} onChange={(event) => setFreeOnly(event.target.checked)} type="checkbox" />
            free only
          </label>
          <span className="chip green">one provider setup</span>
          <span className="chip signal">{modelDiscovery.models.length} models</span>
          {modelDiscovery.statusText && (
            <span className="feed-meta break-all" style={{ flexBasis: "100%" }}>{modelDiscovery.statusText}</span>
          )}
          {modelDiscovery.filteredStatusText && (
            <span className="feed-meta break-all" style={{ flexBasis: "100%" }}>{modelDiscovery.filteredStatusText}</span>
          )}
          {modelDiscovery.freeFilteredStatusText && (
            <span className="feed-meta break-all" style={{ flexBasis: "100%" }}>{modelDiscovery.freeFilteredStatusText}</span>
          )}
          {modelDiscovery.testStatusText && (
            <span className="feed-meta break-all" style={{ flexBasis: "100%" }}>{modelDiscovery.testStatusText}</span>
          )}
        </div>
        {team.map((agent) => (
          <div className="feed-item" key={agent.id || `${agent.role}-${agent.name}`}>
            <div>
              <div className="feed-title">{agent.name || agent.role}</div>
              <div className="feed-meta">{agent.role || "agent"} · {agent.provider || "provider"} · {agent.model || "model"}</div>
            </div>
            <button className="btn" disabled={saving} onClick={() => onSaveAgents(team.filter((item) => item.id !== agent.id))} type="button">Remove</button>
          </div>
        ))}
        {showManualAgent && (
          <>
            <div className="form-grid compact">
              <label className="v2-field">
                Name
                <input className="v2-input" onChange={(event) => set("name", event.target.value)} value={draft.name} />
              </label>
              <label className="v2-field">
                Role
                <select className="v2-input" onChange={(event) => set("role", event.target.value)} value={draft.role}>
                  {AGENT_ROLES.map((role) => <option key={role.value} value={role.value}>{role.label}</option>)}
                </select>
              </label>
              <label className="v2-field">
                Provider
                <select className="v2-input" onChange={(event) => set("provider", event.target.value)} value={draft.provider}>
                  {Object.entries(PROVIDER_LABELS).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
                </select>
              </label>
              <label className="v2-field">
                Model
                <input className="v2-input" onChange={(event) => set("model", event.target.value)} value={draft.model} />
              </label>
              {draft.provider === "custom" && (
                <label className="v2-field">
                  Base URL
                  <input className="v2-input" onChange={(event) => set("base_url", event.target.value)} placeholder="https://api.example.com/v1" value={draft.base_url} />
                </label>
              )}
              <label className="v2-field">
                Provider key
                <input className="v2-input" onChange={(event) => set("api_key", event.target.value)} placeholder="optional when global/local is configured" type="password" value={draft.api_key} />
              </label>
              <div className="v2-field">
                Action
                <button className="btn primary" disabled={saving || !draft.name.trim() || !draft.model.trim()} onClick={addAgent} type="button">Add agent</button>
              </div>
            </div>
            <div className="repo-meta">
              {AGENT_ROLES.map((role) => <span className="chip signal" key={role.value}>{role.label}: {role.detail}</span>)}
            </div>
          </>
        )}
      </div>
    </Panel>
  );
}

function ChecksSurface({ agents, apiKey, config, health, history, onClearRun, onRefresh, onSaveAgents, runtime, savingAgents, selectedRun, stream, watchMode }) {
  const checks = runtime.checks || [];
  const warnings = checks.filter((check) => check.level === "warn" || check.level === "error").length;
  const metrics = [
    { label: "Status", value: health.status || "unknown", tone: health.status === "ok" ? "ok" : "warn", sub: health.version || "backend" },
    { label: "Bot", value: githubReady(config, health) ? "ready" : "missing", tone: githubReady(config, health) ? "ok" : "warn", sub: health.bot || "identity" },
    { label: "AI", value: aiReady(config) ? "ready" : "missing", tone: aiReady(config) ? "ok" : "warn", sub: config?.PATCHHIVE_AI_URL ? "local gateway" : "provider" },
    { label: "Agents", value: String(agents.length || asCount(health.agents)), tone: agents.length || health.agents ? "ok" : "warn", sub: "team roles" },
    { label: "Checks", value: warnings ? String(warnings) : "clear", tone: warnings ? "warn" : "ok", sub: "startup" },
  ];
  return (
    <SecondaryFrame config={config} health={health} history={history} selectedRun={selectedRun} stream={stream} watchMode={watchMode}>
      <div className="hero-row">
        <div>
          <div className="eyebrow">// RepoReaper readiness</div>
          <h1>Checks</h1>
          <p className="subline">Backend health, bot identity, AI provider readiness, team setup, and startup checks.</p>
        </div>
        <div className="actions">
          {selectedRun && <button className="btn" onClick={onClearRun} type="button">Clear run</button>}
          <button className="btn" onClick={onRefresh} type="button">{runtime.loading ? "Refreshing..." : "Refresh"}</button>
        </div>
      </div>
      {runtime.error && <StatusBanner tone="red">{runtime.error}</StatusBanner>}
      <MetricBand metrics={metrics} />
      <div className="atlas-layout suite-four-layout">
        <AgentTeamPanel agents={agents} apiKey={apiKey} config={config} onSaveAgents={onSaveAgents} saving={savingAgents} />
        <Panel eyebrow="Health" title="Backend status" action={<span className={`chip ${health.status === "ok" ? "green" : "amber"}`}>{health.status || "unknown"}</span>}>
          <div className="panelbody repo-list">
            <div className="rowline"><span className="muted">Auth enabled</span><span className={`chip ${health.auth_enabled ? "green" : "amber"}`}>{health.auth_enabled ? "yes" : "no"}</span></div>
            <div className="rowline"><span className="muted">Watch mode</span><span className={`chip ${watchMode ? "green" : "amber"}`}>{watchMode ? "on" : "off"}</span></div>
            <div className="rowline"><span className="muted">Run active</span><span className={`chip ${health.run_active ? "amber" : "green"}`}>{health.run_active ? "yes" : "no"}</span></div>
            <div className="feed-item">
              <div>
                <div className="feed-title">Database</div>
                <div className="feed-meta break-all">{health.db_path || "unknown"}</div>
              </div>
              <span className={`chip ${health.db_ok ? "green" : "red"}`}>{health.db_ok ? "ok" : "check"}</span>
            </div>
          </div>
        </Panel>
        <Panel eyebrow="Startup" title="Startup checks" action={<span className={`chip ${warnings ? "amber" : "green"}`}>{warnings ? `${warnings} warnings` : "clear"}</span>}>
          <div className="panelbody repo-list">
            {checks.length ? checks.map((check, index) => (
              <div className="feed-item" key={`${check.msg}-${index}`}>
                <div>
                  <div className="feed-title">{check.level || "info"}</div>
                  <div className="feed-meta">{check.msg}</div>
                </div>
                <span className={`chip ${checkTone(check.level)}`}>{check.level || "info"}</span>
              </div>
            )) : (
              <div className="empty-v2">
                <strong>No checks</strong>
                <span>No startup checks were returned by the backend.</span>
              </div>
            )}
          </div>
        </Panel>
      </div>
    </SecondaryFrame>
  );
}

function createInitialStream() {
  return {
    agentStatuses: {},
    done: null,
    issues: [],
    logs: [],
    phase: "",
    report: "",
    runCost: 0,
    running: false,
  };
}

export default function App() {
  const [activeTab, setActiveTab] = usePersistentProductTab("repo-reaper", TABS, "mission");
  const [agents, setAgents] = useState([]);
  const [config, setConfig] = useState(null);
  const [dryParams, setDryParams] = useState(DEFAULT_DRY_PARAMS);
  const [dryStream, setDryStream] = useState(createInitialStream);
  const [error, setError] = useState("");
  const [history, setHistory] = useState([]);
  const [leaderboard, setLeaderboard] = useState([]);
  const [lifetimeCost, setLifetimeCost] = useState(0);
  const [loading, setLoading] = useState(false);
  const [params, setParams] = useState(DEFAULT_PARAMS);
  const [prs, setPrs] = useState([]);
  const [rejected, setRejected] = useState([]);
  const [savingAgents, setSavingAgents] = useState(false);
  const [selectedRun, setSelectedRun] = useState(null);
  const [stream, setStream] = useState(createInitialStream);
  const [watchMode, setWatchMode] = useState(false);
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "repo-reaper_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const ready = auth.checked && !auth.needsAuth;
  const runtime = useProductRuntime({ apiBase: API, fetcher: fetch_, ready });
  const authConfigured = Boolean(runtime.authStatus?.auth_configured || runtime.health?.auth_enabled);
  const abortRef = useRef(null);

  async function fetchJson(path, options, fallbackError) {
    const response = await fetch_(`${API}${path}`, options);
    return parseJsonResponse(response, fallbackError);
  }

  async function refreshData() {
    if (!ready) return;
    setLoading(true);
    const [historyResult, agentsResult, configResult, rejectedResult, prsResult, leaderboardResult, watchResult, costResult] = await Promise.allSettled([
      fetchJson("/history", undefined, "RepoReaper could not load history."),
      fetchJson("/agents", undefined, "RepoReaper could not load agents."),
      fetchJson("/config", undefined, "RepoReaper could not load config."),
      fetchJson("/rejected", undefined, "RepoReaper could not load rejected patches."),
      fetchJson("/pr-tracking", undefined, "RepoReaper could not load PR tracking."),
      fetchJson("/leaderboard", undefined, "RepoReaper could not load leaderboard."),
      fetchJson("/watch-mode", undefined, "RepoReaper could not load watch mode."),
      fetchJson("/stats/lifetime-cost", undefined, "RepoReaper could not load lifetime cost."),
    ]);
    const historyPayload = historyResult.status === "fulfilled" ? historyResult.value : {};
    setHistory(historyPayload.history || []);
    setAgents(agentsResult.status === "fulfilled" ? agentsResult.value?.agents || [] : []);
    setConfig(configResult.status === "fulfilled" ? configResult.value : null);
    setRejected(rejectedResult.status === "fulfilled" ? rejectedResult.value?.rejected || [] : []);
    setPrs(prsResult.status === "fulfilled" ? prsResult.value?.prs || [] : []);
    setLeaderboard(leaderboardResult.status === "fulfilled" ? leaderboardResult.value?.leaderboard || [] : []);
    setWatchMode(Boolean(watchResult.status === "fulfilled" && watchResult.value?.watch_mode));
    setLifetimeCost(costResult.status === "fulfilled" ? costResult.value?.lifetime_cost_usd || 0 : 0);
    setLoading(false);
    const failed = [historyResult, agentsResult, configResult, rejectedResult, prsResult, leaderboardResult, watchResult, costResult].find((result) => result.status === "rejected");
    if (failed) {
      setError(failed.reason?.message || "RepoReaper could not load one or more backend resources.");
    }
  }

  useEffect(() => {
    refreshData();
  }, [ready, fetch_]);

  useEffect(() => () => abortRef.current?.abort(), []);

  async function saveAgents(nextAgents) {
    setSavingAgents(true);
    setError("");
    try {
      const payload = await fetchJson(
        "/agents",
        {
          body: JSON.stringify({ agents: nextAgents }),
          headers: { "Content-Type": "application/json" },
          method: "POST",
        },
        "RepoReaper could not save the agent team.",
      );
      setAgents(payload.agents || []);
      await runtime.refresh();
    } catch (err) {
      setError(err.message || "RepoReaper could not save the agent team.");
    } finally {
      setSavingAgents(false);
    }
  }

  function applyStreamEvent(event, payload, setter) {
    setter((current) => {
      if (event === "phase") {
        return { ...current, phase: payload.phase || "" };
      }
      if (event === "agent_log" || event === "log") {
        return { ...current, logs: [...current.logs.slice(-80), payload] };
      }
      if (event === "agent_status") {
        return {
          ...current,
          agentStatuses: {
            ...current.agentStatuses,
            [payload.agent_id]: { status: payload.status, task: payload.task },
          },
        };
      }
      if (event === "issues") {
        return { ...current, issues: payload.issues || [] };
      }
      if (event === "issue_assign" || event === "issue_confidence" || event === "issue_result") {
        const issues = normalizeIssues(current.issues);
        const next = issues.map((issue) => {
          if (String(issue.id) !== String(payload.id)) return issue;
          if (event === "issue_assign") return { ...issue, status: "running", fixability_score: payload.score ?? issue.fixability_score };
          if (event === "issue_confidence") return { ...issue, confidence: payload.confidence };
          return {
            ...issue,
            confidence: payload.confidence ?? payload.pr?.confidence ?? issue.confidence,
            feedback: payload.feedback,
            pr_number: payload.pr?.number,
            pr_url: payload.pr?.url,
            reason: payload.reason,
            status: payload.status,
          };
        });
        return { ...current, issues: next };
      }
      if (event === "cost_update") {
        return { ...current, runCost: payload.run_cost || 0 };
      }
      if (event === "dry_run_report") {
        return { ...current, report: payload.report || "" };
      }
      if (event === "done") {
        return { ...current, done: payload, running: false };
      }
      if (event === "error") {
        return { ...current, logs: [...current.logs.slice(-80), { msg: payload.msg || "Run failed", type: "error" }], running: false };
      }
      return current;
    });
  }

  async function startStream(path, runParams, setter, nextTab) {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    setError("");
    setter({ ...createInitialStream(), running: true, phase: "scan" });
    setActiveTab(nextTab);
    try {
      const response = await fetch_(`${API}${path}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        signal: controller.signal,
        body: JSON.stringify(normalizeRunRequest(runParams)),
      });
      if (!response.ok || !response.body) {
        throw new Error(`RepoReaper ${path} request failed.`);
      }
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = "";
      let done = false;
      while (!done) {
        const chunk = await reader.read();
        done = chunk.done;
        if (chunk.value) {
          buffer += decoder.decode(chunk.value, { stream: true });
          const parts = buffer.split("\n\n");
          buffer = parts.pop() || "";
          parts.forEach((part) => {
            const eventMatch = part.match(/^event: (.+)$/m);
            const dataMatch = part.match(/^data: (.+)$/m);
            if (!eventMatch || !dataMatch) return;
            try {
              applyStreamEvent(eventMatch[1].trim(), JSON.parse(dataMatch[1]), setter);
            } catch (err) {
              setter((current) => ({
                ...current,
                logs: [...current.logs.slice(-80), { msg: `Skipped malformed stream event: ${err.message}`, type: "warn" }],
              }));
            }
          });
        }
      }
      setter((current) => ({ ...current, running: false }));
      await refreshData();
      await runtime.refresh();
    } catch (err) {
      if (err?.name !== "AbortError") {
        setError(err.message || "RepoReaper stream failed.");
        setter((current) => ({ ...current, running: false, logs: [...current.logs, { msg: err.message || "Run failed", type: "error" }] }));
      }
    } finally {
      if (abortRef.current === controller) abortRef.current = null;
    }
  }

  async function loadRun(id) {
    if (!id) return;
    setError("");
    try {
      const run = await fetchJson(`/history/${id}`, undefined, "RepoReaper could not load that run.");
      setSelectedRun(run);
    } catch (err) {
      setError(err.message || "RepoReaper could not load that run.");
    }
  }

  function clearRun() {
    setSelectedRun(null);
    setError("");
  }

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

  const health = runtime.health || {};

  return (
    <ProductV2Shell authConfigured={authConfigured} productKey="repo-reaper" productName="RepoReaper" runtime={runtime}>
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="RepoReaper"
        navLabel="RepoReaper navigation"
        onTabChange={setActiveTab}
        productKey="repo-reaper"
        tabs={TABS}
      />
      {activeTab === "mission" && (
        <MissionDeck
          agents={agents}
          config={config}
          error={error}
          health={health}
          history={history}
          lifetimeCost={lifetimeCost}
          onChangeParams={setParams}
          onClearRun={clearRun}
          onLoadRun={loadRun}
          onRefresh={() => {
            refreshData();
            runtime.refresh();
          }}
          onStartRun={() => startStream("/run", params, setStream, "mission")}
          params={params}
          prs={prs}
          rejected={rejected}
          selectedRun={selectedRun}
          stream={stream}
          watchMode={watchMode}
        />
      )}
      {activeTab === "dryrun" && (
        <DryRunSurface
          agents={agents}
          config={config}
          dry={dryStream}
          error={error}
          health={health}
          history={history}
          onChangeParams={setDryParams}
          onClearRun={clearRun}
          onRefresh={() => {
            refreshData();
            runtime.refresh();
          }}
          onStartDryRun={() => startStream("/dry-run", dryParams, setDryStream, "dryrun")}
          params={dryParams}
          selectedRun={selectedRun}
          stream={stream}
          watchMode={watchMode}
        />
      )}
      {activeTab === "history" && (
        <HistorySurface
          config={config}
          health={health}
          history={history}
          loading={loading}
          onClearRun={clearRun}
          onLoadRun={loadRun}
          onRefresh={refreshData}
          selectedRun={selectedRun}
          stream={stream}
          watchMode={watchMode}
        />
      )}
      {activeTab === "prs" && (
        <PrMonitorSurface
          config={config}
          health={health}
          history={history}
          onClearRun={clearRun}
          onRefresh={refreshData}
          prs={prs}
          selectedRun={selectedRun}
          stream={stream}
          watchMode={watchMode}
        />
      )}
      {activeTab === "checks" && (
        <ChecksSurface
          agents={agents}
          apiKey={auth.apiKey}
          config={config}
          health={health}
          history={history}
          onClearRun={clearRun}
          onRefresh={() => {
            refreshData();
            runtime.refresh();
          }}
          onSaveAgents={saveAgents}
          runtime={runtime}
          savingAgents={savingAgents}
          selectedRun={selectedRun}
          stream={stream}
          watchMode={watchMode}
        />
      )}
    </ProductV2Shell>
  );
}

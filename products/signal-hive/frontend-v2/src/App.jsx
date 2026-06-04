import { useCallback, useEffect, useMemo, useState } from "react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import {
  DeckBar,
  MetricBand,
  Panel,
  ProductV2Shell,
  ProductRail,
  SuiteRadar,
  SuiteTopline,
  usePersistentProductTab,
} from "@patchhivehq/ui-v2";
import { API } from "./config.js";

const TABS = [
  { id: "atlas", label: "Atlas board" },
  { id: "ledger", label: "Ops ledger" },
  { id: "floor", label: "Watch floor" },
];

const DEFAULT_PARAMS = {
  search_query: "",
  topics: "",
  languages: "rust,typescript,python",
  min_stars: "25",
  max_repos: "8",
  issues_per_repo: "30",
  stale_days: "45",
};

const CADENCE_OPTIONS = [
  { value: "6", label: "Every 6 hours" },
  { value: "12", label: "Every 12 hours" },
  { value: "24", label: "Daily" },
  { value: "72", label: "Every 3 days" },
  { value: "168", label: "Weekly" },
];

const LIST_OPTIONS = [
  { value: "allowlist", label: "Allowlist" },
  { value: "denylist", label: "Denylist" },
  { value: "opt_out", label: "Opt-out" },
];

const SORT_OPTIONS = [
  { value: "priority", label: "Priority score" },
  { value: "stale", label: "Stale issues" },
  { value: "duplicates", label: "Duplicate pressure" },
  { value: "markers", label: "TODO/FIXME markers" },
  { value: "name", label: "Repo name" },
];

function toList(value) {
  return String(value || "")
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
}

function toRequestParams(params) {
  return {
    search_query: params.search_query || "",
    topics: toList(params.topics),
    languages: toList(params.languages),
    min_stars: Number(params.min_stars) || 25,
    max_repos: Number(params.max_repos) || 8,
    issues_per_repo: Number(params.issues_per_repo) || 30,
    stale_days: Number(params.stale_days) || 45,
  };
}

function toFormParams(params = {}) {
  return {
    search_query: params.search_query || "",
    topics: (params.topics || []).join(","),
    languages: (params.languages || []).join(","),
    min_stars: String(params.min_stars ?? 25),
    max_repos: String(params.max_repos ?? 8),
    issues_per_repo: String(params.issues_per_repo ?? 30),
    stale_days: String(params.stale_days ?? 45),
  };
}

async function fetchJson(fetch_, url, opts = {}) {
  const res = await fetch_(url, opts);
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw new Error(data.error || data.message || `Request failed: ${res.status}`);
  }
  return data;
}

function formatDate(value) {
  if (!value) {
    return "none";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
}

function timeAgo(value) {
  if (!value) {
    return "never";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function scoreTone(score) {
  if (score >= 75) return "red";
  if (score >= 55) return "amber";
  if (score >= 38) return "blue";
  return "green";
}

function scoreTextTone(score) {
  if (score >= 75) return "hot";
  if (score >= 55) return "warn";
  if (score >= 38) return "signal";
  return "ok";
}

function shortRepoName(fullName = "") {
  return fullName.split("/").pop() || fullName || "repo";
}

function markerCount(repo) {
  return Number(repo.todo_count || 0) + Number(repo.fixme_count || 0);
}

function duplicateCount(repo) {
  return repo.duplicate_candidates?.length || 0;
}

function recurringCount(repo) {
  return repo.recurring_bug_clusters?.length || 0;
}

function sortRepos(repos, sortBy) {
  const next = [...repos];
  next.sort((a, b) => {
    if (sortBy === "name") {
      return a.full_name.localeCompare(b.full_name);
    }
    if (sortBy === "stale") {
      return Number(b.stale_issues || 0) - Number(a.stale_issues || 0);
    }
    if (sortBy === "duplicates") {
      return duplicateCount(b) - duplicateCount(a);
    }
    if (sortBy === "markers") {
      return markerCount(b) - markerCount(a);
    }
    return Number(b.priority_score || 0) - Number(a.priority_score || 0);
  });
  return next;
}

function radarPosition(index, total, score) {
  const angle = -76 + index * 137.5;
  const radians = (angle * Math.PI) / 180;
  const scoreRadius = 18 + Math.max(0, Math.min(100, score)) * 0.27;
  const crowdOffset = total > 8 ? (index % 3) * 3 : 0;
  const radius = Math.min(43, scoreRadius + crowdOffset);
  return {
    left: `${Math.round((50 + Math.cos(radians) * radius) * 10) / 10}%`,
    top: `${Math.round((50 + Math.sin(radians) * radius) * 10) / 10}%`,
  };
}

function buildRadarItems(scan) {
  const repos = sortRepos(scan?.repos || [], "priority");
  const total = repos.length || 1;
  return repos.map((repo, index) => {
    const score = Math.round(Number(repo.priority_score || 0));
    const trend = repo.trend?.priority_delta;
    const trendLabel = typeof trend === "number" ? `${trend > 0 ? "+" : ""}${Math.round(trend)}` : "new";
    return {
      id: repo.full_name,
      label: `${shortRepoName(repo.full_name)} ${score}`,
      tone: scoreTone(score),
      minWindow: index < 5 ? 7 : index < 8 ? 14 : 30,
      position: radarPosition(index, total, score),
      pingDelay: `${index * 0.22}s`,
      title: repo.full_name,
      detail: repo.full_name,
      vector: String(Math.round((index * 47 + score * 2) % 360)).padStart(3, "0"),
      vectorTone: scoreTextTone(score),
      gain: `${score}%`,
      gainTone: scoreTextTone(score),
      gainMeta: "priority score",
      value: `${score}`,
      stats: [
        { label: "Score", value: score },
        { label: "Stale", value: `${repo.stale_issues || 0}/${repo.sampled_issues || 0}` },
        { label: "Dupes", value: duplicateCount(repo) },
        { label: "Markers", value: markerCount(repo) },
        { label: "Trend", value: trendLabel },
      ],
      summary: repo.summary || repo.signals?.[0] || "No major maintenance signals found.",
    };
  });
}

function buildRadarWindows(scan) {
  const repos = scan?.repos || [];
  const count = (days) => buildRadarItems({ repos }).filter((item) => item.minWindow <= days).length;
  return {
    7: { label: "7 day live pass", count: `${count(7)} signals`, outer: "7d", mid: "3d", inner: "24h" },
    14: { label: "14 day history pass", count: `${count(14)} signals`, outer: "14d", mid: "7d", inner: "3d" },
    30: { label: "30 day deep sweep", count: `${count(30)} signals`, outer: "30d", mid: "14d", inner: "7d" },
  };
}

function scanMetrics(scan) {
  const repos = scan?.repos || [];
  const stale = repos.reduce((sum, repo) => sum + Number(repo.stale_issues || 0), 0);
  const dupes = repos.reduce((sum, repo) => sum + duplicateCount(repo), 0);
  const recurring = repos.reduce((sum, repo) => sum + recurringCount(repo), 0);
  return [
    { label: "Repos scanned", value: String(scan?.summary?.total_repos || 0), tone: "sig", sub: scan ? "complete" : "waiting" },
    { label: "Signals found", value: String(scan?.summary?.total_signals || 0), tone: "warn", sub: scan?.trend ? `${scan.trend.total_signals_delta > 0 ? "+" : ""}${scan.trend.total_signals_delta} vs prior` : "latest scan" },
    { label: "Stale issues", value: String(stale), tone: "hot", sub: `${scan?.params?.stale_days || 45}+ days` },
    { label: "Duplicate pairs", value: String(dupes), tone: "", sub: "likely matches" },
    { label: "Recurring bugs", value: String(recurring), tone: "ok", sub: "cluster leads" },
  ];
}

function buildReportSummary(scan) {
  if (!scan) return "";
  const lines = [
    `SignalHive scan ${scan.id}`,
    `Top repo: ${scan.summary?.top_repo || "none"}`,
    `Repos scanned: ${scan.summary?.total_repos || 0}`,
    `Signals found: ${scan.summary?.total_signals || 0}`,
    "",
    "Ranked queue:",
    ...sortRepos(scan.repos || [], "priority").map((repo, index) => (
      `${index + 1}. ${repo.full_name} - ${Math.round(repo.priority_score || 0)} - ${repo.summary || "No summary"}`
    )),
  ];
  return lines.join("\n");
}

function downloadTextFile(filename, text, type = "text/plain;charset=utf-8") {
  const blob = new Blob([text || ""], { type });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

function Field({ label, children }) {
  return (
    <label className="v2-field">
      <span>{label}</span>
      {children}
    </label>
  );
}

function TextInput({ value, onChange, type = "text", placeholder = "" }) {
  return (
    <input
      className="v2-input"
      onChange={(event) => onChange(event.target.value)}
      placeholder={placeholder}
      type={type}
      value={value}
    />
  );
}

function SelectInput({ value, onChange, options }) {
  return (
    <select className="v2-input" onChange={(event) => onChange(event.target.value)} value={value}>
      {options.map((option) => (
        <option key={option.value} value={option.value}>{option.label}</option>
      ))}
    </select>
  );
}

function EmptyV2({ title, body }) {
  return (
    <div className="empty-v2">
      <span className="micro">// Empty</span>
      <strong>{title}</strong>
      {body && <span>{body}</span>}
    </div>
  );
}

function StatusBanner({ tone = "signal", children }) {
  if (!children) return null;
  return <div className={`status-banner ${tone}`}>{children}</div>;
}

function AuthScreen({
  authError,
  bootstrapRequired,
  checked,
  generateKey,
  login,
}) {
  const [key, setKey] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [generatedKey, setGeneratedKey] = useState("");
  const [copiedKey, setCopiedKey] = useState(false);

  const submit = async (event) => {
    event.preventDefault();
    if (!key.trim()) return;
    setBusy(true);
    setError("");
    try {
      const res = await fetch(`${API}/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ api_key: key.trim() }),
      });
      if (!res.ok) {
        throw new Error("Invalid API key.");
      }
      login(key.trim());
    } catch (err) {
      setError(err.message || "Cannot reach SignalHive.");
    } finally {
      setBusy(false);
    }
  };

  const generate = async () => {
    setBusy(true);
    setError("");
    try {
      const nextKey = await generateKey({ autoLogin: false });
      setGeneratedKey(nextKey);
      setKey(nextKey);
      setCopiedKey(false);
    } catch (err) {
      setError(err.message || "Could not generate an API key.");
    } finally {
      setBusy(false);
    }
  };

  const copyGeneratedKey = async () => {
    if (!generatedKey) return;
    try {
      await navigator.clipboard.writeText(generatedKey);
      setCopiedKey(true);
    } catch {
      setError("Could not copy generated API key.");
    }
  };

  if (!checked) {
    return (
      <div className="auth-shell">
        <div className="auth-card">
          <span className="micro">// SignalHive</span>
          <div className="auth-title">Checking session</div>
          <div className="auth-meter" />
        </div>
      </div>
    );
  }

  return (
    <div className="auth-shell">
      <form className="auth-card" onSubmit={submit}>
        <span className="micro">// Operator access</span>
        <div className="auth-title">SignalHive</div>
        <p className="auth-copy">
          {bootstrapRequired ? "Generate the first local API key or enter an existing one." : "Enter the local SignalHive API key."}
        </p>
        <Field label="API endpoint">
          <input className="v2-input" readOnly value={API} />
        </Field>
        <Field label="API key">
          <TextInput onChange={setKey} placeholder="sh-..." type="password" value={key} />
        </Field>
        {(authError || error) && <StatusBanner tone="red">{error || authError}</StatusBanner>}
        {generatedKey && (
          <StatusBanner tone="green">
            Generated key for this browser session. Copy it now, then press Enter: <span className="break-all">{generatedKey}</span>
          </StatusBanner>
        )}
        {copiedKey && <StatusBanner tone="signal">API key copied.</StatusBanner>}
        <button className="btn primary" disabled={busy || !key.trim()} type="submit">
          {busy ? "Authenticating" : "Enter"}
        </button>
        {generatedKey && (
          <button className="btn" disabled={busy} onClick={copyGeneratedKey} type="button">
            Copy generated key
          </button>
        )}
        {bootstrapRequired && (
          <button className="btn" disabled={busy} onClick={generate} type="button">
            {busy ? "Generating" : "Generate local API key"}
          </button>
        )}
      </form>
    </div>
  );
}

function RepoCard({ repo }) {
  const score = Math.round(Number(repo.priority_score || 0));
  const factors = repo.score_breakdown?.slice(0, 3) || [];
  return (
    <article className="repo-card">
      <div className="repo-head">
        <div>
          <div className="repo-name">{repo.full_name}</div>
          <div className="repo-meta">
            {repo.language && <span className="chip signal">{repo.language}</span>}
            <span className={`chip ${scoreTextTone(score)}`}>{scoreTone(score)}</span>
            <span className="chip">{repo.open_issues || 0} issues</span>
          </div>
        </div>
        <div className={`score ${scoreTextTone(score)}`}>{score}</div>
      </div>
      <p className="muted">{repo.summary || "No major maintenance signals found."}</p>
      {factors.length > 0 && (
        <div className="bargrid">
          {factors.map((factor) => {
            const impact = Math.max(0, Math.min(100, Number(factor.impact || 0) * 3));
            const tone = Number(factor.impact || 0) >= 15 ? "red" : Number(factor.impact || 0) >= 8 ? "amber" : "green";
            return (
              <div className="driver" key={factor.key || factor.label}>
                <span>{factor.label}</span>
                <div className="bar"><span className={tone} style={{ width: `${impact}%` }} /></div>
                <span className={tone === "red" ? "hot" : "warn"}>{Math.round(factor.impact || 0)}</span>
              </div>
            );
          })}
        </div>
      )}
      {repo.signals?.length > 0 && (
        <div className="signal-list">
          {repo.signals.slice(0, 3).map((signal) => <span key={signal}>{signal}</span>)}
        </div>
      )}
      <div className="repo-meta">
        <span className="chip">{repo.stale_issues || 0} stale</span>
        <span className="chip">{duplicateCount(repo)} duplicate pairs</span>
        <span className="chip">{recurringCount(repo)} clusters</span>
        <span className="chip">{markerCount(repo)} markers</span>
        {repo.repo_url && (
          <a className="chip signal" href={repo.repo_url} rel="noreferrer" target="_blank">GitHub</a>
        )}
      </div>
    </article>
  );
}

function RadarScope({ scan }) {
  const items = useMemo(() => buildRadarItems(scan), [scan]);
  const windows = useMemo(() => buildRadarWindows(scan), [scan]);
  const feed = useMemo(() => {
    const warnings = (scan?.warnings || []).map((text) => ({ text, tone: "amber" }));
    const signals = sortRepos(scan?.repos || [], "priority")
      .flatMap((repo) => (repo.signals || []).slice(0, 1).map((text) => ({
        text: `${repo.full_name}: ${text}`,
        tone: scoreTone(repo.priority_score || 0),
      })));
    return [...warnings, ...signals].slice(0, 3);
  }, [scan]);

  if (!scan || items.length === 0) {
    return (
      <div className="signal-map radar-empty-shell">
        <div className="radar-frame">
          <EmptyV2
            title="No scan loaded"
            body="Run a sweep or open a saved scan to populate the radar with real repository signals."
          />
        </div>
      </div>
    );
  }

  return (
    <SuiteRadar
      ariaLabel="Live maintenance signal radar"
      detailLabel="Selected repo scan"
      echoes={[]}
      feed={feed}
      gainLabel="Priority gain"
      itemQueryParam="repo"
      items={items}
      signalLabel="signals"
      vectorLabel="Sweep vector"
      windows={windows}
    />
  );
}

function QueuePanel({ onExportReport, scan, sortBy, setSortBy }) {
  const repos = useMemo(() => sortRepos(scan?.repos || [], sortBy), [scan, sortBy]);
  return (
    <Panel
      eyebrow="Queue"
      title="Ranked targets"
      action={
        <div className="actions">
          <SelectInput
            onChange={setSortBy}
            options={SORT_OPTIONS}
            value={sortBy}
          />
          <button className="btn" disabled={!scan} onClick={onExportReport} type="button">Export</button>
        </div>
      }
    >
      <div className="panelbody repo-list">
        {repos.length === 0 ? (
          <EmptyV2 title="No ranked targets" body="SignalHive will fill this queue after a scan finishes." />
        ) : (
          repos.map((repo) => <RepoCard key={repo.full_name} repo={repo} />)
        )}
      </div>
    </Panel>
  );
}

function ScanControlPanel({
  actionMessage,
  error,
  onClearPreset,
  onClearSchedule,
  onDeletePreset,
  onDeleteSchedule,
  onLoadPreset,
  onLoadSchedule,
  onRefresh,
  onRun,
  onRunSchedule,
  onSavePreset,
  onSaveSchedule,
  params,
  presetName,
  presets,
  running,
  scheduleCadence,
  scheduleEnabled,
  scheduleName,
  schedules,
  selectedPresetName,
  selectedScheduleName,
  setParams,
  setPresetName,
  setScheduleCadence,
  setScheduleEnabled,
  setScheduleName,
  setSelectedPresetName,
  setSelectedScheduleName,
}) {
  const set = (key, value) => setParams((prev) => ({ ...prev, [key]: value }));
  return (
    <Panel eyebrow="Controls" title="Scan control">
      <div className="panelbody control-stack">
        <div className="form-grid">
          <Field label="Search query">
            <TextInput onChange={(value) => set("search_query", value)} placeholder="bug triage, maintenance" value={params.search_query} />
          </Field>
          <Field label="Topics">
            <TextInput onChange={(value) => set("topics", value)} placeholder="payments, api" value={params.topics} />
          </Field>
          <Field label="Languages">
            <TextInput onChange={(value) => set("languages", value)} placeholder="rust,typescript,python" value={params.languages} />
          </Field>
          <Field label="Min stars">
            <TextInput onChange={(value) => set("min_stars", value)} type="number" value={params.min_stars} />
          </Field>
          <Field label="Max repos">
            <TextInput onChange={(value) => set("max_repos", value)} type="number" value={params.max_repos} />
          </Field>
          <Field label="Issues / repo">
            <TextInput onChange={(value) => set("issues_per_repo", value)} type="number" value={params.issues_per_repo} />
          </Field>
          <Field label="Stale days">
            <TextInput onChange={(value) => set("stale_days", value)} type="number" value={params.stale_days} />
          </Field>
        </div>

        <div className="actions split-actions">
          <span className="micro">Read-only GitHub issue, repo, and marker scan</span>
          <button className="btn primary" disabled={running} onClick={onRun} type="button">
            {running ? "Scanning" : "Run sweep"}
          </button>
        </div>
        {error && <StatusBanner tone="red">{error}</StatusBanner>}
        {actionMessage && <StatusBanner tone={actionMessage.tone}>{actionMessage.text}</StatusBanner>}

        <div className="ops-grid">
          <div className="mini-panel">
            <div className="mini-head">
              <span className="label">Presets</span>
              <span className="chip signal">{presets.length} saved</span>
            </div>
            <Field label="Saved preset">
              <SelectInput
                onChange={setSelectedPresetName}
                options={presets.length ? presets.map((preset) => ({ value: preset.name, label: preset.name })) : [{ value: "", label: "No saved presets" }]}
                value={selectedPresetName}
              />
            </Field>
            <div className="actions">
              <button className="btn" disabled={!selectedPresetName} onClick={onLoadPreset} type="button">Load</button>
              <button className="btn" onClick={onClearPreset} type="button">Clear preset</button>
              <button className="btn" disabled={!selectedPresetName} onClick={onDeletePreset} type="button">Delete</button>
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
            <Field label="Save current config">
              <TextInput onChange={setPresetName} placeholder="nightly-rust" value={presetName} />
            </Field>
            <button className="btn primary" disabled={!presetName.trim()} onClick={onSavePreset} type="button">Save preset</button>
          </div>

          <div className="mini-panel">
            <div className="mini-head">
              <span className="label">Schedules</span>
              <span className="chip amber">{schedules.filter((schedule) => schedule.enabled).length} active</span>
            </div>
            <Field label="Saved schedule">
              <SelectInput
                onChange={setSelectedScheduleName}
                options={schedules.length ? schedules.map((schedule) => ({ value: schedule.name, label: schedule.name })) : [{ value: "", label: "No saved schedules" }]}
                value={selectedScheduleName}
              />
            </Field>
            <div className="actions">
              <button className="btn" disabled={!selectedScheduleName} onClick={onLoadSchedule} type="button">Load</button>
              <button className="btn" onClick={onClearSchedule} type="button">Clear schedule</button>
              <button className="btn" disabled={!selectedScheduleName || running} onClick={onRunSchedule} type="button">Run now</button>
              <button className="btn" disabled={!selectedScheduleName} onClick={onDeleteSchedule} type="button">Delete</button>
            </div>
            <div className="form-grid compact">
              <Field label="Name">
                <TextInput onChange={setScheduleName} placeholder="daily-rust-q" value={scheduleName} />
              </Field>
              <Field label="Cadence">
                <SelectInput onChange={setScheduleCadence} options={CADENCE_OPTIONS} value={scheduleCadence} />
              </Field>
              <Field label="State">
                <SelectInput
                  onChange={setScheduleEnabled}
                  options={[{ value: "true", label: "Enabled" }, { value: "false", label: "Paused" }]}
                  value={scheduleEnabled}
                />
              </Field>
            </div>
            <button className="btn primary" disabled={!scheduleName.trim()} onClick={onSaveSchedule} type="button">Save schedule</button>
          </div>
        </div>
      </div>
    </Panel>
  );
}

function SidePanels({ checks, health, repoLists, scan }) {
  const evidence = useMemo(() => {
    const warnings = (scan?.warnings || []).map((warning) => ({ title: warning, meta: "scan warning", tone: "amber", label: "warn" }));
    const signals = sortRepos(scan?.repos || [], "priority")
      .flatMap((repo) => (repo.signals || []).slice(0, 1).map((signal) => ({
        title: signal,
        meta: repo.full_name,
        tone: scoreTextTone(repo.priority_score || 0),
        label: Math.round(repo.priority_score || 0),
      })));
    return [...warnings, ...signals].slice(0, 3);
  }, [scan]);

  const grouped = groupRepoLists(repoLists);
  const blockingChecks = checks.filter((check) => check.level === "error" || check.level === "warn").slice(0, 3);
  return (
    <aside className="side">
      <Panel eyebrow="Evidence" title="Why it matters">
        <div className="panelbody repo-list">
          {evidence.length === 0 ? (
            <EmptyV2 title="No evidence yet" body="Run or open a scan to populate evidence." />
          ) : (
            evidence.map((item) => (
              <div className="feed-item" key={`${item.meta}-${item.title}`}>
                <div>
                  <div className="feed-title">{item.title}</div>
                  <div className="feed-meta">{item.meta}</div>
                </div>
                <span className={`chip ${item.tone}`}>{item.label}</span>
              </div>
            ))
          )}
        </div>
      </Panel>

      <Panel eyebrow="Guardrails" title="Scope control">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">Allowlist</span><span className="chip green">{grouped.allowlist.length} targets</span></div>
          <div className="rowline"><span className="muted">Denylist</span><span className="chip red">{grouped.denylist.length} targets</span></div>
          <div className="rowline"><span className="muted">Opt-out</span><span className="chip">{grouped.opt_out.length} targets</span></div>
        </div>
      </Panel>

      <Panel eyebrow="Checks" title="Startup signal">
        <div className="panelbody repo-list">
          <div className="rowline"><span className="muted">Backend</span><span className={`chip ${health?.status === "ok" ? "green" : "amber"}`}>{health?.status || "unknown"}</span></div>
          {blockingChecks.length === 0 ? (
            <div className="rowline"><span className="muted">Startup checks</span><span className="chip green">clear</span></div>
          ) : (
            blockingChecks.map((check, index) => (
              <div className="feed-item" key={`${check.msg}-${index}`}>
                <div className="feed-title">{check.msg}</div>
                <span className={`chip ${check.level === "error" ? "red" : "amber"}`}>{check.level}</span>
              </div>
            ))
          )}
        </div>
      </Panel>
    </aside>
  );
}

function groupRepoLists(repoLists) {
  return {
    allowlist: repoLists.filter((item) => item.list_type === "allowlist"),
    denylist: repoLists.filter((item) => item.list_type === "denylist"),
    opt_out: repoLists.filter((item) => item.list_type === "opt_out"),
  };
}

function buildTopline(health, scan, schedules, authConfigured) {
  return [
    { label: "SignalHive", value: "Scout array 01", tone: "sig" },
    { label: "System", value: health?.status || "unknown", tone: health?.status === "ok" ? "ok" : "warn" },
    { label: "Mode", value: health?.read_only === false ? "write" : "read only" },
    { label: "Auth", value: authConfigured ? "configured" : "open", tone: authConfigured ? "sig" : "warn" },
    { label: "Schedules", value: `${schedules.filter((schedule) => schedule.enabled).length} active`, tone: "warn" },
    { label: "Last sweep", value: scan?.created_at ? timeAgo(scan.created_at) : "none" },
  ];
}

function buildRail(presets, schedules, scan) {
  const presetItems = presets.slice(0, 4).map((preset, index) => ({
    label: preset.name,
    active: index === 0,
    value: preset.params?.max_repos || "",
  }));
  const scheduleItems = schedules.slice(0, 4).map((schedule, index) => ({
    label: schedule.name,
    active: index === 0,
    badge: schedule.enabled ? "on" : "hold",
    badgeTone: schedule.enabled ? "green" : "amber",
  }));
  return {
    sections: [
      {
        title: "Presets",
        items: presetItems.length ? presetItems : [{ label: "no presets", value: "0" }],
      },
      {
        title: "Schedules",
        items: scheduleItems.length ? scheduleItems : [{ label: "no schedules", badge: "off" }],
      },
    ],
    stats: {
      title: "Last sweep",
      items: [
        { label: "Top target", value: scan?.summary?.top_repo || "none" },
        { label: "Signals", value: scan?.summary?.total_signals || "0", large: true, tone: "warn" },
        { label: "Saved scans", value: String(scan ? 1 : 0) },
      ],
    },
  };
}

function buildLedgerRail(history, scan, timeline) {
  return {
    sections: [
      {
        title: "Scan history",
        items: history.slice(0, 4).map((item) => ({
          active: item.id === scan?.id,
          badge: String(item.total_signals || 0),
          badgeTone: item.warning_count > 0 ? "amber" : "signal",
          label: item.top_repo || "scan",
          value: item.trigger_type || "manual",
        })),
      },
      {
        title: "Selected sweep",
        items: [
          { label: "repos", badge: String(scan?.summary?.total_repos || 0), badgeTone: "signal" },
          { label: "signals", badge: String(scan?.summary?.total_signals || 0), badgeTone: "amber" },
          { label: "timeline", badge: String(timeline?.points?.length || 0), badgeTone: "signal" },
        ],
      },
    ],
    stats: {
      title: "Ledger",
      items: [
        { label: "Saved scans", value: String(history.length) },
        { label: "Selected", value: scan?.summary?.top_repo || "none" },
        { label: "Signals", value: String(scan?.summary?.total_signals || 0), large: true, tone: "warn" },
      ],
    },
  };
}

function AtlasBoard({
  actionMessage,
  authConfigured,
  checks,
  error,
  health,
  onClearPreset,
  onClearScan,
  onClearSchedule,
  onDeletePreset,
  onDeleteSchedule,
  onExportReport,
  onLoadPreset,
  onLoadSchedule,
  onRefresh,
  onRun,
  onRunSchedule,
  onSavePreset,
  onSaveSchedule,
  params,
  presetName,
  presets,
  repoLists,
  running,
  scan,
  scheduleCadence,
  scheduleEnabled,
  scheduleName,
  schedules,
  selectedPresetName,
  selectedScheduleName,
  setParams,
  setPresetName,
  setScheduleCadence,
  setScheduleEnabled,
  setScheduleName,
  setSelectedPresetName,
  setSelectedScheduleName,
  sortBy,
  setSortBy,
}) {
  const rail = buildRail(presets, schedules, scan);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, scan, schedules, authConfigured)} />
      <div className="main-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - scan array</div>
              <h1>Signal Atlas</h1>
              <p className="subline">Live GitHub reconnaissance with scope controls, scan history, schedules, and exportable evidence.</p>
            </div>
            <div className="actions">
              <span className="chip signal">{params.max_repos} repos max</span>
              <span className="chip">{toList(params.languages).join(" - ") || "allowlist"}</span>
              {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
              <button className="btn primary" disabled={running} onClick={onRun} type="button">
                {running ? "Scanning" : "Run sweep"}
              </button>
            </div>
          </div>
          <MetricBand metrics={scanMetrics(scan)} />
          <ScanControlPanel
            actionMessage={actionMessage}
            error={error}
            onClearPreset={onClearPreset}
            onClearSchedule={onClearSchedule}
            onDeletePreset={onDeletePreset}
            onDeleteSchedule={onDeleteSchedule}
            onLoadPreset={onLoadPreset}
            onLoadSchedule={onLoadSchedule}
            onRefresh={onRefresh}
            onRun={onRun}
            onRunSchedule={onRunSchedule}
            onSavePreset={onSavePreset}
            onSaveSchedule={onSaveSchedule}
            params={params}
            presetName={presetName}
            presets={presets}
            running={running}
            scheduleCadence={scheduleCadence}
            scheduleEnabled={scheduleEnabled}
            scheduleName={scheduleName}
            schedules={schedules}
            selectedPresetName={selectedPresetName}
            selectedScheduleName={selectedScheduleName}
            setParams={setParams}
            setPresetName={setPresetName}
            setScheduleCadence={setScheduleCadence}
            setScheduleEnabled={setScheduleEnabled}
            setScheduleName={setScheduleName}
            setSelectedPresetName={setSelectedPresetName}
            setSelectedScheduleName={setSelectedScheduleName}
          />
          <div className="atlas-layout suite-four-layout">
            <Panel
              eyebrow="Cartography"
              title="Field intensity map"
              action={<span className="chip signal">{scan ? "live radar" : "idle radar"}</span>}
            >
              <RadarScope scan={scan} />
            </Panel>
            <QueuePanel onExportReport={onExportReport} scan={scan} sortBy={sortBy} setSortBy={setSortBy} />
          </div>
        </main>
        <SidePanels checks={checks} health={health} repoLists={repoLists} scan={scan} />
      </div>
    </>
  );
}

function LedgerBoard({
  authConfigured,
  health,
  history,
  loadingScan,
  onClearScan,
  onCopySummary,
  onExportReport,
  onLoadScan,
  onRefresh,
  schedules,
  scan,
  sortBy,
  setSortBy,
  timeline,
}) {
  const repos = useMemo(() => sortRepos(scan?.repos || [], sortBy), [scan, sortBy]);
  const rail = useMemo(() => buildLedgerRail(history, scan, timeline), [history, scan, timeline]);
  return (
    <>
      <SuiteTopline cells={buildTopline(health, scan, schedules, authConfigured)} />
      <div className="main-grid hive-workspace-grid">
        <ProductRail sections={rail.sections} stats={rail.stats} />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - scan ledger</div>
              <h1>Ops Ledger</h1>
              <p className="subline">Saved scan history, selected sweep evidence, trend snapshots, and exportable reports.</p>
            </div>
            <div className="actions">
              {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
            </div>
          </div>
          <Panel eyebrow="History" title="Scan ledger" action={<span className="chip signal">{history.length} saved</span>}>
            <div className="panelbody repo-list">
              {history.length === 0 ? (
                <EmptyV2 title="No scans saved" body="Run a sweep from the atlas board to create history." />
              ) : (
                history.map((item) => (
                  <button
                    className={`history-row${scan?.id === item.id ? " active" : ""}`}
                    key={item.id}
                    onClick={() => onLoadScan(item.id)}
                    type="button"
                  >
                    <span>
                      <strong>{item.top_repo || "No top repo"}</strong>
                      <small>{item.total_repos} repos - {item.total_signals} signals - {timeAgo(item.created_at)}</small>
                    </span>
                    <span className={`chip ${item.warning_count > 0 ? "amber" : "signal"}`}>{item.trigger_type || "manual"}</span>
                  </button>
                ))
              )}
            </div>
          </Panel>
          {loadingScan && <Panel eyebrow="Loading" title="Opening scan"><div className="panelbody"><EmptyV2 title="Loading scan detail" /></div></Panel>}
          {!loadingScan && !scan && (
            <Panel eyebrow="Selected" title="Scan detail"><div className="panelbody"><EmptyV2 title="No scan selected" /></div></Panel>
          )}
          {!loadingScan && scan && (
            <>
              <Panel
                eyebrow="Report"
                title={scan.summary?.top_repo || "Saved scan"}
                action={
                  <div className="actions">
                    <button className="btn" onClick={onCopySummary} type="button">Copy summary</button>
                    <button className="btn primary" onClick={onExportReport} type="button">Export report</button>
                  </div>
                }
              >
                <div className="panelbody report-grid">
                  <div><span className="label">Scan ID</span><strong>{scan.id}</strong></div>
                  <div><span className="label">Created</span><strong>{formatDate(scan.created_at)}</strong></div>
                  <div><span className="label">Repos</span><strong>{scan.summary?.total_repos || 0}</strong></div>
                  <div><span className="label">Signals</span><strong>{scan.summary?.total_signals || 0}</strong></div>
                  <div><span className="label">Warnings</span><strong>{scan.warnings?.length || 0}</strong></div>
                  <div><span className="label">Timeline</span><strong>{timeline?.points?.length || 0}</strong></div>
                </div>
              </Panel>
              {scan.trend && (
                <Panel eyebrow="Trend" title="Previous similar scan">
                  <div className="panelbody chip-row">
                    <span className="chip amber">signals {scan.trend.total_signals_delta > 0 ? "+" : ""}{scan.trend.total_signals_delta}</span>
                    <span className="chip">repos {scan.trend.total_repos_delta > 0 ? "+" : ""}{scan.trend.total_repos_delta}</span>
                    <span className="chip signal">{scan.trend.rising_repos} rising</span>
                    <span className="chip green">{scan.trend.improving_repos} improving</span>
                    <span className="chip">{scan.trend.steady_repos} steady</span>
                  </div>
                </Panel>
              )}
              <Panel
                eyebrow="Queue"
                title="Saved ranked targets"
                action={<SelectInput onChange={setSortBy} options={SORT_OPTIONS} value={sortBy} />}
              >
                <div className="panelbody repo-list">
                  {repos.map((repo) => <RepoCard key={repo.full_name} repo={repo} />)}
                </div>
              </Panel>
            </>
          )}
        </main>
      </div>
    </>
  );
}

function WatchFloor({
  actionMessage,
  authConfigured,
  authStatus,
  checks,
  error,
  generatedServiceToken,
  health,
  onAddRepoControl,
  onClearScan,
  onCopyServiceToken,
  onGenerateServiceToken,
  onRefresh,
  onRemoveRepoControl,
  onRotateServiceToken,
  onSignOut,
  repoControl,
  repoLists,
  schedules,
  scan,
  serviceTokenBusy,
  setRepoControl,
}) {
  const grouped = groupRepoLists(repoLists);
  const serviceToken = authStatus?.service_auth_token;
  const serviceScopes = authStatus?.service_auth_scopes || serviceToken?.scopes || [];
  return (
    <>
      <SuiteTopline cells={buildTopline(health, scan, schedules, authConfigured)} />
      <div className="main-grid focus-grid">
        <ProductRail
          sections={[
            { title: "Control lists", items: LIST_OPTIONS.map((item) => ({ label: item.label, value: grouped[item.value].length })) },
            { title: "Startup", items: checks.slice(0, 4).map((check) => ({ label: check.level, value: check.msg.slice(0, 12) })) },
          ]}
          stats={{
            title: "Backend",
            items: [
              { label: "Status", value: health?.status || "unknown", large: true, tone: health?.status === "ok" ? "ok" : "warn" },
              { label: "Version", value: health?.version || "unknown" },
              { label: "Auth", value: authConfigured ? "configured" : "open" },
            ],
          }}
        />
        <main className="workspace">
          <div className="hero-row">
            <div>
              <div className="eyebrow">// Module - watch floor</div>
              <h1>Signal Controls</h1>
              <p className="subline">Operator auth, startup checks, and repo discovery boundaries for the read-only scan layer.</p>
            </div>
            <div className="actions">
              {scan && <button className="btn" onClick={onClearScan} type="button">Clear scan</button>}
              <button className="btn" onClick={onRefresh} type="button">Refresh</button>
              {authConfigured && <button className="btn" onClick={onSignOut} type="button">Sign out</button>}
            </div>
          </div>
          {error && <StatusBanner tone="red">{error}</StatusBanner>}
          {actionMessage && <StatusBanner tone={actionMessage.tone}>{actionMessage.text}</StatusBanner>}
          <div className="atlas-layout">
            <Panel eyebrow="Discovery" title="Repo controls">
              <div className="panelbody control-stack">
                <div className="form-grid compact">
                  <Field label="Repo">
                    <TextInput
                      onChange={(value) => setRepoControl((prev) => ({ ...prev, repo: value }))}
                      placeholder="owner/repo"
                      value={repoControl.repo}
                    />
                  </Field>
                  <Field label="List">
                    <SelectInput
                      onChange={(value) => setRepoControl((prev) => ({ ...prev, list_type: value }))}
                      options={LIST_OPTIONS}
                      value={repoControl.list_type}
                    />
                  </Field>
                </div>
                <button className="btn primary" disabled={!repoControl.repo.trim()} onClick={onAddRepoControl} type="button">Add repo control</button>
                {Object.entries(grouped).map(([key, items]) => (
                  <div className="mini-panel" key={key}>
                    <div className="mini-head">
                      <span className="label">{LIST_OPTIONS.find((item) => item.value === key)?.label || key}</span>
                      <span className="chip">{items.length} repos</span>
                    </div>
                    <div className="repo-list">
                      {items.length === 0 ? (
                        <EmptyV2 title="No entries" />
                      ) : (
                        items.map((item) => (
                          <div className="rowline control-row" key={`${item.list_type}-${item.repo}`}>
                            <span>
                              <strong>{item.repo}</strong>
                              <small>Added {formatDate(item.added_at)}</small>
                            </span>
                            <button className="btn" onClick={() => onRemoveRepoControl(item.repo)} type="button">Remove</button>
                          </div>
                        ))
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </Panel>

            <Panel eyebrow="Startup" title="Checks and auth">
              <div className="panelbody control-stack">
                <div className="report-grid">
                  <div><span className="label">Status</span><strong>{health?.status || "unknown"}</strong></div>
                  <div><span className="label">DB</span><strong>{health?.db_ok ? "ok" : "check"}</strong></div>
                  <div><span className="label">Path</span><strong>{health?.db_path || "unknown"}</strong></div>
                  <div><span className="label">Service auth</span><strong>{authStatus?.service_auth_configured ? "paired" : "not paired"}</strong></div>
                </div>
                <div className="mini-panel">
                  <div className="mini-head">
                    <span className="label">HiveCore service token</span>
                    <span className={`chip ${authStatus?.service_auth_configured ? "green" : "amber"}`}>
                      {authStatus?.service_auth_configured ? "paired" : "needs pairing"}
                    </span>
                  </div>
                  <div className="report-grid">
                    <div><span className="label">Name</span><strong>{serviceToken?.name || "hivecore"}</strong></div>
                    <div><span className="label">Fingerprint</span><strong>{serviceToken?.fingerprint || "none"}</strong></div>
                    <div><span className="label">Scoped</span><strong>{authStatus?.service_auth_scoped ? "yes" : "no"}</strong></div>
                    <div><span className="label">Expires</span><strong>{serviceToken?.expires_at ? formatDate(serviceToken.expires_at) : "none"}</strong></div>
                  </div>
                  <div className="chip-row">
                    {(serviceScopes.length ? serviceScopes : ["runs:read", "actions:dispatch"]).map((scope) => (
                      <span className="chip signal" key={scope}>{scope}</span>
                    ))}
                    {authStatus?.service_auth_legacy && <span className="chip amber">legacy token</span>}
                    {authStatus?.service_auth_expired && <span className="chip red">expired</span>}
                    {authStatus?.service_auth_expires_soon && <span className="chip amber">expires soon</span>}
                  </div>
                  {generatedServiceToken && (
                    <StatusBanner tone="green">
                      New service token: <span className="break-all">{generatedServiceToken}</span>
                    </StatusBanner>
                  )}
                  <div className="actions">
                    <button
                      className="btn primary"
                      disabled={serviceTokenBusy || authStatus?.service_auth_configured}
                      onClick={onGenerateServiceToken}
                      type="button"
                    >
                      {serviceTokenBusy ? "Working" : "Generate token"}
                    </button>
                    <button
                      className="btn"
                      disabled={serviceTokenBusy || !authStatus?.service_auth_configured}
                      onClick={onRotateServiceToken}
                      type="button"
                    >
                      {serviceTokenBusy ? "Working" : "Rotate token"}
                    </button>
                    <button
                      className="btn"
                      disabled={!generatedServiceToken}
                      onClick={onCopyServiceToken}
                      type="button"
                    >
                      Copy token
                    </button>
                  </div>
                </div>
                <div className="repo-list">
                  {checks.length === 0 ? (
                    <EmptyV2 title="No startup checks returned" />
                  ) : (
                    checks.map((check, index) => (
                      <div className="feed-item" key={`${check.msg}-${index}`}>
                        <div className="feed-title">{check.msg}</div>
                        <span className={`chip ${check.level === "error" ? "red" : check.level === "warn" ? "amber" : "green"}`}>{check.level}</span>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </Panel>
          </div>
        </main>
      </div>
    </>
  );
}

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "signal_api_key" });
  const fetch_ = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const [activeTab, setActiveTab] = usePersistentProductTab("signal-hive", TABS, "atlas");
  const [params, setParams] = useState(DEFAULT_PARAMS);
  const [scan, setScan] = useState(null);
  const [timeline, setTimeline] = useState(null);
  const [health, setHealth] = useState(null);
  const [checks, setChecks] = useState([]);
  const [authStatus, setAuthStatus] = useState(null);
  const [history, setHistory] = useState([]);
  const [presets, setPresets] = useState([]);
  const [schedules, setSchedules] = useState([]);
  const [repoLists, setRepoLists] = useState([]);
  const [sortBy, setSortBy] = useState("priority");
  const [running, setRunning] = useState(false);
  const [loadingScan, setLoadingScan] = useState(false);
  const [error, setError] = useState("");
  const [actionMessage, setActionMessage] = useState(null);
  const [generatedServiceToken, setGeneratedServiceToken] = useState("");
  const [serviceTokenBusy, setServiceTokenBusy] = useState(false);
  const [presetName, setPresetName] = useState("");
  const [selectedPresetName, setSelectedPresetName] = useState("");
  const [scheduleName, setScheduleName] = useState("");
  const [selectedScheduleName, setSelectedScheduleName] = useState("");
  const [scheduleCadence, setScheduleCadence] = useState("24");
  const [scheduleEnabled, setScheduleEnabled] = useState("true");
  const [repoControl, setRepoControl] = useState({ repo: "", list_type: "allowlist" });

  const ready = auth.checked && !auth.needsAuth;

  const loadTimeline = useCallback(async (scanId) => {
    if (!scanId) {
      setTimeline(null);
      return;
    }
    try {
      setTimeline(await fetchJson(fetch_, `${API}/history/${scanId}/timeline`));
    } catch {
      setTimeline(null);
    }
  }, [fetch_]);

  const loadScan = useCallback(async (scanId) => {
    if (!scanId) return;
    setLoadingScan(true);
    setError("");
    try {
      const detail = await fetchJson(fetch_, `${API}/history/${scanId}`);
      setScan(detail);
      setParams(toFormParams(detail.params));
      await loadTimeline(detail.id);
    } catch (err) {
      setError(err.message || "Could not load scan.");
    } finally {
      setLoadingScan(false);
    }
  }, [fetch_, loadTimeline]);

  const refreshCollections = useCallback(async ({ loadLatest = false } = {}) => {
    if (!ready) return;
    setError("");
    const results = await Promise.allSettled([
      fetchJson(fetch_, `${API}/health`),
      fetchJson(fetch_, `${API}/startup/checks`),
      fetchJson(fetch_, `${API}/auth/status`),
      fetchJson(fetch_, `${API}/history`),
      fetchJson(fetch_, `${API}/presets`),
      fetchJson(fetch_, `${API}/schedules`),
      fetchJson(fetch_, `${API}/repo-lists`),
    ]);

    const [healthResult, checksResult, authResult, historyResult, presetsResult, schedulesResult, reposResult] = results;
    if (healthResult.status === "fulfilled") setHealth(healthResult.value);
    if (checksResult.status === "fulfilled") setChecks(checksResult.value.checks || []);
    if (authResult.status === "fulfilled") setAuthStatus(authResult.value);
    if (historyResult.status === "fulfilled") {
      const scans = historyResult.value.scans || [];
      setHistory(scans);
      if (loadLatest && scans[0]?.id) {
        await loadScan(scans[0].id);
      }
    }
    if (presetsResult.status === "fulfilled") {
      const nextPresets = presetsResult.value.presets || [];
      setPresets(nextPresets);
      setSelectedPresetName((current) => nextPresets.some((preset) => preset.name === current) ? current : nextPresets[0]?.name || "");
    }
    if (schedulesResult.status === "fulfilled") {
      const nextSchedules = schedulesResult.value.schedules || [];
      setSchedules(nextSchedules);
      setSelectedScheduleName((current) => nextSchedules.some((schedule) => schedule.name === current) ? current : nextSchedules[0]?.name || "");
    }
    if (reposResult.status === "fulfilled") setRepoLists(reposResult.value.repos || []);

    const failed = results.find((result) => result.status === "rejected");
    if (failed) {
      setError(failed.reason?.message || "SignalHive could not load one or more backend resources.");
    }
  }, [fetch_, loadScan, ready]);

  useEffect(() => {
    if (ready) {
      refreshCollections({ loadLatest: true });
    }
  }, [ready, refreshCollections]);

  const runScan = async () => {
    setRunning(true);
    setError("");
    setActionMessage(null);
    try {
      const data = await fetchJson(fetch_, `${API}/scan`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(toRequestParams(params)),
      });
      setScan(data);
      await loadTimeline(data.id);
      await refreshCollections();
      setActionMessage({ tone: "green", text: `Scan complete: ${data.summary?.total_signals || 0} signals across ${data.summary?.total_repos || 0} repos.` });
    } catch (err) {
      setError(err.message || "Signal scan failed.");
    } finally {
      setRunning(false);
    }
  };

  const selectedPreset = presets.find((preset) => preset.name === selectedPresetName);
  const selectedSchedule = schedules.find((schedule) => schedule.name === selectedScheduleName);

  const savePreset = async () => {
    if (!presetName.trim()) return;
    try {
      await fetchJson(fetch_, `${API}/presets`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: presetName.trim(), params: toRequestParams(params) }),
      });
      setActionMessage({ tone: "green", text: `Preset saved: ${presetName.trim()}` });
      await refreshCollections();
      setSelectedPresetName(presetName.trim());
      setPresetName("");
    } catch (err) {
      setError(err.message || "Could not save preset.");
    }
  };

  const deletePreset = async () => {
    if (!selectedPreset) return;
    try {
      await fetchJson(fetch_, `${API}/presets/${encodeURIComponent(selectedPreset.name)}`, { method: "DELETE" });
      setActionMessage({ tone: "amber", text: `Preset deleted: ${selectedPreset.name}` });
      await refreshCollections();
    } catch (err) {
      setError(err.message || "Could not delete preset.");
    }
  };

  const loadPreset = () => {
    if (!selectedPreset) return;
    setParams(toFormParams(selectedPreset.params));
    setPresetName(selectedPreset.name);
    setActionMessage({ tone: "signal", text: `Loaded preset: ${selectedPreset.name}` });
  };

  const clearPreset = () => {
    setParams(DEFAULT_PARAMS);
    setPresetName("");
    setSelectedPresetName("");
    setActionMessage({ tone: "signal", text: "Preset form cleared." });
  };

  const saveSchedule = async () => {
    if (!scheduleName.trim()) return;
    try {
      await fetchJson(fetch_, `${API}/schedules`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: scheduleName.trim(),
          params: toRequestParams(params),
          cadence_hours: Number(scheduleCadence) || 24,
          enabled: scheduleEnabled === "true",
        }),
      });
      setActionMessage({ tone: "green", text: `Schedule saved: ${scheduleName.trim()}` });
      await refreshCollections();
      setSelectedScheduleName(scheduleName.trim());
    } catch (err) {
      setError(err.message || "Could not save schedule.");
    }
  };

  const deleteSchedule = async () => {
    if (!selectedSchedule) return;
    try {
      await fetchJson(fetch_, `${API}/schedules/${encodeURIComponent(selectedSchedule.name)}`, { method: "DELETE" });
      setActionMessage({ tone: "amber", text: `Schedule deleted: ${selectedSchedule.name}` });
      await refreshCollections();
    } catch (err) {
      setError(err.message || "Could not delete schedule.");
    }
  };

  const loadSchedule = () => {
    if (!selectedSchedule) return;
    setParams(toFormParams(selectedSchedule.params));
    setScheduleName(selectedSchedule.name);
    setScheduleCadence(String(selectedSchedule.cadence_hours || 24));
    setScheduleEnabled(selectedSchedule.enabled ? "true" : "false");
    setActionMessage({ tone: "signal", text: `Loaded schedule: ${selectedSchedule.name}` });
  };

  const clearSchedule = () => {
    setParams(DEFAULT_PARAMS);
    setScheduleName("");
    setScheduleCadence("24");
    setScheduleEnabled("true");
    setSelectedScheduleName("");
    setActionMessage({ tone: "signal", text: "Schedule form cleared." });
  };

  const runSchedule = async () => {
    if (!selectedSchedule) return;
    setRunning(true);
    setError("");
    try {
      const data = await fetchJson(fetch_, `${API}/schedules/${encodeURIComponent(selectedSchedule.name)}/run`, { method: "POST" });
      setScan(data);
      await loadTimeline(data.id);
      await refreshCollections();
      setActionMessage({ tone: "green", text: `Schedule run complete: ${selectedSchedule.name}` });
      setActiveTab("atlas");
    } catch (err) {
      setError(err.message || "Could not run schedule.");
    } finally {
      setRunning(false);
    }
  };

  const clearScan = () => {
    setScan(null);
    setTimeline(null);
    setError("");
  };

  const exportReport = async () => {
    if (!scan?.id) return;
    try {
      const report = await fetchJson(fetch_, `${API}/history/${scan.id}/report`);
      downloadTextFile(report.filename || `signalhive-report-${scan.id}.md`, report.markdown, "text/markdown;charset=utf-8");
      setActionMessage({ tone: "green", text: "Report exported." });
    } catch (err) {
      setError(err.message || "Could not export report.");
    }
  };

  const copySummary = async () => {
    if (!scan) return;
    try {
      await navigator.clipboard.writeText(buildReportSummary(scan));
      setActionMessage({ tone: "green", text: "Summary copied." });
    } catch {
      setError("Could not copy summary.");
    }
  };

  const addRepoControl = async () => {
    if (!repoControl.repo.trim()) return;
    try {
      await fetchJson(fetch_, `${API}/repo-lists`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(repoControl),
      });
      setRepoControl((prev) => ({ ...prev, repo: "" }));
      await refreshCollections();
      setActionMessage({ tone: "green", text: "Repo control saved." });
    } catch (err) {
      setError(err.message || "Could not save repo control.");
    }
  };

  const removeRepoControl = async (repo) => {
    try {
      await fetchJson(fetch_, `${API}/repo-lists/${encodeURIComponent(repo)}`, { method: "DELETE" });
      await refreshCollections();
      setActionMessage({ tone: "amber", text: `Repo control removed: ${repo}` });
    } catch (err) {
      setError(err.message || "Could not remove repo control.");
    }
  };

  const serviceTokenRequest = async (path, successText) => {
    setServiceTokenBusy(true);
    setError("");
    setActionMessage(null);
    try {
      const data = await fetchJson(fetch_, `${API}${path}`, { method: "POST" });
      const token = data.service_token || "";
      setGeneratedServiceToken(token);
      await refreshCollections();
      setActionMessage({ tone: "green", text: successText });
    } catch (err) {
      setError(err.message || "Could not update HiveCore service token.");
    } finally {
      setServiceTokenBusy(false);
    }
  };

  const generateServiceToken = () => (
    serviceTokenRequest("/auth/generate-service-token", "HiveCore service token generated.")
  );

  const rotateServiceToken = () => (
    serviceTokenRequest("/auth/rotate-service-token", "HiveCore service token rotated.")
  );

  const copyServiceToken = async () => {
    if (!generatedServiceToken) return;
    try {
      await navigator.clipboard.writeText(generatedServiceToken);
      setActionMessage({ tone: "green", text: "Service token copied." });
    } catch {
      setError("Could not copy service token.");
    }
  };

  if (!ready) {
    return (
      <AuthScreen
        authError={auth.authError}
        bootstrapRequired={auth.bootstrapRequired}
        checked={auth.checked}
        generateKey={auth.generateKey}
        login={auth.login}
      />
    );
  }

  return (
    <ProductV2Shell
      authConfigured={Boolean(authStatus?.auth_configured || health?.auth_enabled)}
      productKey="signal-hive"
      productName="SignalHive"
    >
      <DeckBar
        activeTab={activeTab}
        brandEyebrow="PatchHive"
        brandName="SignalHive"
        navLabel="SignalHive navigation"
        onTabChange={setActiveTab}
        productKey="signal-hive"
        tabs={TABS}
      />
      {activeTab === "atlas" && (
        <AtlasBoard
          actionMessage={actionMessage}
          authConfigured={Boolean(authStatus?.auth_configured || health?.auth_enabled)}
          checks={checks}
          error={error}
          health={health}
          onClearScan={clearScan}
          onClearPreset={clearPreset}
          onClearSchedule={clearSchedule}
          onDeletePreset={deletePreset}
          onDeleteSchedule={deleteSchedule}
          onExportReport={exportReport}
          onLoadPreset={loadPreset}
          onLoadSchedule={loadSchedule}
          onRefresh={() => refreshCollections()}
          onRun={runScan}
          onRunSchedule={runSchedule}
          onSavePreset={savePreset}
          onSaveSchedule={saveSchedule}
          params={params}
          presetName={presetName}
          presets={presets}
          repoLists={repoLists}
          running={running}
          scan={scan}
          scheduleCadence={scheduleCadence}
          scheduleEnabled={scheduleEnabled}
          scheduleName={scheduleName}
          schedules={schedules}
          selectedPresetName={selectedPresetName}
          selectedScheduleName={selectedScheduleName}
          setParams={setParams}
          setPresetName={setPresetName}
          setScheduleCadence={setScheduleCadence}
          setScheduleEnabled={setScheduleEnabled}
          setScheduleName={setScheduleName}
          setSelectedPresetName={setSelectedPresetName}
          setSelectedScheduleName={setSelectedScheduleName}
          setSortBy={setSortBy}
          sortBy={sortBy}
        />
      )}
      {activeTab === "ledger" && (
        <LedgerBoard
          authConfigured={Boolean(authStatus?.auth_configured || health?.auth_enabled)}
          health={health}
          history={history}
          loadingScan={loadingScan}
          onClearScan={clearScan}
          onCopySummary={copySummary}
          onExportReport={exportReport}
          onLoadScan={loadScan}
          onRefresh={() => refreshCollections()}
          schedules={schedules}
          scan={scan}
          setSortBy={setSortBy}
          sortBy={sortBy}
          timeline={timeline}
        />
      )}
      {activeTab === "floor" && (
        <WatchFloor
          actionMessage={actionMessage}
          authConfigured={Boolean(authStatus?.auth_configured || health?.auth_enabled)}
          authStatus={authStatus}
          checks={checks}
          error={error}
          generatedServiceToken={generatedServiceToken}
          health={health}
          onAddRepoControl={addRepoControl}
          onClearScan={clearScan}
          onCopyServiceToken={copyServiceToken}
          onGenerateServiceToken={generateServiceToken}
          onRefresh={() => refreshCollections()}
          onRemoveRepoControl={removeRepoControl}
          onRotateServiceToken={rotateServiceToken}
          onSignOut={auth.logout}
          repoControl={repoControl}
          repoLists={repoLists}
          schedules={schedules}
          scan={scan}
          serviceTokenBusy={serviceTokenBusy}
          setRepoControl={setRepoControl}
        />
      )}
    </ProductV2Shell>
  );
}

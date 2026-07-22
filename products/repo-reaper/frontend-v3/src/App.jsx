import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Skull } from "lucide-react";
import { createApiFetcher, useApiKeyAuth } from "@patchhivehq/product-shell/auth";
import { ProductHeader, ProductLoginScreen, ProductShell, V3_TEXT } from "@patchhivehq/ui-v3";
import { API } from "./config.js";
import ChecksPanel from "./ChecksPanel.jsx";
import ControlsPanel from "./ControlsPanel.jsx";
import HistoryPanel from "./HistoryPanel.jsx";
import PrPanel from "./PrPanel.jsx";
import RunPanel from "./RunPanel.jsx";
import SquadPanel from "./SquadPanel.jsx";
import { createStreamState, DEFAULT_DRY_PARAMS, DEFAULT_PARAMS, readResponse } from "./shared.jsx";

const PRODUCT = { productKey: "repo-reaper", name: "RepoReaper", subtitle: "autonomous patch execution", icon: Skull };
const TABS = [
  { id: "mission", label: "Mission" },
  { id: "dry", label: "Dry Stalk" },
  { id: "history", label: "History" },
  { id: "prs", label: "PRs" },
  { id: "squad", label: "Squad" },
  { id: "controls", label: "Controls" },
  { id: "checks", label: "Checks" },
];

function issueKey(issue) {
  return String(issue.id || issue.issue_id || `${issue.repo || "repo"}-${issue.issue_number || issue.number || "issue"}`);
}

function applyIssueEvent(current, event, payload) {
  if (event === "issues") return Array.isArray(payload.issues) ? payload.issues : current;
  const incoming = payload.issue && typeof payload.issue === "object" ? payload.issue : payload;
  const key = issueKey(incoming);
  let matched = false;
  const next = current.map((issue) => {
    const same = issueKey(issue) === key || (incoming.issue_number || incoming.number) === (issue.issue_number || issue.number);
    if (!same) return issue;
    matched = true;
    return { ...issue, ...incoming, ...(event === "issue_assign" ? { status: "assigned" } : {}), ...(event === "issue_confidence" ? { confidence: incoming.confidence } : {}) };
  });
  return matched ? next : [...next, incoming];
}

function MainProduct({ auth }) {
  const fetcher = useMemo(() => createApiFetcher(auth.apiKey), [auth.apiKey]);
  const abortRef = useRef(null);
  const [activeTab, setActiveTab] = useState(() => localStorage.getItem("repo-reaper.v3.tab") || "mission");
  const [params, setParams] = useState(DEFAULT_PARAMS);
  const [dryParams, setDryParams] = useState(DEFAULT_DRY_PARAMS);
  const [targetMode, setTargetMode] = useState("direct");
  const [dryTargetMode, setDryTargetMode] = useState("discovery");
  const [missionStream, setMissionStream] = useState(createStreamState);
  const [dryStream, setDryStream] = useState(createStreamState);
  const [health, setHealth] = useState({});
  const [checks, setChecks] = useState([]);
  const [history, setHistory] = useState([]);
  const [selectedRun, setSelectedRun] = useState(null);
  const [agents, setAgents] = useState([]);
  const [config, setConfig] = useState({});
  const [rejected, setRejected] = useState([]);
  const [prs, setPrs] = useState([]);
  const [leaderboard, setLeaderboard] = useState([]);
  const [watchMode, setWatchMode] = useState(false);
  const [presets, setPresets] = useState([]);
  const [cooldowns, setCooldowns] = useState({});
  const [loading, setLoading] = useState(true);
  const [prBusy, setPrBusy] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async ({ loadLatest = false } = {}) => {
    setLoading(true); setError("");
    const paths = ["health", "startup/checks", "history", "agents", "config", "rejected", "pr-tracking", "leaderboard", "watch-mode", "presets", "cooldowns"];
    const results = await Promise.allSettled(paths.map(async (path) => readResponse(await fetcher(`${API}/${path}`), `Could not load ${path}`)));
    const value = (index) => results[index].status === "fulfilled" ? results[index].value : {};
    const nextHistory = value(2).history || [];
    setHealth(value(0)); setChecks(value(1).checks || []); setHistory(nextHistory);
    setAgents(value(3).agents || []); setCooldowns(value(10).cooldowns || value(3).cooldowns || {});
    setConfig(value(4)); setRejected(value(5).rejected || []); setPrs(value(6).prs || []);
    setLeaderboard(value(7).leaderboard || []); setWatchMode(Boolean(value(8).watch_mode)); setPresets(value(9).presets || []);
    const failed = results.find((result) => result.status === "rejected");
    if (failed) setError(failed.reason?.message || "RepoReaper could not load one or more resources.");
    if (loadLatest && nextHistory[0]?.id) {
      try { setSelectedRun(await readResponse(await fetcher(`${API}/history/${encodeURIComponent(nextHistory[0].id)}`), "Could not load latest run")); }
      catch (nextError) { setError(nextError.message); }
    }
    setLoading(false);
  }, [fetcher]);

  useEffect(() => { refresh({ loadLatest: true }); }, [refresh]);
  useEffect(() => { localStorage.setItem("repo-reaper.v3.tab", activeTab); }, [activeTab]);
  useEffect(() => () => abortRef.current?.abort(), []);

  async function loadRun(id) {
    setError("");
    try { setSelectedRun(await readResponse(await fetcher(`${API}/history/${encodeURIComponent(id)}`), "Could not load run")); }
    catch (nextError) { setError(nextError.message); }
  }

  function startStream(path, payload, setter) {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    setter({ ...createStreamState(), running: true, phase: "starting" });
    setError("");
    fetcher(`${API}/${path}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload), signal: controller.signal })
      .then(async (response) => {
        if (!response.ok || !response.body) throw new Error((await response.text()) || `Request failed: ${response.status}`);
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = "";
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          const parts = buffer.split("\n\n");
          buffer = parts.pop() || "";
          parts.forEach((part) => {
            const event = part.match(/^event:\s*(.+)$/m)?.[1]?.trim();
            const raw = part.match(/^data:\s*(.+)$/m)?.[1];
            if (!event || !raw) return;
            try {
              const data = JSON.parse(raw);
              setter((current) => {
                if (event === "phase") return { ...current, phase: data.phase || current.phase };
                if (event === "issues" || event.startsWith("issue_")) return { ...current, issues: applyIssueEvent(current.issues, event, data) };
                if (event === "agent_status") return { ...current, agentStatuses: { ...current.agentStatuses, [data.agent_id]: data } };
                if (event === "agent_log" || event === "log") return { ...current, logs: [...current.logs.slice(-299), data] };
                if (event === "cost_update") return { ...current, runCost: Number(data.run_cost || data.cost || current.runCost) };
                if (event === "dry_run_report") return { ...current, report: data.report || data };
                if (event === "done") return { ...current, done: data, phase: data.status || "done", runCost: Number(data.cost || current.runCost), running: false };
                if (event === "error") return { ...current, logs: [...current.logs, { ...data, type: "error" }], phase: "error", running: false };
                return current;
              });
            } catch { setError("RepoReaper emitted a malformed live event; the saved run remains available in History."); }
          });
        }
      })
      .catch((streamError) => { if (streamError.name !== "AbortError") setError(streamError.message || "RepoReaper stream ended unexpectedly."); })
      .finally(async () => { if (abortRef.current === controller) abortRef.current = null; setter((current) => ({ ...current, running: false })); await refresh(); });
  }

  async function refreshPr(pr) {
    setPrBusy(true); setError("");
    try { await readResponse(await fetcher(`${API}/pr-tracking/refresh`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ repo: pr.repo, pr_number: pr.pr_number }) }), "Could not refresh PR"); await refresh(); }
    catch (nextError) { setError(nextError.message); }
    finally { setPrBusy(false); }
  }

  return <ProductShell productKey="repo-reaper">
    <ProductHeader activeTab={activeTab} githubLabel={health.github_ready ? "GitHub verified" : health.github?.token_configured ? "GitHub unverified" : "Write token missing"} icon={Skull} onRun={() => setActiveTab("mission")} onSignOut={auth.logout} onTabChange={setActiveTab} productName="RepoReaper" runDisabled={missionStream.running || dryStream.running} runLabel="Scope mission" subtitle="autonomous patch execution" tabs={TABS}/>
    {error ? <div className="mx-auto mt-5 max-w-[1400px] px-3 sm:px-6"><div className="surface px-5 py-4 text-[12px] text-red-800 dark:text-red-300">{error}</div></div> : null}
    {activeTab === "mission" ? <RunPanel agents={agents} health={health} onParamsChange={setParams} onStart={(payload) => startStream("run", payload, setMissionStream)} onTargetSelectionModeChange={setTargetMode} params={params} stream={missionStream} targetSelectionMode={targetMode}/> : null}
    {activeTab === "dry" ? <RunPanel agents={agents} dry health={health} onParamsChange={setDryParams} onStart={(payload) => startStream("dry-run", payload, setDryStream)} onTargetSelectionModeChange={setDryTargetMode} params={dryParams} stream={dryStream} targetSelectionMode={dryTargetMode}/> : null}
    {activeTab === "history" ? <HistoryPanel history={history} leaderboard={leaderboard} loading={loading} onLoadRun={loadRun} onRefresh={refresh} rejected={rejected} selectedRun={selectedRun}/> : null}
    {activeTab === "prs" ? <PrPanel busy={prBusy} onRefresh={refresh} onRefreshPr={refreshPr} prs={prs}/> : null}
    {activeTab === "squad" ? <SquadPanel agents={agents} apiBase={API} cooldowns={cooldowns} fetcher={fetcher} onError={setError} onRefresh={refresh} presets={presets}/> : null}
    {activeTab === "controls" ? <ControlsPanel apiBase={API} config={config} dryParams={dryParams} dryTargetMode={dryTargetMode} fetcher={fetcher} onError={setError} onRefresh={refresh} params={params} setDryParams={setDryParams} setDryTargetMode={setDryTargetMode} setParams={setParams} setTargetMode={setTargetMode} targetMode={targetMode} watchMode={watchMode}/> : null}
    {activeTab === "checks" ? <ChecksPanel checks={checks} config={config} health={health} onRefresh={refresh}/> : null}
  </ProductShell>;
}

export default function App() {
  const auth = useApiKeyAuth({ apiBase: API, storageKey: "repo-reaper_api_key" });
  if (!auth.checked) return <ProductShell productKey="repo-reaper"><div className={`grid min-h-screen place-items-center ${V3_TEXT.mute}`}>Connecting…</div></ProductShell>;
  if (auth.needsAuth) return <ProductLoginScreen apiBase={API} auth={auth} config={PRODUCT}/>;
  return <MainProduct auth={auth}/>;
}

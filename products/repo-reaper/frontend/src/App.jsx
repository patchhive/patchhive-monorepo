import { useState, useCallback, useEffect, useRef } from "react";
import {
  applyTheme, PHASE_LABEL, PHASE_ICON,
  DiffViewer,
} from "@patchhivehq/ui";
import {
  ProductAppFrame,
  ProductSessionGate,
  useApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import { API } from "./config.js";
import TeamPanel       from "./panels/TeamPanel.jsx";
import RunPanel        from "./panels/RunPanel.jsx";
import DryRunPanel     from "./panels/DryRunPanel.jsx";
import HistoryPanel    from "./panels/HistoryPanel.jsx";
import LeaderboardPanel from "./panels/LeaderboardPanel.jsx";
import PresetsPanel    from "./panels/PresetsPanel.jsx";
import RejectedPanel   from "./panels/RejectedPanel.jsx";
import ConfigPanel     from "./panels/ConfigPanel.jsx";
import {
  RepoListsPanel, SchedulesPanel, WebhookPanel,
  PRTrackingPanel, StartupChecksPanel,
} from "./panels/misc.jsx";

const TABS = [
  { id:"team",     label:"⚔ Team" },
  { id:"run",      label:"🔱 Hunt" },
  { id:"dryrun",   label:"◌ Dry Stalk" },
  { id:"history",  label:"◎ History" },
  { id:"board",    label:"⚖ Leaderboard" },
  { id:"rejected", label:"⬢ Rejected" },
  { id:"prs",      label:"↗ PRs" },
  { id:"presets",  label:"Presets" },
  { id:"repos",    label:"Repo Lists" },
  { id:"sched",    label:"Schedules" },
  { id:"webhook",  label:"Webhooks" },
  { id:"startup",  label:"Checks" },
  { id:"cfg",      label:"Config" },
];

const DEFAULT_PARAMS = {
  language:"python", min_stars:50, max_repos:10, max_issues:10,
  concurrency:3, search_query:"", cost_budget_usd:0, retry_count:3,
};

export default function App() {
  const { apiKey, checked, needsAuth, login, logout, authError, bootstrapRequired, generateKey } = useApiKeyAuth({
    apiBase: API,
    storageKey: "reaper_api_key",
  });
  const [tab,          setTab]          = useState("team");
  const [agents,       setAgents]       = useState({});
  const [logs,         setLogs]         = useState([]);
  const [issues,       setIssues]       = useState({});
  const [running,      setRunning]      = useState(false);
  const [phase,        setPhase]        = useState("");
  const [cooldowns,    setCooldowns]    = useState({});
  const [runCost,      setRunCost]      = useState(0);
  const [lifetimeCost, setLifetimeCost] = useState(0);
  const [viewDiff,     setViewDiff]     = useState(null);
  const [runStats,     setRunStats]     = useState(null);
  const [params,       setParams]       = useState(DEFAULT_PARAMS);
  const [existingCfg,  setExistingCfg]  = useState({});
  const [watchMode,    setWatchMode]    = useState(false);
  const runAbortRef = useRef(null);

  const fetch_ = useApiFetcher(apiKey);

  useEffect(() => { applyTheme("repo-reaper"); }, []);

  const refreshConfig = useCallback(() => {
    return fetch_(`${API}/config`).then(r=>r.json()).then(setExistingCfg).catch(()=>{});
  }, [fetch_]);

  useEffect(() => {
    if (!checked || needsAuth) return;
    fetch_(`${API}/agents`).then(r=>r.json()).then(d => {
      const m={}; (d.agents||[]).forEach(a => m[a.id]=a);
      setAgents(m); setCooldowns(d.cooldowns||{});
    }).catch(()=>{});
    refreshConfig();
    fetch_(`${API}/stats/lifetime-cost`).then(r=>r.json()).then(d=>setLifetimeCost(d.lifetime_cost_usd||0)).catch(()=>{});
    fetch_(`${API}/watch-mode`).then(r=>r.json()).then(d=>setWatchMode(d.watch_mode||false)).catch(()=>{});
  }, [checked, needsAuth, fetch_, refreshConfig]);

  useEffect(() => () => runAbortRef.current?.abort(), []);

  const pushTeam = useCallback(async map => {
    await fetch_(`${API}/agents`, { method:"POST", headers:{"Content-Type":"application/json"}, body:JSON.stringify({ agents:Object.values(map) }) });
  }, [fetch_]);

  const addAgent = useCallback(async cfg => {
    const id = Math.random().toString(36).slice(2,10);
    const next = { ...agents, [id]: { ...cfg, id, status:"idle", current_task:"", stats:{fixed:0,skipped:0,errors:0,cost:0} } };
    setAgents(next); await pushTeam(next);
  }, [agents, pushTeam]);

  const removeAgent = useCallback(async id => {
    const next = { ...agents }; delete next[id]; setAgents(next); await pushTeam(next);
  }, [agents, pushTeam]);

  const loadPreset = useCallback(async list => {
    const m = {};
    list.forEach(a => { const id=a.id||Math.random().toString(36).slice(2,10); m[id]={...a,id,status:"idle",current_task:"",stats:{fixed:0,skipped:0,errors:0,cost:0}}; });
    setAgents(m); await pushTeam(m);
  }, [pushTeam]);

  const toggleWatchMode = useCallback(async () => {
    const next = !watchMode;
    await fetch_(`${API}/watch-mode`, { method:"POST", headers:{"Content-Type":"application/json"}, body:JSON.stringify({ enabled:next }) });
    setWatchMode(next);
  }, [watchMode, fetch_]);

  const handle = useCallback((ev, d) => {
    if (ev==="phase")            setPhase(d.phase);
    if (ev==="agent_log")        setLogs(l => [...l.slice(-200), d]);
    if (ev==="agent_status")     setAgents(p => p[d.agent_id] ? { ...p, [d.agent_id]:{ ...p[d.agent_id], status:d.status, current_task:d.task } } : p);
    if (ev==="issues")           { const m={}; (d.issues||[]).forEach(i => m[i.id]={...i,status:"queued"}); setIssues(m); }
    if (ev==="issue_assign")     setIssues(p => p[d.id] ? { ...p, [d.id]:{ ...p[d.id], status:"running", fixability_score:d.score??p[d.id].fixability_score } } : p);
    if (ev==="issue_confidence") setIssues(p => p[d.id] ? { ...p, [d.id]:{ ...p[d.id], confidence:d.confidence } } : p);
    if (ev==="issue_result")     setIssues(p => p[d.id] ? { ...p, [d.id]:{ ...p[d.id], status:d.status, reason:d.reason, pr_url:d.pr?.url, pr_number:d.pr?.number, diff:d.pr?.diff, confidence:d.pr?.confidence, feedback:d.feedback } } : p);
    if (ev==="cost_update")      { setRunCost(d.run_cost||0); if(d.lifetime_cost!=null) setLifetimeCost(d.lifetime_cost); }
    if (ev==="done") {
      setRunning(false); setRunStats(d);
      fetch_(`${API}/cooldowns`).then(r=>r.json()).then(d=>setCooldowns(d.cooldowns||{})).catch(()=>{});
      fetch_(`${API}/stats/lifetime-cost`).then(r=>r.json()).then(d=>setLifetimeCost(d.lifetime_cost_usd||0)).catch(()=>{});
    }
  }, [fetch_]);

  const startRun = useCallback(() => {
    if (running) return;
    runAbortRef.current?.abort();
    const controller = new AbortController();
    runAbortRef.current = controller;
    setRunning(true); setLogs([]); setIssues({}); setPhase("scan"); setRunStats(null); setRunCost(0);
    fetch_(`${API}/run`, {
      method:"POST", headers:{"Content-Type":"application/json"},
      signal: controller.signal,
      body: JSON.stringify({ ...params, min_stars:+params.min_stars, max_repos:+params.max_repos, max_issues:+params.max_issues, concurrency:+params.concurrency, cost_budget_usd:+params.cost_budget_usd, retry_count:+params.retry_count, labels:["bug"] }),
    }).then(res => {
      if (!res.ok || !res.body) {
        setRunning(false);
        return;
      }
      const reader = res.body.getReader(); const dec = new TextDecoder(); let buf = "";
      const pump = () => reader.read().then(({ done, value }) => {
        if (done) { if (runAbortRef.current === controller) runAbortRef.current = null; setRunning(false); return; }
        buf += dec.decode(value, { stream:true });
        const parts = buf.split("\n\n"); buf = parts.pop();
        parts.forEach(p => {
          const em = p.match(/^event: (.+)/m); const dm = p.match(/^data: (.+)/m);
          if (em && dm) {
            try {
              handle(em[1].trim(), JSON.parse(dm[1]));
            } catch (error) {
              console.warn("Skipping malformed SSE payload", error);
            }
          }
        });
        pump();
      }).catch(error => {
        if (error?.name !== "AbortError") {
          console.warn("RepoReaper run stream ended unexpectedly", error);
        }
        if (runAbortRef.current === controller) runAbortRef.current = null;
        setRunning(false);
      });
      pump();
    }).catch(error => {
      if (error?.name !== "AbortError") {
        console.warn("RepoReaper run request failed", error);
      }
      if (runAbortRef.current === controller) runAbortRef.current = null;
      setRunning(false);
    });
  }, [running, params, handle, fetch_]);

  const hasCooldown = Object.keys(cooldowns).length > 0;

  return (
    <ProductSessionGate
      checked={checked}
      needsAuth={needsAuth}
      onLogin={login}
      icon="🔱"
      title="RepoReaper"
      storageKey="reaper_api_key"
      apiBase={API}
      authError={authError}
      bootstrapRequired={bootstrapRequired}
      onGenerateKey={generateKey}
      loadingColor="#1c1c30"
    >
      <ProductAppFrame
        icon="🔱"
        title="RepoReaper"
        product="RepoReaper"
        running={running}
        phase={phase}
        phaseLabel={PHASE_LABEL}
        phaseIcon={PHASE_ICON}
        headerChildren={
          <>
            {watchMode && <div style={{ fontSize:9, color:"var(--green)", border:"1px solid var(--green)44", borderRadius:3, padding:"2px 7px" }}>● Watch Mode</div>}
            {hasCooldown && <div style={{ fontSize:9, color:"var(--purple)", border:"1px solid var(--purple)33", borderRadius:3, padding:"2px 7px" }}>⏸ {Object.keys(cooldowns).join(",")} cooling</div>}
            {runCost > 0 && <span style={{ fontSize:10, color:"var(--gold)" }}>Run: ${runCost.toFixed(4)}</span>}
            {lifetimeCost > 0 && <span style={{ fontSize:10, color:"var(--text-dim)" }}>Lifetime: ${lifetimeCost.toFixed(4)}</span>}
          </>
        }
        tabs={TABS}
        activeTab={tab}
        onTabChange={setTab}
        maxWidth={1400}
        contentStyle={{ gap: 0 }}
        onSignOut={logout}
        showSignOut={Boolean(apiKey)}
      >
        {tab==="team"     && <TeamPanel agents={agents} logs={logs} running={running} cooldowns={cooldowns} onAdd={addAgent} onRemove={removeAgent} apiKey={apiKey} existingConfig={existingCfg} />}
        {tab==="run"      && <RunPanel running={running} onStart={startRun} params={params} setParams={setParams} issues={issues} logs={logs} agents={agents} runStats={runStats} runCost={runCost} onViewDiff={setViewDiff} />}
        {tab==="dryrun"   && <DryRunPanel agents={agents} apiKey={apiKey} onViewDiff={setViewDiff} />}
        {tab==="history"  && <HistoryPanel apiKey={apiKey} onViewDiff={setViewDiff} />}
        {tab==="board"    && <LeaderboardPanel apiKey={apiKey} />}
        {tab==="rejected" && <RejectedPanel apiKey={apiKey} onViewDiff={setViewDiff} />}
        {tab==="prs"      && <PRTrackingPanel apiKey={apiKey} />}
        {tab==="presets"  && <PresetsPanel apiKey={apiKey} currentAgents={Object.values(agents)} onLoadPreset={loadPreset} />}
        {tab==="repos"    && <RepoListsPanel apiKey={apiKey} />}
        {tab==="sched"    && <SchedulesPanel apiKey={apiKey} />}
        {tab==="webhook"  && <WebhookPanel watchMode={watchMode} onToggleWatch={toggleWatchMode} />}
        {tab==="startup"  && <StartupChecksPanel apiKey={apiKey} />}
        {tab==="cfg"      && <ConfigPanel existingConfig={existingCfg} apiKey={apiKey} onSaved={refreshConfig} />}
      </ProductAppFrame>

      {viewDiff && <DiffViewer diff={viewDiff} onClose={() => setViewDiff(null)} />}
    </ProductSessionGate>
  );
}

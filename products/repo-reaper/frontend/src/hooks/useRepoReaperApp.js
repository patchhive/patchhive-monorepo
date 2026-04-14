import { useState, useCallback, useEffect, useRef } from "react";
import { applyTheme } from "@patchhivehq/ui";
import {
  useApiFetcher,
  useApiKeyAuth,
} from "@patchhivehq/product-shell";
import { API } from "../config.js";

export const REPO_REAPER_TABS = [
  { id: "team", label: "⚔ Team" },
  { id: "run", label: "🔱 Hunt" },
  { id: "dryrun", label: "◌ Dry Stalk" },
  { id: "history", label: "◎ History" },
  { id: "board", label: "⚖ Leaderboard" },
  { id: "rejected", label: "⬢ Rejected" },
  { id: "prs", label: "↗ PRs" },
  { id: "presets", label: "Presets" },
  { id: "repos", label: "Repo Lists" },
  { id: "sched", label: "Schedules" },
  { id: "webhook", label: "Webhooks" },
  { id: "startup", label: "Checks" },
  { id: "cfg", label: "Config" },
];

export const DEFAULT_RUN_PARAMS = {
  language: "python",
  min_stars: 50,
  max_repos: 10,
  max_issues: 10,
  concurrency: 3,
  search_query: "",
  cost_budget_usd: 0,
  retry_count: 3,
};

const DEFAULT_AGENT_STATS = { fixed: 0, skipped: 0, errors: 0, cost: 0 };

function normalizeAgent(agent, fallbackId) {
  const id = agent.id || fallbackId || Math.random().toString(36).slice(2, 10);
  return {
    ...agent,
    id,
    status: agent.status || "idle",
    current_task: agent.current_task || "",
    stats: agent.stats || DEFAULT_AGENT_STATS,
  };
}

function normalizeAgentMap(list) {
  const map = {};
  (list || []).forEach(agent => {
    const normalized = normalizeAgent(agent);
    map[normalized.id] = normalized;
  });
  return map;
}

function normalizeRunRequest(params) {
  return {
    ...params,
    min_stars: +params.min_stars,
    max_repos: +params.max_repos,
    max_issues: +params.max_issues,
    concurrency: +params.concurrency,
    cost_budget_usd: +params.cost_budget_usd,
    retry_count: +params.retry_count,
    labels: ["bug"],
  };
}

function applyIssueEvent(currentIssues, event, payload) {
  if (event === "issues") {
    const next = {};
    (payload.issues || []).forEach(issue => {
      next[issue.id] = { ...issue, status: "queued" };
    });
    return next;
  }

  if (!payload.id || !currentIssues[payload.id]) {
    return currentIssues;
  }

  const current = currentIssues[payload.id];

  if (event === "issue_assign") {
    return {
      ...currentIssues,
      [payload.id]: {
        ...current,
        status: "running",
        fixability_score: payload.score ?? current.fixability_score,
      },
    };
  }

  if (event === "issue_confidence") {
    return {
      ...currentIssues,
      [payload.id]: { ...current, confidence: payload.confidence },
    };
  }

  if (event === "issue_result") {
    return {
      ...currentIssues,
      [payload.id]: {
        ...current,
        status: payload.status,
        reason: payload.reason,
        pr_url: payload.pr?.url,
        pr_number: payload.pr?.number,
        diff: payload.pr?.diff,
        confidence: payload.pr?.confidence,
        feedback: payload.feedback,
      },
    };
  }

  return currentIssues;
}

export function useRepoReaperApp() {
  const auth = useApiKeyAuth({
    apiBase: API,
    storageKey: "reaper_api_key",
  });
  const { apiKey, checked, needsAuth } = auth;
  const fetch_ = useApiFetcher(apiKey);
  const runAbortRef = useRef(null);

  const [tab, setTab] = useState("team");
  const [agents, setAgents] = useState({});
  const [logs, setLogs] = useState([]);
  const [issues, setIssues] = useState({});
  const [running, setRunning] = useState(false);
  const [phase, setPhase] = useState("");
  const [cooldowns, setCooldowns] = useState({});
  const [runCost, setRunCost] = useState(0);
  const [lifetimeCost, setLifetimeCost] = useState(0);
  const [viewDiff, setViewDiff] = useState(null);
  const [runStats, setRunStats] = useState(null);
  const [params, setParams] = useState(DEFAULT_RUN_PARAMS);
  const [existingCfg, setExistingCfg] = useState({});
  const [watchMode, setWatchMode] = useState(false);

  useEffect(() => {
    applyTheme("repo-reaper");
  }, []);

  const refreshConfig = useCallback(() => {
    return fetch_(`${API}/config`)
      .then(r => r.json())
      .then(setExistingCfg)
      .catch(() => {});
  }, [fetch_]);

  const refreshAgents = useCallback(() => {
    return fetch_(`${API}/agents`)
      .then(r => r.json())
      .then(data => {
        setAgents(normalizeAgentMap(data.agents));
        setCooldowns(data.cooldowns || {});
      })
      .catch(() => {});
  }, [fetch_]);

  const refreshLifetimeCost = useCallback(() => {
    return fetch_(`${API}/stats/lifetime-cost`)
      .then(r => r.json())
      .then(data => setLifetimeCost(data.lifetime_cost_usd || 0))
      .catch(() => {});
  }, [fetch_]);

  const refreshWatchMode = useCallback(() => {
    return fetch_(`${API}/watch-mode`)
      .then(r => r.json())
      .then(data => setWatchMode(data.watch_mode || false))
      .catch(() => {});
  }, [fetch_]);

  useEffect(() => {
    if (!checked || needsAuth) return;
    refreshAgents();
    refreshConfig();
    refreshLifetimeCost();
    refreshWatchMode();
  }, [checked, needsAuth, refreshAgents, refreshConfig, refreshLifetimeCost, refreshWatchMode]);

  useEffect(() => () => runAbortRef.current?.abort(), []);

  const pushTeam = useCallback(async nextAgents => {
    await fetch_(`${API}/agents`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ agents: Object.values(nextAgents) }),
    });
  }, [fetch_]);

  const addAgent = useCallback(async agentConfig => {
    const nextAgent = normalizeAgent(agentConfig);
    const nextAgents = { ...agents, [nextAgent.id]: nextAgent };
    setAgents(nextAgents);
    await pushTeam(nextAgents);
  }, [agents, pushTeam]);

  const removeAgent = useCallback(async id => {
    const nextAgents = { ...agents };
    delete nextAgents[id];
    setAgents(nextAgents);
    await pushTeam(nextAgents);
  }, [agents, pushTeam]);

  const loadPreset = useCallback(async list => {
    const nextAgents = {};
    list.forEach(agent => {
      const normalized = normalizeAgent(agent);
      nextAgents[normalized.id] = normalized;
    });
    setAgents(nextAgents);
    await pushTeam(nextAgents);
  }, [pushTeam]);

  const toggleWatchMode = useCallback(async () => {
    const next = !watchMode;
    await fetch_(`${API}/watch-mode`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ enabled: next }),
    });
    setWatchMode(next);
  }, [watchMode, fetch_]);

  const handleStreamEvent = useCallback((event, payload) => {
    if (event === "phase") {
      setPhase(payload.phase);
      return;
    }

    if (event === "agent_log") {
      setLogs(current => [...current.slice(-200), payload]);
      return;
    }

    if (event === "agent_status") {
      setAgents(current =>
        current[payload.agent_id]
          ? {
              ...current,
              [payload.agent_id]: {
                ...current[payload.agent_id],
                status: payload.status,
                current_task: payload.task,
              },
            }
          : current,
      );
      return;
    }

    if (event === "issues" || event === "issue_assign" || event === "issue_confidence" || event === "issue_result") {
      setIssues(current => applyIssueEvent(current, event, payload));
      return;
    }

    if (event === "cost_update") {
      setRunCost(payload.run_cost || 0);
      if (payload.lifetime_cost != null) {
        setLifetimeCost(payload.lifetime_cost);
      }
      return;
    }

    if (event === "done") {
      setRunning(false);
      setRunStats(payload);
      fetch_(`${API}/cooldowns`).then(r => r.json()).then(d => setCooldowns(d.cooldowns || {})).catch(() => {});
      refreshLifetimeCost();
    }
  }, [fetch_, refreshLifetimeCost]);

  const startRun = useCallback(() => {
    if (running) return;

    runAbortRef.current?.abort();
    const controller = new AbortController();
    runAbortRef.current = controller;

    setRunning(true);
    setLogs([]);
    setIssues({});
    setPhase("scan");
    setRunStats(null);
    setRunCost(0);

    fetch_(`${API}/run`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      signal: controller.signal,
      body: JSON.stringify(normalizeRunRequest(params)),
    })
      .then(res => {
        if (!res.ok || !res.body) {
          setRunning(false);
          return;
        }

        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        let buffer = "";

        const pump = () =>
          reader.read().then(({ done, value }) => {
            if (done) {
              if (runAbortRef.current === controller) {
                runAbortRef.current = null;
              }
              setRunning(false);
              return;
            }

            buffer += decoder.decode(value, { stream: true });
            const parts = buffer.split("\n\n");
            buffer = parts.pop() || "";

            parts.forEach(part => {
              const eventMatch = part.match(/^event: (.+)/m);
              const dataMatch = part.match(/^data: (.+)/m);
              if (!eventMatch || !dataMatch) {
                return;
              }
              try {
                handleStreamEvent(eventMatch[1].trim(), JSON.parse(dataMatch[1]));
              } catch (error) {
                console.warn("Skipping malformed SSE payload", error);
              }
            });

            pump();
          }).catch(error => {
            if (error?.name !== "AbortError") {
              console.warn("RepoReaper run stream ended unexpectedly", error);
            }
            if (runAbortRef.current === controller) {
              runAbortRef.current = null;
            }
            setRunning(false);
          });

        pump();
      })
      .catch(error => {
        if (error?.name !== "AbortError") {
          console.warn("RepoReaper run request failed", error);
        }
        if (runAbortRef.current === controller) {
          runAbortRef.current = null;
        }
        setRunning(false);
      });
  }, [running, params, fetch_, handleStreamEvent]);

  return {
    ...auth,
    apiKey,
    checked,
    needsAuth,
    tab,
    setTab,
    agents,
    logs,
    issues,
    running,
    phase,
    cooldowns,
    runCost,
    lifetimeCost,
    viewDiff,
    setViewDiff,
    runStats,
    params,
    setParams,
    existingCfg,
    watchMode,
    refreshConfig,
    addAgent,
    removeAgent,
    loadPreset,
    toggleWatchMode,
    startRun,
    hasCooldown: Object.keys(cooldowns).length > 0,
  };
}

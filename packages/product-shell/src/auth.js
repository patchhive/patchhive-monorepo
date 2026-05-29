import { useEffect, useMemo, useState } from "react";

const apiFetcherCache = new Map();

export function createApiFetcher(apiKey) {
  const cacheKey = apiKey || "";
  if (!apiFetcherCache.has(cacheKey)) {
    apiFetcherCache.set(cacheKey, (url, opts = {}) =>
      fetch(url, {
        ...opts,
        headers: {
          ...(opts.headers || {}),
          ...(apiKey ? { "X-API-Key": apiKey } : {}),
        },
      }),
    );
  }
  return apiFetcherCache.get(cacheKey);
}

export function useApiFetcher(apiKey) {
  return useMemo(() => createApiFetcher(apiKey), [apiKey]);
}

function sessionStore() {
  if (typeof window === "undefined") return null;
  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
}

function readStoredKey(storageKey) {
  return sessionStore()?.getItem(storageKey) || "";
}

function writeStoredKey(storageKey, value) {
  sessionStore()?.setItem(storageKey, value);
}

function clearStoredKey(storageKey) {
  sessionStore()?.removeItem(storageKey);
}

export function useApiKeyAuth({ apiBase, storageKey }) {
  const [apiKey, setApiKey] = useState(() => readStoredKey(storageKey));
  const [checked, setChecked] = useState(false);
  const [needsAuth, setNeedsAuth] = useState(false);
  const [authError, setAuthError] = useState("");
  const [bootstrapRequired, setBootstrapRequired] = useState(false);

  useEffect(() => {
    setChecked(false);
    setAuthError("");
    fetch(`${apiBase}/auth/status`)
      .then((res) => res.json())
      .then((data) => {
        if (!data.auth_enabled && !data.bootstrap_required) {
          setApiKey("");
          setNeedsAuth(false);
          setBootstrapRequired(false);
          setChecked(true);
          return;
        }

        if (data.bootstrap_required) {
          clearStoredKey(storageKey);
          setApiKey("");
          setNeedsAuth(true);
          setBootstrapRequired(true);
          setChecked(true);
          return;
        }

        setBootstrapRequired(false);
        const stored = readStoredKey(storageKey);
        if (!stored) {
          setApiKey("");
          setNeedsAuth(true);
          setChecked(true);
          return;
        }

        fetch(`${apiBase}/auth/login`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ api_key: stored }),
        })
          .then((res) => {
            if (res.ok) {
              setApiKey(stored);
              setNeedsAuth(false);
              setAuthError("");
            } else {
              setApiKey("");
              setNeedsAuth(true);
              clearStoredKey(storageKey);
            }
            setChecked(true);
          })
          .catch(() => {
            setApiKey("");
            setNeedsAuth(true);
            setAuthError("Cannot verify the stored API key with the backend.");
            clearStoredKey(storageKey);
            setChecked(true);
          });
      })
      .catch(() => {
        setApiKey("");
        setNeedsAuth(true);
        setBootstrapRequired(false);
        setAuthError("Cannot reach the backend.");
        setChecked(true);
      });
  }, [apiBase, storageKey]);

  const login = (key) => {
    writeStoredKey(storageKey, key);
    setApiKey(key);
    setNeedsAuth(false);
    setBootstrapRequired(false);
    setAuthError("");
  };

  const logout = () => {
    clearStoredKey(storageKey);
    setApiKey("");
    setNeedsAuth(true);
    setAuthError("");
  };

  const generateKey = async ({ autoLogin = true } = {}) => {
    const res = await fetch(`${apiBase}/auth/generate-key`, { method: "POST" });
    const data = await res.json().catch(() => ({}));
    if (!res.ok || !data.api_key) {
      throw new Error(data.error || "Could not generate an API key.");
    }
    if (autoLogin) {
      login(data.api_key);
    }
    return data.api_key;
  };

  return {
    apiKey,
    checked,
    needsAuth,
    login,
    logout,
    authError,
    bootstrapRequired,
    generateKey,
  };
}

async function fetchRuntimeJson(fetcher, url) {
  const res = await fetcher(url);
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw new Error(data.error || data.message || `Request failed: ${res.status}`);
  }
  return data;
}

export function useProductRuntime({ apiBase, fetcher, ready }) {
  const [runtime, setRuntime] = useState({
    authStatus: null,
    capabilities: null,
    checks: [],
    error: "",
    health: null,
    latestRun: null,
    loading: false,
    runs: [],
  });

  const refresh = useMemo(() => async () => {
    if (!ready || !fetcher) {
      return;
    }

    setRuntime((current) => ({ ...current, error: "", loading: true }));
    const [health, checks, authStatus, capabilities, runs] = await Promise.allSettled([
      fetchRuntimeJson(fetcher, `${apiBase}/health`),
      fetchRuntimeJson(fetcher, `${apiBase}/startup/checks`),
      fetchRuntimeJson(fetcher, `${apiBase}/auth/status`),
      fetchRuntimeJson(fetcher, `${apiBase}/capabilities`),
      fetchRuntimeJson(fetcher, `${apiBase}/runs`),
    ]);

    const nextRuns = runs.status === "fulfilled" ? runs.value.runs || [] : [];
    setRuntime({
      authStatus: authStatus.status === "fulfilled" ? authStatus.value : null,
      capabilities: capabilities.status === "fulfilled" ? capabilities.value : null,
      checks: checks.status === "fulfilled" ? checks.value.checks || [] : [],
      error: health.status === "rejected" ? health.reason?.message || "Could not reach product backend." : "",
      health: health.status === "fulfilled" ? health.value : null,
      latestRun: nextRuns[0] || null,
      loading: false,
      runs: nextRuns,
    });
  }, [apiBase, fetcher, ready]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return {
    ...runtime,
    refresh,
  };
}

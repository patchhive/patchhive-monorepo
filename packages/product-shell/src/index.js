import { useEffect, useState } from "react";

export function createApiFetcher(apiKey) {
  return (url, opts = {}) =>
    fetch(url, {
      ...opts,
      headers: {
        ...(opts.headers || {}),
        ...(apiKey ? { "X-API-Key": apiKey } : {}),
      },
    });
}

export function useApiKeyAuth({ apiBase, storageKey }) {
  const [apiKey, setApiKey] = useState(() => localStorage.getItem(storageKey) || "");
  const [checked, setChecked] = useState(false);
  const [needsAuth, setNeedsAuth] = useState(false);

  useEffect(() => {
    fetch(`${apiBase}/auth/status`)
      .then((res) => res.json())
      .then((data) => {
        if (!data.auth_enabled) {
          setApiKey("");
          setNeedsAuth(false);
          setChecked(true);
          return;
        }

        const stored = localStorage.getItem(storageKey);
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
        }).then((res) => {
          if (res.ok) {
            setApiKey(stored);
            setNeedsAuth(false);
          } else {
            setApiKey("");
            setNeedsAuth(true);
          }
          setChecked(true);
        });
      })
      .catch(() => {
        setNeedsAuth(false);
        setChecked(true);
      });
  }, [apiBase, storageKey]);

  const login = (key) => {
    localStorage.setItem(storageKey, key);
    setApiKey(key);
    setNeedsAuth(false);
  };

  const logout = () => {
    localStorage.removeItem(storageKey);
    setApiKey("");
    setNeedsAuth(true);
  };

  return { apiKey, checked, needsAuth, login, logout };
}

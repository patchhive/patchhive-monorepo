import { useEffect, useState, useCallback } from "react";

const THEME_KEY = "vt.theme";

function readTheme(): boolean {
  if (typeof window === "undefined") return false;
  const stored = window.localStorage.getItem(THEME_KEY);
  if (stored === "dark") return true;
  if (stored === "light") return false;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
}

function applyDarkClass(dark: boolean) {
  if (typeof document === "undefined") return;
  document.documentElement.classList.toggle("dark", dark);
}

/**
 * Shared theme hook. Persists user choice in localStorage, follows OS
 * `prefers-color-scheme` when the user hasn't picked explicitly, and syncs
 * across browser tabs via the `storage` event. Applies `.dark` on
 * `<html>` so tokens cascade to every route.
 * SSR-safe: reads storage inside useEffect so hydration matches server output.
 */
export function useTheme() {
  const [dark, setDark] = useState(false);

  useEffect(() => {
    const initial = readTheme();
    setDark(initial);
    applyDarkClass(initial);

    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onOsChange = (e: MediaQueryListEvent) => {
      if (!window.localStorage.getItem(THEME_KEY)) setDark(e.matches);
    };
    const onStorage = (e: StorageEvent) => {
      if (e.key !== THEME_KEY && e.key !== null) return;
      setDark(readTheme());
    };
    mq.addEventListener("change", onOsChange);
    window.addEventListener("storage", onStorage);
    return () => {
      mq.removeEventListener("change", onOsChange);
      window.removeEventListener("storage", onStorage);
    };
  }, []);

  // Keep the DOM class in sync with state on every change (toggle, OS, tabs).
  useEffect(() => {
    applyDarkClass(dark);
  }, [dark]);

  const toggle = useCallback(() => {
    setDark((v) => {
      const next = !v;
      window.localStorage.setItem(THEME_KEY, next ? "dark" : "light");
      return next;
    });
  }, []);

  return { dark, toggle };
}
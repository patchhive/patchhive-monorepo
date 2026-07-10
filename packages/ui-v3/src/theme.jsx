import { useCallback, useEffect, useState } from "react";

export const PATCHHIVE_THEME_KEY = "patchhive.theme";

function storage() {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function readPatchHiveTheme() {
  const stored = storage()?.getItem(PATCHHIVE_THEME_KEY);
  if (stored === "dark" || stored === "light") return stored;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function applyPatchHiveTheme(theme) {
  if (typeof document === "undefined") return;
  document.documentElement.classList.toggle("dark", theme === "dark");
  document.documentElement.style.colorScheme = theme;
}

export function usePatchHiveTheme() {
  const [theme, setTheme] = useState(() => (
    typeof window === "undefined" ? "light" : readPatchHiveTheme()
  ));

  useEffect(() => {
    applyPatchHiveTheme(theme);
  }, [theme]);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onSystemTheme = (event) => {
      if (!storage()?.getItem(PATCHHIVE_THEME_KEY)) {
        setTheme(event.matches ? "dark" : "light");
      }
    };
    const onStorage = (event) => {
      if (event.key === PATCHHIVE_THEME_KEY || event.key === null) {
        setTheme(readPatchHiveTheme());
      }
    };
    media.addEventListener("change", onSystemTheme);
    window.addEventListener("storage", onStorage);
    return () => {
      media.removeEventListener("change", onSystemTheme);
      window.removeEventListener("storage", onStorage);
    };
  }, []);

  const toggleTheme = useCallback(() => {
    setTheme((current) => {
      const next = current === "dark" ? "light" : "dark";
      storage()?.setItem(PATCHHIVE_THEME_KEY, next);
      applyPatchHiveTheme(next);
      return next;
    });
  }, []);

  return { dark: theme === "dark", theme, toggleTheme };
}

export const PATCHHIVE_THEME_BOOTSTRAP = `(function(){try{var k='${PATCHHIVE_THEME_KEY}',v=localStorage.getItem(k),d=v==='dark'||(!v&&matchMedia('(prefers-color-scheme: dark)').matches);document.documentElement.classList.toggle('dark',d);document.documentElement.style.colorScheme=d?'dark':'light'}catch(e){}})();`;

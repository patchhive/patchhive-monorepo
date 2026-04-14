const defaultApiBase =
  typeof window !== "undefined" && !import.meta.env.DEV && window.location?.origin
    ? `${window.location.origin}/api`
    : "";

export const API = import.meta.env.VITE_API_URL || defaultApiBase;

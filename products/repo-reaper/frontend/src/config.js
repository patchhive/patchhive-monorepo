const defaultApiBase =
  typeof window !== "undefined" && window.location?.origin
    ? window.location.origin
    : "";

export const API = import.meta.env.VITE_API_URL || defaultApiBase;

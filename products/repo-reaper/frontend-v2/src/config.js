const defaultApiBase =
  typeof window !== "undefined"
    ? "/api/products/repo-reaper"
    : "http://127.0.0.1:8120/api/products/repo-reaper";

export const API = import.meta.env.VITE_API_URL || defaultApiBase;

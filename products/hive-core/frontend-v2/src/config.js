const defaultApiBase = typeof window !== "undefined" ? "/api" : "http://127.0.0.1:8100";

export const API = import.meta.env.VITE_API_URL || defaultApiBase;

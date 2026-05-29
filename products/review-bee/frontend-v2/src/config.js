const defaultApiBase = typeof window !== "undefined" ? "/api" : "http://127.0.0.1:8040";

export const API = import.meta.env.VITE_API_URL || defaultApiBase;

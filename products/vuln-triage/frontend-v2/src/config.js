const defaultApiBase = typeof window !== "undefined" ? "/api" : "http://127.0.0.1:8110";

export const API = import.meta.env.VITE_API_URL || defaultApiBase;

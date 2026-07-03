const defaultApiBase = "http://127.0.0.1:8100/api/products/release-sentry";

export const API = import.meta.env.VITE_API_URL || defaultApiBase;

// Single source of truth for where the control plane lives.
// Nothing else in this app may read env directly, and nothing but this URL is
// configurable from the browser — no secrets ship to the client.
export const API: string = import.meta.env.VITE_API_URL || "http://localhost:8100";

export const PRODUCT_KEY = "hive-core";
export const API_KEY_STORAGE = "hive-core_api_key";

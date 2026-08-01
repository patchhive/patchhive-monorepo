// Single fetch wrapper for the control plane.
//
// Every protected suite route requires the operator key, and attaching it at each
// call site is how one gets forgotten. This is the only place the deck reads the
// in-memory key, and the only place it is attached to a request.
//
// The key goes to VITE_API_URL and nowhere else — no cookie, no third-party origin.

import { API, API_KEY_STORAGE } from "@/config";

let apiKey = "";

function clearLegacyStoredKey(): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.removeItem(API_KEY_STORAGE);
  } catch {
    // Storage can be unavailable by browser policy. The active key is memory-only.
  }
  try {
    window.sessionStorage.removeItem(API_KEY_STORAGE);
  } catch {
    // Best-effort cleanup of the short-lived storage used by earlier builds.
  }
}

export function readApiKey(): string {
  clearLegacyStoredKey();
  return apiKey;
}

export function storeApiKey(value: string): void {
  clearLegacyStoredKey();
  apiKey = value.trim();
}

export function clearApiKey(): void {
  apiKey = "";
  clearLegacyStoredKey();
}

/** Fetch a control-plane path with the operator key attached. */
export function apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const headers = new Headers(init.headers);
  const key = readApiKey();
  if (key) headers.set("X-API-Key", key);
  if (init.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
  return fetch(`${API}${path}`, { ...init, headers });
}

/** Fetch and parse JSON, throwing with the server's message when it fails. */
export async function apiJson<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await apiFetch(path, init);
  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as { message?: string } | null;
    throw new Error(payload?.message ?? `HTTP ${response.status} from ${path}`);
  }
  return (await response.json()) as T;
}

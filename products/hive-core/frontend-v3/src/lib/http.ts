// Single fetch wrapper for the control plane.
//
// Every protected suite route requires the operator key, and attaching it at each
// call site is how one gets forgotten. This is the only place the deck reads the
// stored key, and the only place it is attached to a request.
//
// The key goes to VITE_API_URL and nowhere else — no cookie, no third-party origin.

import { API, API_KEY_STORAGE } from "@/config";

export function readApiKey(): string {
  if (typeof window === "undefined") return "";
  return window.localStorage.getItem(API_KEY_STORAGE) ?? "";
}

export function clearApiKey(): void {
  if (typeof window !== "undefined") window.localStorage.removeItem(API_KEY_STORAGE);
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

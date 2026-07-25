// HiveCore API client.
//
// The operator key lives in localStorage and travels as X-API-Key, exactly as the
// other PatchHive frontends do. It is never sent to any origin but VITE_API_URL, and
// there is no server-side session — nothing between the browser and the Rust backend
// holds a credential.

import { API, API_KEY_STORAGE } from "@/config";

export interface ApiMeta {
  product: string;
  version: string;
  request_id: string;
  timestamp: string;
}

export interface ApiError {
  code: string;
  message: string;
  retryable: boolean;
  details: unknown;
}

export interface ApiEnvelope<T> {
  status: "ok" | "error";
  data: T | null;
  error: ApiError | null;
  meta: ApiMeta;
}

export function readApiKey(): string {
  if (typeof window === "undefined") return "";
  return window.localStorage.getItem(API_KEY_STORAGE) ?? "";
}

export function writeApiKey(key: string): void {
  window.localStorage.setItem(API_KEY_STORAGE, key);
}

export function clearApiKey(): void {
  window.localStorage.removeItem(API_KEY_STORAGE);
}

export class HiveApiError extends Error {
  readonly code: string;
  readonly retryable: boolean;
  readonly status: number;

  constructor(message: string, code: string, retryable: boolean, status: number) {
    super(message);
    this.name = "HiveApiError";
    this.code = code;
    this.retryable = retryable;
    this.status = status;
  }
}

/** Unwraps the contract-v1 envelope and throws a typed error on failure. */
export async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const key = readApiKey();
  const headers = new Headers(init.headers);
  headers.set("Accept", "application/json");
  if (init.body) headers.set("Content-Type", "application/json");
  if (key) headers.set("X-API-Key", key);

  let response: Response;
  try {
    response = await fetch(`${API}${path}`, { ...init, headers });
  } catch (cause) {
    throw new HiveApiError(
      `Could not reach HiveCore at ${API}.`,
      "control_plane_unreachable",
      true,
      0,
    );
  }

  const payload = (await response.json().catch(() => null)) as ApiEnvelope<T> | null;

  if (!response.ok || payload?.status === "error") {
    throw new HiveApiError(
      payload?.error?.message ?? `HiveCore returned HTTP ${response.status}.`,
      payload?.error?.code ?? "request_failed",
      payload?.error?.retryable ?? false,
      response.status,
    );
  }

  return (payload?.data ?? (payload as unknown)) as T;
}

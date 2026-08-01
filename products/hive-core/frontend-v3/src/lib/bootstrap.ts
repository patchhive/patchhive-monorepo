// Suite bootstrap: bringing the suite up from nothing.
//
// The backend for this has existed for a while — launcher status, per-product
// credential requirements, pairing, fleet launch, smoke tiers — and the v3 deck never
// surfaced any of it. This is the client for those endpoints.
//
// The one rule the whole surface turns on: the browser never writes a `.env` file and
// never controls Docker. HiveCore plans and validates; `patchhive-launcher` performs
// host-level work, localhost-only. Secrets typed here go to HiveCore, which forwards
// approved writes to the launcher — they are never persisted in the browser and never
// come back on a read.

import { apiFetch } from "./http";

const BASE = "/api/products/hive-core";

export interface LauncherStatus {
  available: boolean;
  message: string;
  repo_root: string;
  docker_available: boolean;
  docker_compose_available: boolean;
  image_mode?: string;
  image_tag?: string;
}

export interface CredentialRequirement {
  key: string;
  label: string;
  kind: string;
  profile: string;
  required: boolean;
  redact: boolean;
  configured: boolean;
  /** True when the value present is a placeholder rather than a real credential. */
  placeholder: boolean;
  status: string;
  message: string;
  description: string;
}

export interface SetupProduct {
  runtime: {
    slug: string;
    title: string;
    status: string;
    enabled: boolean;
    api_url: string;
  };
  auth_status_error: string;
  pairing_ready: boolean;
  credentials: CredentialRequirement[];
  /**
   * Whether `credentials` is an answer or an absence. Empty-and-known means nothing
   * left to configure; empty-and-unknown means HiveCore could not reach the launcher
   * to ask. Rendering those the same way tells an operator setup is complete when it
   * is merely unmeasured.
   */
  credentials_known: boolean;
}

export interface SmokeStep {
  slug: string;
  title: string;
  check: string;
  status: string;
  message: string;
  remote_status: number | null;
  evidence: unknown;
}

export interface SmokeRun {
  id: string;
  tier: string;
  status: string;
  started_at: string;
  finished_at: string;
  summary: string;
  steps: SmokeStep[];
}

export interface BootstrapState {
  stack_id: string;
  launcher: LauncherStatus;
  requirements_known: boolean;
  requirements_error: string;
  suite_bootstrap_configured: boolean;
  latest_smoke: SmokeRun | null;
  actions: string[];
  products: SetupProduct[];
}

interface Envelope<T> {
  data?: T;
  error?: { message?: string };
}

export async function fetchBootstrap(signal?: AbortSignal): Promise<BootstrapState> {
  const response = await apiFetch(`${BASE}/setup/first-stack`, { signal });
  if (!response.ok) throw new Error(`HTTP ${response.status} from /setup/first-stack`);
  const body = (await response.json()) as Envelope<BootstrapState>;
  if (!body.data) throw new Error("Control plane returned no setup state.");
  return body.data;
}

async function post<T>(path: string, payload?: unknown): Promise<{ data: T | null; message: string }> {
  try {
    const response = await apiFetch(`${BASE}${path}`, {
      method: "POST",
      body: payload === undefined ? undefined : JSON.stringify(payload),
    });
    const body = (await response.json().catch(() => null)) as Envelope<T> | null;
    if (!response.ok) {
      return {
        data: null,
        message: body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`,
      };
    }
    return { data: body?.data ?? null, message: "" };
  } catch {
    return { data: null, message: "Could not reach the control plane." };
  }
}

/**
 * Save credentials for one product.
 *
 * Values go to HiveCore and on to the launcher, which owns the file. Nothing is kept
 * in the browser: the form clears on success and a re-read returns configured/not,
 * never the value.
 */
export function saveProductEnv(
  slug: string,
  values: Record<string, string>,
): Promise<{ data: unknown; message: string }> {
  return post(`/setup/products/${slug}/env`, { values });
}

/** Validate a GitHub token before it is written anywhere. */
export function validateGitHubToken(
  token: string,
  profile: string,
): Promise<{ data: { valid?: boolean; login?: string; scopes?: string[]; message?: string } | null; message: string }> {
  return post(`/setup/credentials/github/validate`, { token, profile });
}

/** Mint scoped service tokens for products that are up but unpaired. */
export function pairFirstStack(): Promise<{ data: BootstrapState | null; message: string }> {
  return post(`/setup/first-stack/pair`);
}

export function startFirstStack(): Promise<{ data: BootstrapState | null; message: string }> {
  return post(`/setup/first-stack/start`);
}

export function runSmoke(tier: string): Promise<{ data: BootstrapState | null; message: string }> {
  return post(`/setup/smoke/${tier}`);
}

export const SMOKE_TIERS = [
  { slug: "first-stack", label: "First stack" },
  { slug: "read-only-fleet", label: "Read-only fleet" },
  { slug: "write-dry-run", label: "Write dry-run" },
  { slug: "release-gate", label: "Release gate" },
] as const;

export const STATUS_TONE: Record<string, string> = {
  ok: "text-[var(--ok)] border-[var(--ok)]/40",
  pass: "text-[var(--ok)] border-[var(--ok)]/40",
  warn: "text-[var(--warn)] border-[var(--warn)]/40",
  fail: "text-[var(--crit)] border-[var(--crit)]/40",
  error: "text-[var(--crit)] border-[var(--crit)]/40",
  skipped: "text-muted-foreground border-border",
  unknown: "text-muted-foreground border-border",
};

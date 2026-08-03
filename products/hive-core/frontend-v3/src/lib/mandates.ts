import { apiFetch } from "./http";
import type { ProposedDispatch } from "./work-ledger";

export type MandateAutonomy = "observe" | "propose" | "act_with_approval" | "act";

export interface MandateScope {
  search_query: string;
  topics: string[];
  languages: string[];
  min_stars: number;
  max_repositories: number;
  issues_per_repository: number;
  stale_days: number;
}

export interface MandateLimits {
  pr_budget: number;
  cost_budget_cents_per_day: number;
  per_owner_open_prs: number;
  cooldown_after_close_days: number;
}

export interface MandateConfig {
  name: string;
  objective: string;
  scope: MandateScope;
  requested_autonomy: MandateAutonomy;
  limits: MandateLimits;
}

export type MandateLifecycle =
  | { state: "active"; activated_at: string }
  | { state: "paused"; paused_at: string; reason: string }
  | { state: "archived"; archived_at: string; reason: string }
  | { state: "unknown"; raw_state: string; raw_evidence: unknown };

export interface MandateRecord {
  id: string;
  config: MandateConfig;
  lifecycle: MandateLifecycle;
  revision: number;
  created_at: string;
  updated_at: string;
}

export type ConductorDecision =
  | { decision: "deferred"; mandate_id: string; reason: string }
  | { decision: "observed_only"; mandate_id: string; requested_autonomy: MandateAutonomy; reason: string }
  | {
      decision: "planned_discovery";
      mandate_id: string;
      requested_autonomy: MandateAutonomy;
      effective_autonomy: MandateAutonomy;
      proposed_dispatch: ProposedDispatch;
      rationale: string;
    };

export type ConductorTickLifecycle =
  | { state: "running"; started_at: string; lease_until: string }
  | { state: "completed"; started_at: string; finished_at: string; decisions: ConductorDecision[]; remaining_active_mandates: number }
  | { state: "failed"; started_at: string; failed_at: string; reason: string }
  | { state: "unknown"; raw_state: string; raw_evidence: unknown };

export interface ConductorTickRecord {
  id: string;
  trigger: "operator" | "background";
  lifecycle: ConductorTickLifecycle;
  created_at: string;
  updated_at: string;
}

export type RunTickOutcome =
  | { outcome: "settled"; tick: ConductorTickRecord }
  | { outcome: "busy"; active_tick_id: string; lease_until: string };

interface Envelope<T> {
  data?: T;
  error?: { message?: string };
}

const BASE = "/api/products/hive-core";

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await apiFetch(`${BASE}${path}`, init);
  const body = (await response.json().catch(() => null)) as Envelope<T> | null;
  if (!response.ok) throw new Error(body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`);
  if (body?.data === undefined) throw new Error("HiveCore returned no conductor data.");
  return body.data;
}

export function fetchMandates(signal?: AbortSignal): Promise<MandateRecord[]> {
  return request<MandateRecord[]>("/mandates", { signal });
}

export function createMandate(config: MandateConfig): Promise<MandateRecord> {
  return request<MandateRecord>("/mandates", { method: "POST", body: JSON.stringify(config) });
}

export function changeMandateState(id: string, action: "activate" | "pause" | "archive", reason = ""): Promise<MandateRecord> {
  return request<MandateRecord>(`/mandates/${encodeURIComponent(id)}/${action}`, {
    method: "POST",
    body: action === "activate" ? undefined : JSON.stringify({ reason }),
  });
}

export function fetchConductorTicks(signal?: AbortSignal): Promise<ConductorTickRecord[]> {
  return request<ConductorTickRecord[]>("/conductor/ticks", { signal });
}

export function runConductorTick(): Promise<RunTickOutcome> {
  return request<RunTickOutcome>("/conductor/ticks", { method: "POST" });
}

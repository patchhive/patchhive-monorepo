import { apiFetch } from "./http";

export type WorkOrigin =
  | { origin: "operator" }
  | { origin: "product_run"; product_slug: string; run_id: string }
  | { origin: "suite_run"; run_id: string }
  | { origin: "conductor_tick"; tick_id: string };

export interface WorkIdentity {
  kind: string;
  repository: string;
  subject_ref: string;
}

export interface ProposedDispatch {
  product_slug: string;
  action_id: string;
  input: Record<string, unknown>;
}

export interface WorkProposal {
  mandate_id?: string;
  identity: WorkIdentity;
  proposed_dispatch: ProposedDispatch;
  origin: WorkOrigin;
  rationale: string;
}

export type WorkLifecycle =
  | { state: "discovered"; discovered_at: string }
  | { state: "dispatching"; claim_id: string; started_at: string; lease_until: string }
  | { state: "awaiting_approval"; approval_id: string; requested_at: string }
  | { state: "gated"; gate_product: string; gate_run_id: string; recommendation: string; gated_at: string }
  | { state: "dispatched"; action_event_id: string; receiving_run_id?: string; dispatched_at: string }
  | { state: "shipped"; pr_url: string; shipped_at: string }
  | { state: "completed"; outcome: string; completed_at: string }
  | { state: "blocked"; reason: string; blocked_at: string; retryable: boolean; next_attempt_at?: string }
  | { state: "failed"; reason: string; failed_at: string; retryable: boolean; next_attempt_at?: string }
  | { state: "abandoned"; reason: string; abandoned_at: string }
  | { state: "unknown"; raw_state: string; raw_evidence: unknown };

export interface WorkItem {
  id: string;
  fingerprint: string;
  proposal: WorkProposal;
  lifecycle: WorkLifecycle;
  attempts: number;
  created_at: string;
  updated_at: string;
}

export interface ProductFinding {
  mandate_id?: string;
  source: {
    product_slug: string;
    run_id: string;
    finding_id: string;
  };
  identity: WorkIdentity;
  proposed_dispatch: ProposedDispatch;
  rationale: string;
  evidence: Record<string, unknown>;
}

export interface FindingReceipt {
  finding: ProductFinding;
  work_item_id: string;
  work_fingerprint: string;
  finding_fingerprint: string;
  ingested_at: string;
}

export interface WorkHandoffEdge {
  from_product: string;
  to_product: string;
  work_items: number;
  active_work_items: number;
  last_observed_at: string;
}

interface Envelope<T> {
  data?: T;
  error?: { message?: string };
}

export async function fetchWorkItems(signal?: AbortSignal): Promise<WorkItem[]> {
  const path = "/api/products/hive-core/work-items";
  const response = await apiFetch(path, { signal });
  const body = (await response.json().catch(() => null)) as Envelope<WorkItem[]> | null;
  if (!response.ok) {
    throw new Error(body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`);
  }
  if (!body?.data) throw new Error("HiveCore returned no work-ledger data.");
  return body.data;
}

export async function fetchFindingReceipts(signal?: AbortSignal): Promise<FindingReceipt[]> {
  const path = "/api/products/hive-core/work-items/findings";
  const response = await apiFetch(path, { signal });
  const body = (await response.json().catch(() => null)) as Envelope<FindingReceipt[]> | null;
  if (!response.ok) {
    throw new Error(body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`);
  }
  if (!body?.data) throw new Error("HiveCore returned no finding-receipt data.");
  return body.data;
}

export async function fetchLiveBlastRadius(slug: string, signal?: AbortSignal): Promise<WorkHandoffEdge[]> {
  const response = await apiFetch(`/api/products/hive-core/blast-radius/${encodeURIComponent(slug)}`, { signal });
  const body = (await response.json().catch(() => null)) as Envelope<WorkHandoffEdge[]> | null;
  if (!response.ok) {
    throw new Error(body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`);
  }
  if (!body?.data) throw new Error("HiveCore returned no blast-radius evidence.");
  return body.data;
}

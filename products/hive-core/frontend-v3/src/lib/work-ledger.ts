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

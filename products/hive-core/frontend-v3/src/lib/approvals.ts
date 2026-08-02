import { apiFetch } from "./http";
import type { ActionEffect } from "./dispatch";

export type ApprovalOrigin =
  | { origin: "operator_dispatch" }
  | { origin: "suite_run"; run_id: string };

export interface ApprovalSubject {
  fingerprint: string;
  product: string;
  action_id: string;
  action_label: string;
  repository?: string;
  run_id?: string;
  input_hash: string;
  effect: ActionEffect;
  required_scopes: string[];
  origin: ApprovalOrigin;
}

export interface ApprovalDispatch {
  payload: unknown;
  path_params: Record<string, string>;
  query: Record<string, string>;
}

export type ApprovalLifecycle =
  | { state: "pending"; expires_at: string }
  | { state: "granted"; granted_at: string; expires_at: string }
  | { state: "denied"; denied_at: string; reason: string }
  | { state: "revoked"; revoked_at: string; reason: string }
  | { state: "consuming"; claimed_at: string }
  | {
      state: "consumed";
      claimed_at: string;
      consumed_at: string;
      event_id: string;
      outcome: Record<string, unknown>;
    }
  | { state: "expired"; expired_at: string; previous: "pending" | "granted" }
  | { state: "unknown"; raw_state: string; raw_evidence: unknown };

export interface ApprovalEvent {
  id: number;
  approval_id: string;
  event: string;
  reason: string;
  created_at: string;
}

export interface ApprovalRecord {
  id: string;
  subject: ApprovalSubject;
  dispatch: ApprovalDispatch;
  lifecycle: ApprovalLifecycle;
  created_at: string;
  updated_at: string;
  history: ApprovalEvent[];
}

interface Envelope<T> {
  data?: T;
  error?: { message?: string };
}

const BASE = "/api/products/hive-core/approvals";

async function approvalRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await apiFetch(`${BASE}${path}`, init);
  const body = (await response.json().catch(() => null)) as Envelope<T> | null;
  if (!response.ok) {
    throw new Error(body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`);
  }
  if (body?.data === undefined) throw new Error("HiveCore returned no approval data.");
  return body.data;
}

export function fetchApprovals(signal?: AbortSignal): Promise<ApprovalRecord[]> {
  return approvalRequest<ApprovalRecord[]>("", { signal });
}

export function grantApproval(id: string): Promise<ApprovalRecord> {
  return approvalRequest<ApprovalRecord>(`/${id}/grant`, { method: "POST" });
}

export function denyApproval(id: string, reason: string): Promise<ApprovalRecord> {
  return approvalRequest<ApprovalRecord>(`/${id}/deny`, {
    method: "POST",
    body: JSON.stringify({ reason }),
  });
}

export function revokeApproval(id: string, reason: string): Promise<ApprovalRecord> {
  return approvalRequest<ApprovalRecord>(`/${id}/revoke`, {
    method: "POST",
    body: JSON.stringify({ reason }),
  });
}

export function dispatchApproved(id: string): Promise<unknown> {
  return approvalRequest<unknown>(`/${id}/dispatch`, { method: "POST" });
}

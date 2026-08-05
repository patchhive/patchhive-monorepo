import { apiFetch } from "./http";

export type EngagementLifecycle =
  | { state: "awaiting_operator"; reason: string; classified_at: string }
  | { state: "no_response"; reason: string; decided_at: string }
  | { state: "work_proposed"; work_item_id: string; proposed_at: string }
  | { state: "paused"; reason: string; paused_at: string }
  | { state: "quarantined"; reason: string; quarantined_at: string }
  | { state: "resolved"; reason: string; resolved_at: string }
  | { state: "unknown"; raw_state: string; raw_evidence: unknown };

export interface MaintainerEngagement {
  id: string;
  event_name: string;
  event_action: string;
  artifact_kind: "pull_request" | "issue";
  repository: string;
  artifact_number: number;
  artifact_url: string;
  owner_product: string;
  author_login: string;
  author_association: string;
  trust: "trusted_maintainer" | "untrusted_participant" | "unknown";
  body: string;
  intent: string;
  lifecycle: EngagementLifecycle;
  received_at: string;
  updated_at: string;
}

interface Envelope<T> {
  data?: T;
  error?: { message?: string };
}

async function envelope<T>(response: Response): Promise<T> {
  const payload = (await response.json().catch(() => null)) as Envelope<T> | null;
  if (!response.ok || !payload?.data) {
    throw new Error(payload?.error?.message ?? `HiveCore returned HTTP ${response.status}.`);
  }
  return payload.data;
}

export async function fetchEngagements(signal?: AbortSignal): Promise<MaintainerEngagement[]> {
  return envelope(await apiFetch("/api/products/hive-core/engagements", { signal }));
}

export type EngagementDecision =
  | { decision: "no_response" | "pause_repository" | "quarantine" | "resolve"; reason: string }
  | { decision: "queue_change"; reason: string }
  | { decision: "queue_reply"; body: string; reason: string };

export async function decideEngagement(
  id: string,
  decision: EngagementDecision,
): Promise<MaintainerEngagement> {
  return envelope(
    await apiFetch(`/api/products/hive-core/engagements/${encodeURIComponent(id)}/decision`, {
      method: "POST",
      body: JSON.stringify(decision),
    }),
  );
}

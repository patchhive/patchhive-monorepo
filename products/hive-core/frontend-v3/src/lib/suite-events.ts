// Durable suite events from the control plane.
//
// The change log previously held only in-session operator actions, which vanished
// on reload — so the one surface meant to answer "what happened" forgot everything
// the moment you refreshed. /api/events is the backend's persisted record and
// already exists; this reads it.
//
// The two sources stay visually distinct rather than being merged into one
// indistinguishable list: a durable server event and an unsaved local action are
// different kinds of fact, and collapsing them would imply the local ones survive.

import { apiFetch } from "./http";

export type SuiteEventTone = "info" | "warn" | "crit";

export interface SuiteEvent {
  id: string;
  kind: string;
  message: string;
  createdAt: string;
  tone: SuiteEventTone;
}

interface ApiEvent {
  id: string;
  kind: string;
  message: string;
  created_at: string;
}

interface LedgerEvent {
  id: string;
  entity_kind: string;
  entity_id: string;
  event_kind: string;
  evidence: Record<string, unknown>;
  created_at: string;
}

function toneFor(kind: string): SuiteEventTone {
  if (/denied|failed|error|revoked/.test(kind)) return "crit";
  if (/warn|blocked|expired|degraded/.test(kind)) return "warn";
  return "info";
}

/**
 * Collapse consecutive identical events.
 *
 * The backend records one `backend.started` per process start, and a restart loop
 * produces dozens of identical rows that bury everything else. Fifty-five copies of
 * one fact is one fact.
 */
export interface SuiteEventGroup {
  event: SuiteEvent;
  count: number;
  /** Oldest occurrence in the run, when count > 1. */
  since: string;
}

export function groupEvents(events: SuiteEvent[]): SuiteEventGroup[] {
  const groups: SuiteEventGroup[] = [];
  for (const event of events) {
    const previous = groups[groups.length - 1];
    if (previous && previous.event.kind === event.kind && previous.event.message === event.message) {
      previous.count += 1;
      previous.since = event.createdAt;
      continue;
    }
    groups.push({ event, count: 1, since: event.createdAt });
  }
  return groups;
}

export async function fetchSuiteEvents(signal?: AbortSignal): Promise<SuiteEvent[]> {
  const [systemResponse, ledgerResponse] = await Promise.all([
    apiFetch("/api/events", { signal }),
    apiFetch("/api/products/hive-core/events", { signal }),
  ]);
  if (!systemResponse.ok || !ledgerResponse.ok) {
    throw new Error(`HTTP ${systemResponse.status}/${ledgerResponse.status} from the suite ledgers`);
  }
  const rows = (await systemResponse.json()) as ApiEvent[];
  const ledgerBody = (await ledgerResponse.json()) as { data?: LedgerEvent[] };
  const durable = (ledgerBody.data ?? []).map((row) => ({
    id: row.id,
    kind: `${row.entity_kind}.${row.event_kind}`,
    message: ledgerMessage(row),
    created_at: row.created_at,
  }));
  return [...rows, ...durable]
    .map((row) => ({
      id: row.id,
      kind: row.kind,
      message: row.message,
      createdAt: row.created_at,
      tone: toneFor(row.kind),
    }))
    .sort((left, right) => (Date.parse(right.createdAt) || 0) - (Date.parse(left.createdAt) || 0));
}

function ledgerMessage(event: LedgerEvent): string {
  const reason = typeof event.evidence.reason === "string" ? event.evidence.reason : "";
  const repository = typeof event.evidence.repository === "string" ? ` · ${event.evidence.repository}` : "";
  return `${event.entity_kind} ${event.entity_id} · ${event.event_kind}${repository}${reason ? ` · ${reason}` : ""}`;
}

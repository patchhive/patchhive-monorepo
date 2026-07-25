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
  const response = await apiFetch("/api/events", { signal });
  if (!response.ok) throw new Error(`HTTP ${response.status} from /api/events`);
  const rows = (await response.json()) as ApiEvent[];
  return rows
    .map((row) => ({
      id: row.id,
      kind: row.kind,
      message: row.message,
      createdAt: row.created_at,
      tone: toneFor(row.kind),
    }))
    .sort((left, right) => (Date.parse(right.createdAt) || 0) - (Date.parse(left.createdAt) || 0));
}

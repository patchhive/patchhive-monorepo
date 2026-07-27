// Product runbooks: a recorded read-only diagnostic pass over one product.
//
// This replaces a drawer that faked it. The old steps ("Restart worker pool",
// "Force rotate leaking token", "Failover NVD feed to mirror") were a hardcoded
// array; running one slept for ~700ms per step, marked everything done, and wrote an
// audit entry of kind "destructive" claiming it had happened. A fabricated metric
// misleads; a fabricated audit trail of destructive operations corrupts the record
// you would consult to find out what was actually done.
//
// HiveCore now performs the checks for real and returns what it saw. There is no
// step for restarting a worker or failing over a feed, because HiveCore cannot do
// those — they are host operations belonging to the launcher. A runbook diagnoses;
// acting on a product is a dispatch or a suite run, where the approval, scope and
// credential guards live.

import { apiFetch } from "./http";

export interface RunbookStep {
  id: string;
  label: string;
  /** ok | warn | fail | skipped */
  status: string;
  message: string;
  remote_status: number | null;
  /** What the check actually observed, for an operator who wants to disagree with it. */
  evidence: unknown;
}

export interface RunbookRun {
  id: string;
  product_slug: string;
  product_title: string;
  /** ok | degraded | failed */
  status: string;
  started_at: string;
  finished_at: string;
  summary: string;
  steps: RunbookStep[];
}

const BASE = "/api/products/hive-core";

interface Envelope<T> {
  data?: T;
  error?: { message?: string };
}

/** Run the diagnostic for one product. Read-only; nothing is changed. */
export async function runProductRunbook(
  slug: string,
): Promise<{ run: RunbookRun | null; message: string }> {
  try {
    const response = await apiFetch(`${BASE}/products/${slug}/runbook`, { method: "POST" });
    const body = (await response.json().catch(() => null)) as Envelope<RunbookRun> | null;
    if (!response.ok) {
      return {
        run: null,
        message: body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`,
      };
    }
    const run = body?.data ?? null;
    return { run, message: run?.summary ?? "Runbook finished." };
  } catch {
    return { run: null, message: "Could not reach the control plane." };
  }
}

/**
 * Past runbook executions, from the control plane.
 *
 * History used to live in React state, which meant it did not survive a reload. A
 * history that forgets is not a history — this is meant to answer "who checked this
 * product, and what did it say" after the fact.
 */
export async function fetchRunbookRuns(signal?: AbortSignal): Promise<RunbookRun[]> {
  const response = await apiFetch(`${BASE}/runbooks`, { signal });
  if (!response.ok) throw new Error(`HTTP ${response.status} from ${BASE}/runbooks`);
  const body = (await response.json()) as Envelope<RunbookRun[]>;
  return body.data ?? [];
}

export const STEP_TONE: Record<string, string> = {
  ok: "text-[var(--ok)] border-[var(--ok)]/40",
  warn: "text-[var(--warn)] border-[var(--warn)]/40",
  fail: "text-[var(--crit)] border-[var(--crit)]/40",
  skipped: "text-muted-foreground border-border",
};

export const RUN_TONE: Record<string, string> = {
  ok: "text-[var(--ok)] border-[var(--ok)]/40",
  degraded: "text-[var(--warn)] border-[var(--warn)]/40",
  failed: "text-[var(--crit)] border-[var(--crit)]/40",
};

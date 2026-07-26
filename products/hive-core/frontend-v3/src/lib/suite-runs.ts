// Suite runs: an ordered sequence of dispatches, recorded as one unit.
//
// Executed by HiveCore, not the deck. Each step goes through the same dispatch path
// a manual action uses, so a suite run cannot reach anything an operator could not
// reach by hand — it sequences work, it does not widen authority.

import { apiFetch } from "./http";

export interface SuiteRunStep {
  product: string;
  action: string;
  /** queued | running | dispatched | failed | skipped */
  status: string;
  message: string;
  remote_status: number | null;
  /** The dispatch event this step produced, traceable to its evidence. */
  event_id: string;
  started_at: string;
  finished_at: string;
}

export interface SuiteRun {
  id: string;
  name: string;
  /** running | completed | failed | halted */
  status: string;
  started_at: string;
  finished_at: string;
  summary: string;
  steps: SuiteRunStep[];
}

export interface SuiteRunStepInput {
  product: string;
  action: string;
  payload?: unknown;
}

const BASE = "/api/products/hive-core/suite-runs";

interface Envelope<T> {
  data?: T;
  error?: { message?: string };
}

export async function fetchSuiteRuns(signal?: AbortSignal): Promise<SuiteRun[]> {
  const response = await apiFetch(BASE, { signal });
  if (!response.ok) throw new Error(`HTTP ${response.status} from ${BASE}`);
  const body = (await response.json()) as Envelope<SuiteRun[]>;
  return body.data ?? [];
}

export interface StartResult {
  ok: boolean;
  run: SuiteRun | null;
  message: string;
}

export async function startSuiteRun(
  name: string,
  steps: SuiteRunStepInput[],
  continueOnFailure: boolean,
): Promise<StartResult> {
  try {
    const response = await apiFetch(BASE, {
      method: "POST",
      body: JSON.stringify({ name, steps, continue_on_failure: continueOnFailure }),
    });
    const body = (await response.json().catch(() => null)) as Envelope<SuiteRun> | null;
    if (!response.ok) {
      return {
        ok: false,
        run: null,
        message: body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`,
      };
    }
    const run = body?.data ?? null;
    return {
      ok: run?.status === "completed",
      run,
      message: run?.summary ?? "Suite run finished.",
    };
  } catch {
    return { ok: false, run: null, message: "Could not reach the control plane." };
  }
}

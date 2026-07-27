// Suite runs: an ordered sequence of dispatches, recorded as one unit.
//
// Executed by HiveCore, not the deck. Each step goes through the same dispatch path
// a manual action uses, so a suite run cannot reach anything an operator could not
// reach by hand — it sequences work, it does not widen authority.

import { apiFetch } from "./http";

export interface SuiteRunStep {
  product: string;
  action: string;
  /** The payload actually dispatched, after any target substitution. */
  payload?: unknown;
  /** The target this step was expanded for; empty for ordinary steps. */
  target?: string;
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
  targets?: SuiteRunTargets;
}

/**
 * An explicit reference from one step to an earlier step's output.
 *
 * Explicit on purpose. HiveCore could infer that a scan produced repositories and
 * that the next step wants them, but inference here is a guess about what an operator
 * meant, applied to actions that reach real repositories. The operator names the step,
 * the path and the field; HiveCore resolves exactly that.
 *
 * `max_targets` is a request, not a limit — the server clamps it. A cap the browser
 * chooses is a cap the browser can raise.
 */
export interface SuiteRunTargets {
  /** 1-based index of an earlier step in the same run. */
  from_step: number;
  /** Dot path into that step's response body, resolving to an array. */
  path: string;
  /** Field to read from each element; empty means the element is the value. */
  field: string;
  /** Payload field to set on each expanded dispatch. */
  assign_to: string;
  max_targets: number;
}

/** The server's ceiling, mirrored so the composer cannot offer more than it will honour. */
export const MAX_TARGETS_PER_STEP = 25;

/**
 * Shapes products actually return, offered as starting points in the composer.
 *
 * These are suggestions, not detection: the operator confirms or edits them. Guessing
 * silently would make a mistyped path look like a step that legitimately found
 * nothing, which is exactly the failure the server refuses to paper over.
 */
export const TARGET_PRESETS: ReadonlyArray<{
  label: string;
  path: string;
  field: string;
  assign_to: string;
}> = [
  { label: "repos[].full_name", path: "repos", field: "full_name", assign_to: "repo" },
  { label: "data.repos[].full_name", path: "data.repos", field: "full_name", assign_to: "repo" },
  {
    label: "repositories[].full_name",
    path: "repositories",
    field: "full_name",
    assign_to: "repo",
  },
  { label: "results[].repo", path: "results", field: "repo", assign_to: "repo" },
  { label: "repos[] (strings)", path: "repos", field: "", assign_to: "repo" },
];

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

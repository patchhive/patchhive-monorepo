import type { RunEvent } from "./hive-data";

/**
 * Failure detail for a run.
 *
 * The contract-v1 run *summary* carries no failure payload — no error code, no
 * stage, no request id, no retry budget. The mock deck invented all of those, which
 * was harmless against fake runs and actively misleading against real ones: a real
 * RepoReaper failure would have been labelled UPSTREAM_TIMEOUT_504 with an 8000ms
 * budget it never had.
 *
 * So nothing is synthesised here. Only fields the run actually carries are
 * reported, and everything else is absent. Rich failure detail belongs behind
 * GET /runs/:id, which returns the product's own run events and artifacts —
 * wiring the drawer to that is the correct source, not guesswork in the browser.
 */
export interface FailureDetail {
  /** Present only when the product reported one. */
  code: string | null;
  stage: string | null;
  message: string;
  requestId: string;
  /** True when the product exposes a detail route for this run. */
  hasDetail: boolean;
}

export function deriveFailure(run: RunEvent): FailureDetail | null {
  if (run.status !== "failed") return null;
  return {
    code: null,
    stage: null,
    // The feed carries the product's own summary text; that is the only
    // description of the failure available at this level.
    message: `${run.product} reported a failed run for ${run.capability}.`,
    requestId: run.id,
    hasDetail: true,
  };
}

/** The run as the deck knows it. Detail beyond this comes from GET /runs/:id. */
export function buildPayload(run: RunEvent) {
  return {
    run_id: run.id,
    product: run.product,
    subject: run.capability,
    status: run.status,
    duration_ms: run.durationMs || null,
    observed: run.ts,
    detail_source: "GET /runs/:id (not yet wired)",
  };
}

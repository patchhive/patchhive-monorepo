import type { RunEvent } from "./hive-data";

export interface FailureDetail {
  code: string;
  httpStatus: number;
  stage: string;
  message: string;
  hint: string;
  requestId: string;
  traceId: string;
  attempts: number;
  budgetMs: number;
  inputs: Record<string, string | number | boolean>;
}

export function seededRand(seed: string) {
  let h = 2166136261;
  for (let i = 0; i < seed.length; i++) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return () => {
    h ^= h << 13;
    h ^= h >>> 17;
    h ^= h << 5;
    return ((h >>> 0) % 10000) / 10000;
  };
}

export function buildPayload(run: RunEvent) {
  const rand = seededRand(run.id);
  return {
    run_id: run.id,
    product: run.product,
    capability: run.capability,
    status: run.status,
    duration_ms: run.durationMs,
    started_at: new Date(Date.now() - Math.floor(rand() * 600_000)).toISOString(),
    actor: "hivecore:dispatcher",
    request: {
      trace_id: `t_${run.id.slice(2)}${Math.floor(rand() * 9999)}`,
      args: {
        target: run.product.toLowerCase(),
        mode: rand() > 0.5 ? "incremental" : "full",
        retries: Math.floor(rand() * 3),
      },
    },
    response:
      run.status === "failed"
        ? { error: "upstream_timeout", code: 504, retriable: true }
        : run.status === "running"
        ? { state: "in_progress", progress: Math.floor(rand() * 80) + 10 }
        : { ok: true, items_processed: Math.floor(rand() * 500) },
  };
}

export function deriveFailure(run: RunEvent): FailureDetail | null {
  if (run.status !== "failed") return null;
  const rand = seededRand(run.id + "_tl");
  const payload = buildPayload(run);
  const args = payload.request.args;
  return {
    code: "UPSTREAM_TIMEOUT_504",
    httpStatus: 504,
    stage: `${run.capability} → upstream.dispatch`,
    message: `${run.product} did not respond to ${run.capability} within the 8000ms budget.`,
    hint: "Retried 2× with backoff. Check the target service's /health and re-dispatch once it stabilises.",
    requestId: `req_${run.id.slice(2)}${Math.floor(rand() * 9999).toString().padStart(4, "0")}`,
    traceId: payload.request.trace_id,
    attempts: 3,
    budgetMs: 8000,
    inputs: {
      target: args.target,
      mode: args.mode,
      retries: args.retries,
      capability: run.capability,
    },
  };
}

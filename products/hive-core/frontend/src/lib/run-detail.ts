// Run detail, fetched from the product that owns the run.
//
// The drawer used to build all of this locally from a seeded PRNG: a payload with an
// invented trace id and retry count, a log stream including "retry 2/3 after 1200ms"
// and "upstream_timeout (504) — giving up", and a timeline with fabricated queue and
// start times. Against a mock deck that was set dressing. Against a live suite it
// attributed specific, plausible, entirely invented failures to real runs.
//
// The compounding problem was worse: the drawer's "Explain failure" button sent those
// generated log lines to HiveCore's model endpoint as grounding. The model would then
// produce a careful, well-reasoned explanation of events that never happened — and
// present it as analysis of a real failure.
//
// So nothing is synthesised here. HiveCore proxies the product's own run-detail route
// and returns whatever the product actually stores. When a product exposes no detail
// for a run, that is reported as the absence it is.

import { apiFetch } from "./http";

export interface RunDetail {
  slug: string;
  title: string;
  /** The product route HiveCore asked for, so a gap is traceable. */
  detail_path: string;
  detail_ok: boolean;
  remote_status: number | null;
  error: string;
  /** The product's own run record, in whatever shape it stores. */
  detail: unknown;
}

const BASE = "/api/products/hive-core";

interface Envelope<T> {
  data?: T;
  error?: { message?: string };
}

export async function fetchRunDetail(
  slug: string,
  runId: string,
  signal?: AbortSignal,
): Promise<{ detail: RunDetail | null; message: string }> {
  try {
    const response = await apiFetch(
      `${BASE}/products/${slug}/runs/${encodeURIComponent(runId)}`,
      { signal },
    );
    const body = (await response.json().catch(() => null)) as Envelope<RunDetail> | null;
    if (!response.ok) {
      return {
        detail: null,
        message: body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`,
      };
    }
    return { detail: body?.data ?? null, message: "" };
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === "AbortError") throw cause;
    return { detail: null, message: "Could not reach the control plane." };
  }
}

/**
 * Log-ish lines the product actually recorded, if it records any.
 *
 * Products are not required to expose logs, and the contract does not define a shape
 * for them. This looks for the fields products do use and returns nothing when it
 * finds none — the drawer then says so rather than inventing a stream.
 */
export function extractLogLines(detail: unknown): string[] {
  if (!detail || typeof detail !== "object") return [];
  const record = detail as Record<string, unknown>;

  for (const key of ["events", "logs", "warnings", "messages"]) {
    const value = record[key];
    if (!Array.isArray(value)) continue;
    const lines = value
      .map((entry) => {
        if (typeof entry === "string") return entry;
        if (entry && typeof entry === "object") {
          const item = entry as Record<string, unknown>;
          const message = item.message ?? item.msg ?? item.detail ?? item.summary;
          if (typeof message === "string") {
            const level = typeof item.level === "string" ? `[${item.level}] ` : "";
            return `${level}${message}`;
          }
        }
        return null;
      })
      .filter((line): line is string => Boolean(line));
    if (lines.length > 0) return lines;
  }
  return [];
}

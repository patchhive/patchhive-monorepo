// Live suite status, applied onto the registry the deck already reads.
//
// Every panel imports PRODUCTS directly, so rather than threading props through
// fifteen components this patches the entries in place and bumps a version that
// DeckInner holds. DeckInner re-rendering re-renders the tree, and each panel reads
// the refreshed values on the way past.
//
// The tradeoff is deliberate: module mutation is impure, but the alternative is
// rewiring every component's data flow, and that churn is what broke the deck once
// already. Confined to this file, one direction, one writer.

import { useEffect, useState } from "react";

import { apiFetch } from "./http";
import { ID_BY_SLUG, SLUG_BY_ID, TITLE_BY_SLUG } from "./product-slugs";
import { PRODUCTS, RUNS, type RunEvent, type Status } from "./hive-data";
import { fetchConformance } from "./conformance";
import { fetchProbes, summarise } from "./probes";

interface ApiProduct {
  key: string;
  enabled: boolean;
  status: string;
  capabilities: string[];
}

/**
 * `engine-pending` means the manifest is registered but no engine is mounted in this
 * runtime — reachable as a record, not as a service. That is closer to offline than
 * to healthy, and flattening it to "ok" would misreport HiveCore itself.
 */
function toStatus(item: ApiProduct): Status {
  if (!item.enabled) return "offline";
  switch (item.status) {
    case "online":
      return "ok";
    case "degraded":
      return "warn";
    case "offline":
      return "crit";
    default:
      return "offline";
  }
}

interface ApiRunSummary {
  id: string;
  lifecycle_status: string;
  status: string;
  title: string;
  summary: string;
  created_at: string;
  updated_at: string;
}

interface ApiProductRuns {
  key: string;
  runs: { runs: ApiRunSummary[] } | null;
}

function toRunStatus(summary: ApiRunSummary): RunEvent["status"] {
  switch (summary.lifecycle_status || summary.status) {
    case "completed":
      return "success";
    case "queued":
    case "running":
      return "running";
    default:
      return "failed";
  }
}

/** Coarse relative age. The feed reads as a stream, so exact stamps are noise. */
function relativeAge(iso: string): string {
  const at = Date.parse(iso);
  if (Number.isNaN(at)) return "—";
  const seconds = Math.max(0, Math.round((Date.now() - at) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return `${Math.round(hours / 24)}d ago`;
}

/**
 * Duration is only real when a run recorded both ends. Products that never write
 * updated_at report 0, which the feed renders as blank rather than as "instant".
 */
function durationMs(summary: ApiRunSummary): number {
  const start = Date.parse(summary.created_at);
  const end = Date.parse(summary.updated_at);
  if (Number.isNaN(start) || Number.isNaN(end) || end <= start) return 0;
  return end - start;
}

export interface LiveSuite {
  /** True once a poll has succeeded; false means the deck is showing seed values. */
  live: boolean;
  error: string;
  lastSyncedAt: Date | null;
  /** Bumped on every successful poll so consumers re-render. */
  version: number;
}

/**
 * Replace the run feed with the suite-wide index.
 *
 * One request to the backend's in-process aggregate rather than eleven calls from
 * the browser — the same shape as the auth-status aggregate, for the same reason.
 */
async function syncRuns(signal: AbortSignal): Promise<void> {
  const response = await apiFetch("/api/products/runs", { signal });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const rows = (await response.json()) as ApiProductRuns[];

  const collected: { summary: ApiRunSummary; product: string }[] = [];
  for (const row of rows) {
    if (!row.runs) continue;
    const product = TITLE_BY_SLUG[row.key] ?? row.key;
    for (const summary of row.runs.runs) collected.push({ summary, product });
  }

  // Run ids are UUIDs, so they carry no ordering. Sort on the recorded start.
  collected.sort(
    (left, right) =>
      (Date.parse(right.summary.created_at) || 0) - (Date.parse(left.summary.created_at) || 0),
  );

  const events: RunEvent[] = collected.map(({ summary, product }) => ({
    id: summary.id,
    product,
    // Products title runs with their subject (a repo, a PR); that is the most
    // useful label the contract actually carries.
    capability: summary.title || summary.summary || product,
    status: toRunStatus(summary),
    durationMs: durationMs(summary),
    ts: relativeAge(summary.created_at),
    startedAt: summary.created_at,
  }));
  RUNS.length = 0;
  RUNS.push(...events);

  // Per-product 24h counts come from the same index, so a product card can never
  // disagree with the suite total.
  const cutoff = Date.now() - 24 * 3_600_000;
  const recent = new Map<string, number>();
  for (const event of events) {
    const at = Date.parse(event.startedAt);
    if (Number.isNaN(at) || at < cutoff) continue;
    recent.set(event.product, (recent.get(event.product) ?? 0) + 1);
  }
  for (const product of PRODUCTS) {
    product.runs24h = recent.get(product.name) ?? 0;
  }
}

export function useLiveSuite(pollMs = 10_000): LiveSuite {
  const [state, setState] = useState<LiveSuite>({
    live: false,
    error: "",
    lastSyncedAt: null,
    version: 0,
  });

  useEffect(() => {
    let cancelled = false;
    const controller = new AbortController();

    async function sync() {
      try {
        const response = await apiFetch("/api/products", { signal: controller.signal });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const rows = (await response.json()) as ApiProduct[];
        if (cancelled) return;

        for (const row of rows) {
          const id = ID_BY_SLUG[row.key] ?? row.key;
          const product = PRODUCTS.find((item) => item.id === id);
          if (!product) continue;
          product.status = toStatus(row);
          if (row.capabilities.length > 0) product.capabilities = row.capabilities;
        }

        await Promise.all(
          PRODUCTS.map(async (product) => {
            const slug = SLUG_BY_ID[product.id] ?? product.id;
            if (slug === "hive-core") {
              product.latencyMs = null;
              product.uptime = null;
              product.probeState = "not_applicable";
              product.probeReason = "HiveCore has no network round trip to measure in-process.";
              return;
            }
            try {
              const probes = summarise(await fetchProbes(slug, controller.signal));
              product.latencyMs = probes.latest;
              product.uptime = probes.uptime;
              product.probeState = "observed";
              product.probeReason =
                probes.observations === 0 ? "Probe history was observed and is empty." : "";
            } catch (cause) {
              if (cause instanceof DOMException && cause.name === "AbortError") throw cause;
              product.latencyMs = null;
              product.uptime = null;
              product.probeState = "failed";
              product.probeReason =
                cause instanceof Error ? cause.message : "Probe history could not be read.";
            }
          }),
        );

        await syncRuns(controller.signal);
        if (cancelled) return;

        // Drift badges on the product cards come from the same conformance check
        // the inspector runs, so a badge cannot claim drift the panel disagrees with.
        try {
          const conformance = await fetchConformance(controller.signal);
          if (cancelled) return;
          const byKey = new Map(conformance.map((row) => [row.productKey, row]));
          for (const product of PRODUCTS) {
            const slug = SLUG_BY_ID[product.id] ?? product.id;
            product.contractDrift = byKey.get(slug)?.findings.length ?? 0;
          }
        } catch (conformanceError) {
          if (conformanceError instanceof DOMException && conformanceError.name === "AbortError") {
            return;
          }
          // Conformance is supplementary; a failure here must not blank the deck.
          for (const product of PRODUCTS) product.contractDrift = 0;
        }

        setState((current) => ({
          live: true,
          error: "",
          lastSyncedAt: new Date(),
          version: current.version + 1,
        }));
      } catch (cause) {
        if (cancelled || (cause instanceof DOMException && cause.name === "AbortError")) return;
        setState((current) => ({
          ...current,
          live: false,
          error:
            cause instanceof Error
              ? `Control plane unreachable (${cause.message}).`
              : "Control plane unreachable.",
        }));
      }
    }

    sync();
    const timer = setInterval(sync, pollMs);
    return () => {
      cancelled = true;
      controller.abort();
      clearInterval(timer);
    };
  }, [pollMs]);

  return state;
}

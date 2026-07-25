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

import { API } from "@/config";
import { PRODUCTS, type Status } from "./hive-data";

interface ApiProduct {
  key: string;
  enabled: boolean;
  status: string;
  capabilities: string[];
}

/** API product keys are kebab-case; the deck's ids are not. */
const ID_BY_SLUG: Record<string, string> = {
  "repo-reaper": "reporeaper",
  "signal-hive": "signalhive",
  "trust-gate": "trustgate",
  "repo-memory": "repomemory",
  "review-bee": "reviewbee",
  "merge-keeper": "mergekeeper",
  "flake-sting": "flakesting",
  "dep-triage": "deptriage",
  "vuln-triage": "vulntriage",
  "refactor-scout": "refactorscout",
  "release-sentry": "releasesentry",
  "hive-core": "hivecore",
};

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

export interface LiveSuite {
  /** True once a poll has succeeded; false means the deck is showing seed values. */
  live: boolean;
  error: string;
  lastSyncedAt: Date | null;
  /** Bumped on every successful poll so consumers re-render. */
  version: number;
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
        const response = await fetch(`${API}/api/products`, { signal: controller.signal });
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

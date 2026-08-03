export type Status = "ok" | "warn" | "crit" | "offline" | "unknown";

export interface Product {
  id: string;
  name: string;
  tagline: string;
  frontend: string;
  api: string;
  status: Status;
  latencyMs: number | null;
  uptime: number | null; // 0-1 when observed
  probeState: "observed" | "failed" | "not_observed" | "not_applicable";
  probeReason: string;
  runs24h: number | null;
  /** Descriptive manifest tags; never dispatchable action ids. */
  declaredCapabilities: string[];
  /** Dispatchable action ids from an observed runtime capability response. */
  capabilities: string[];
  capabilityState: "observed" | "failed" | "not_observed" | "not_applicable";
  capabilityReason: string;
  /** Null when conformance evidence could not be observed. */
  contractDrift: number | null;
}

// Identity, taglines, and capability ids mirror
// services/patchhive-backend/registry/products/*.toml. Ports mirror
// scripts/suite-common.sh. Status is refreshed from the materialized runtime endpoint.
//
// Runtime fields fail closed to unavailable/null and are populated only from live
// suite endpoints. Identity and declared capability metadata are the sole static data.
export const PRODUCTS: Product[] = ([
  { id: "reporeaper", name: "RepoReaper", tagline: "Autonomous patch and PR execution", frontend: "http://localhost:5173", api: "http://localhost:8000", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["hunt", "dry-run", "watch-mode"], contractDrift: null },
  { id: "signalhive", name: "SignalHive", tagline: "Maintenance signal reconnaissance", frontend: "http://localhost:5174", api: "http://localhost:8010", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["repo-discovery", "signal-scan", "read-only"], contractDrift: null },
  { id: "trustgate", name: "TrustGate", tagline: "Diff policy and risk review", frontend: "http://localhost:5175", api: "http://localhost:8020", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["review-diff", "review-pr", "failguard-submit"], contractDrift: null },
  { id: "repomemory", name: "RepoMemory", tagline: "Durable repo memory and prompt packs", frontend: "http://localhost:5176", api: "http://localhost:8030", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["memory-ingest", "prompt-pack", "failguard"], contractDrift: null },
  { id: "reviewbee", name: "ReviewBee", tagline: "PR review feedback checklist", frontend: "http://localhost:5177", api: "http://localhost:8040", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["review-pr", "maintained-comment"], contractDrift: null },
  { id: "mergekeeper", name: "MergeKeeper", tagline: "Merge readiness assessment", frontend: "http://localhost:5178", api: "http://localhost:8050", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["assess-pr", "publish-status"], contractDrift: null },
  { id: "flakesting", name: "FlakeSting", tagline: "CI flake detection", frontend: "http://localhost:5179", api: "http://localhost:8060", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["scan-actions", "read-only"], contractDrift: null },
  { id: "deptriage", name: "DepTriage", tagline: "Dependency update triage", frontend: "http://localhost:5180", api: "http://localhost:8070", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["scan-dependencies", "read-only"], contractDrift: null },
  { id: "vulntriage", name: "VulnTriage", tagline: "Security finding triage", frontend: "http://localhost:5181", api: "http://localhost:8110", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["scan-security", "read-only"], contractDrift: null },
  { id: "refactorscout", name: "RefactorScout", tagline: "Conservative refactor discovery", frontend: "http://localhost:5182", api: "http://localhost:8090", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["scan-repo", "read-only"], contractDrift: null },
  { id: "hivecore", name: "HiveCore", tagline: "Control plane and suite cockpit", frontend: "http://localhost:5183", api: "http://localhost:8100", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["suite-settings", "repository-policy", "pr-budgets"], contractDrift: null },
  { id: "releasesentry", name: "ReleaseSentry", tagline: "Release readiness evidence", frontend: "http://localhost:5184", api: "http://localhost:8120", status: "unknown", latencyMs: null, uptime: null, runs24h: 0, capabilities: ["check-release", "read-only"], contractDrift: null },
] satisfies Omit<Product, "probeState" | "probeReason" | "declaredCapabilities" | "capabilityState" | "capabilityReason">[]).map((product) => ({
  ...product,
  declaredCapabilities: product.capabilities,
  capabilities: [],
  capabilityState: "not_observed" as const,
  capabilityReason: "Runtime capabilities have not been observed yet.",
  runs24h: null,
  probeState: product.id === "hivecore" ? "not_applicable" : "not_observed",
  probeReason:
    product.id === "hivecore"
      ? "HiveCore has no network round trip to measure in-process."
      : "Probe history has not been observed yet.",
}));


export interface RunEvent {
  id: string;
  product: string;
  capability: string;
  status: RunLifecycleStatus;
  /** Null when the product did not report a complete time interval. */
  durationMs: number | null;
  /** Human-relative age, rendered directly. */
  ts: string;
  /** ISO start, for windowing and sorting. Empty on seed rows. */
  startedAt: string;
}

export type RunLifecycleStatus =
  | "standby"
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "held"
  | "skipped"
  | "unknown";

export const RUNS: RunEvent[] = [];

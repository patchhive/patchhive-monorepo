export type Status = "ok" | "warn" | "crit" | "offline";

export interface Product {
  id: string;
  name: string;
  tagline: string;
  frontend: string;
  api: string;
  status: Status;
  latencyMs: number;
  uptime: number; // 0-1
  runs24h: number;
  capabilities: string[];
  contractDrift: number;
}

// Identity, taglines, and capability ids mirror
// services/patchhive-backend/registry/products/*.toml. Ports mirror
// scripts/suite-common.sh. Status is refreshed from GET /api/products at runtime.
//
// Runtime fields fail closed to unavailable/zero and are populated only from live
// suite endpoints. Identity and declared capability metadata are the sole static data.
export const PRODUCTS: Product[] = [
  { id: "reporeaper", name: "RepoReaper", tagline: "Autonomous patch and PR execution", frontend: "http://localhost:5173", api: "http://localhost:8000", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["hunt", "dry-run", "watch-mode"], contractDrift: 0 },
  { id: "signalhive", name: "SignalHive", tagline: "Maintenance signal reconnaissance", frontend: "http://localhost:5174", api: "http://localhost:8010", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["repo-discovery", "signal-scan", "read-only"], contractDrift: 0 },
  { id: "trustgate", name: "TrustGate", tagline: "Diff policy and risk review", frontend: "http://localhost:5175", api: "http://localhost:8020", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["review-diff", "review-pr", "failguard-submit"], contractDrift: 0 },
  { id: "repomemory", name: "RepoMemory", tagline: "Durable repo memory and prompt packs", frontend: "http://localhost:5176", api: "http://localhost:8030", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["memory-ingest", "prompt-pack", "failguard"], contractDrift: 0 },
  { id: "reviewbee", name: "ReviewBee", tagline: "PR review feedback checklist", frontend: "http://localhost:5177", api: "http://localhost:8040", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["review-pr", "maintained-comment"], contractDrift: 0 },
  { id: "mergekeeper", name: "MergeKeeper", tagline: "Merge readiness assessment", frontend: "http://localhost:5178", api: "http://localhost:8050", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["assess-pr", "publish-status"], contractDrift: 0 },
  { id: "flakesting", name: "FlakeSting", tagline: "CI flake detection", frontend: "http://localhost:5179", api: "http://localhost:8060", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["scan-actions", "read-only"], contractDrift: 0 },
  { id: "deptriage", name: "DepTriage", tagline: "Dependency update triage", frontend: "http://localhost:5180", api: "http://localhost:8070", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["scan-dependencies", "read-only"], contractDrift: 0 },
  { id: "vulntriage", name: "VulnTriage", tagline: "Security finding triage", frontend: "http://localhost:5181", api: "http://localhost:8110", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["scan-security", "read-only"], contractDrift: 0 },
  { id: "refactorscout", name: "RefactorScout", tagline: "Conservative refactor discovery", frontend: "http://localhost:5182", api: "http://localhost:8090", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["scan-repo", "read-only"], contractDrift: 0 },
  { id: "hivecore", name: "HiveCore", tagline: "Control plane and suite cockpit", frontend: "http://localhost:5183", api: "http://localhost:8100", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["suite-settings", "repository-policy", "pr-budgets"], contractDrift: 0 },
  { id: "releasesentry", name: "ReleaseSentry", tagline: "Release readiness evidence", frontend: "http://localhost:5184", api: "http://localhost:8120", status: "offline", latencyMs: 0, uptime: 0, runs24h: 0, capabilities: ["check-release", "read-only"], contractDrift: 0 },
];


export interface RunEvent {
  id: string;
  product: string;
  capability: string;
  status: "success" | "running" | "failed";
  durationMs: number;
  /** Human-relative age, rendered directly. */
  ts: string;
  /** ISO start, for windowing and sorting. Empty on seed rows. */
  startedAt: string;
}

export const RUNS: RunEvent[] = [];

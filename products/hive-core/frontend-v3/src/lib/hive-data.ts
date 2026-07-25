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
// latencyMs / uptime / runs24h are SEEDED SAMPLE VALUES, not measurements — the
// backend does not expose per-product latency or uptime, and PatchHive products are
// not request-serving services. They drive the sparklines and heatmap until real
// run telemetry exists. The deck labels them as sampled.
export const PRODUCTS: Product[] = [
  { id: "reporeaper", name: "RepoReaper", tagline: "Autonomous patch and PR execution", frontend: "http://localhost:5173", api: "http://localhost:8000", status: "ok", latencyMs: 42, uptime: 0.999, runs24h: 184, capabilities: ["hunt", "dry-run", "watch-mode"], contractDrift: 0 },
  { id: "signalhive", name: "SignalHive", tagline: "Maintenance signal reconnaissance", frontend: "http://localhost:5174", api: "http://localhost:8010", status: "ok", latencyMs: 38, uptime: 0.998, runs24h: 1240, capabilities: ["repo-discovery", "signal-scan", "read-only"], contractDrift: 0 },
  { id: "trustgate", name: "TrustGate", tagline: "Diff policy and risk review", frontend: "http://localhost:5175", api: "http://localhost:8020", status: "ok", latencyMs: 71, uptime: 0.997, runs24h: 42, capabilities: ["review-diff", "review-pr", "failguard-submit"], contractDrift: 0 },
  { id: "repomemory", name: "RepoMemory", tagline: "Durable repo memory and prompt packs", frontend: "http://localhost:5176", api: "http://localhost:8030", status: "warn", latencyMs: 312, uptime: 0.982, runs24h: 92, capabilities: ["memory-ingest", "prompt-pack", "failguard"], contractDrift: 1 },
  { id: "reviewbee", name: "ReviewBee", tagline: "PR review feedback checklist", frontend: "http://localhost:5177", api: "http://localhost:8040", status: "ok", latencyMs: 58, uptime: 0.996, runs24h: 311, capabilities: ["review-pr", "maintained-comment"], contractDrift: 0 },
  { id: "mergekeeper", name: "MergeKeeper", tagline: "Merge readiness assessment", frontend: "http://localhost:5178", api: "http://localhost:8050", status: "ok", latencyMs: 49, uptime: 0.999, runs24h: 76, capabilities: ["assess-pr", "publish-status"], contractDrift: 0 },
  { id: "flakesting", name: "FlakeSting", tagline: "CI flake detection", frontend: "http://localhost:5179", api: "http://localhost:8060", status: "warn", latencyMs: 188, uptime: 0.984, runs24h: 53, capabilities: ["scan-actions", "read-only"], contractDrift: 0 },
  { id: "deptriage", name: "DepTriage", tagline: "Dependency update triage", frontend: "http://localhost:5180", api: "http://localhost:8070", status: "ok", latencyMs: 64, uptime: 0.997, runs24h: 22, capabilities: ["scan-dependencies", "read-only"], contractDrift: 0 },
  { id: "vulntriage", name: "VulnTriage", tagline: "Security finding triage", frontend: "http://localhost:5181", api: "http://localhost:8110", status: "crit", latencyMs: 0, uptime: 0.871, runs24h: 0, capabilities: ["scan-security", "read-only"], contractDrift: 2 },
  { id: "refactorscout", name: "RefactorScout", tagline: "Conservative refactor discovery", frontend: "http://localhost:5182", api: "http://localhost:8090", status: "ok", latencyMs: 95, uptime: 0.995, runs24h: 18, capabilities: ["scan-repo", "read-only"], contractDrift: 0 },
  { id: "hivecore", name: "HiveCore", tagline: "Control plane and suite cockpit", frontend: "http://localhost:5183", api: "http://localhost:8100", status: "ok", latencyMs: 12, uptime: 1.0, runs24h: 0, capabilities: ["suite-settings", "repository-policy", "pr-budgets"], contractDrift: 0 },
  { id: "releasesentry", name: "ReleaseSentry", tagline: "Release readiness evidence", frontend: "http://localhost:5184", api: "http://localhost:8120", status: "ok", latencyMs: 54, uptime: 0.994, runs24h: 37, capabilities: ["check-release", "read-only"], contractDrift: 0 },
];

// IDs the live mesh currently reports as participating.
// Anything in PRODUCTS but not in LIVE_MESH is treated as "missing from mesh".
export const LIVE_MESH: string[] = [
  "reporeaper",
  "signalhive",
  "trustgate",
  "repomemory",
  "reviewbee",
  "mergekeeper",
  "flakesting",
  "deptriage",
  "vulntriage",
  "hivecore",
  "releasesentry",
];

export type MeshPresence = "online" | "offline" | "missing";

export function meshPresence(product: Product): MeshPresence {
  if (!LIVE_MESH.includes(product.id)) return "missing";
  if (product.status === "crit" || product.status === "offline") return "offline";
  return "online";
}

export interface RunEvent {
  id: string;
  product: string;
  capability: string;
  status: "success" | "running" | "failed";
  durationMs: number;
  ts: string;
}

export const RUNS: RunEvent[] = [
  { id: "r_8af5", product: "ReleaseSentry", capability: "check.github.release", status: "success", durationMs: 318, ts: "4s ago" },
  { id: "r_8af4", product: "ReleaseSentry", capability: "blockers.scan", status: "running", durationMs: 0, ts: "9s ago" },
  { id: "r_8af3", product: "SignalHive", capability: "alert.dispatch", status: "success", durationMs: 142, ts: "12s ago" },
  { id: "r_8af2", product: "ReviewBee", capability: "pr.review", status: "running", durationMs: 0, ts: "18s ago" },
  { id: "r_8af1", product: "RepoReaper", capability: "scan.repo", status: "success", durationMs: 2810, ts: "44s ago" },
  { id: "r_8af0", product: "VulnTriage", capability: "cve.ingest", status: "failed", durationMs: 9211, ts: "1m ago" },
  { id: "r_8aef", product: "MergeKeeper", capability: "merge.gate", status: "success", durationMs: 88, ts: "1m ago" },
  { id: "r_8aee", product: "FlakeSting", capability: "test.classify", status: "success", durationMs: 1640, ts: "2m ago" },
  { id: "r_8aed", product: "DepTriage", capability: "dep.audit", status: "success", durationMs: 740, ts: "3m ago" },
  { id: "r_8aec", product: "RepoMemory", capability: "embed.commit", status: "success", durationMs: 4220, ts: "3m ago" },
  { id: "r_8aeb", product: "SignalHive", capability: "signal.normalize", status: "success", durationMs: 61, ts: "4m ago" },
  { id: "r_8aea", product: "TrustGate", capability: "token.rotate", status: "success", durationMs: 210, ts: "5m ago" },
];

// Extended ops data: incidents, dependencies, SLOs, drift schemas, tokens.
import { PRODUCTS } from "./hive-data";

export type IncidentSeverity = "warn" | "crit";
export interface Incident {
  id: string;
  productId: string;
  from: string; // ISO
  to: string | null; // null = ongoing
  severity: IncidentSeverity;
  summary: string;
  resolution?: string;
}

// Deterministic-ish recent incidents. Anchored to "now" at module load,
// fine for a mock deck.
const NOW = Date.now();
const m = (mins: number) => new Date(NOW - mins * 60_000).toISOString();

export const INCIDENTS: Incident[] = [
  { id: "i_001", productId: "vulntriage", from: m(74), to: null, severity: "crit", summary: "CVE ingest worker pool exhausted" },
  { id: "i_002", productId: "repomemory", from: m(220), to: m(38), severity: "warn", summary: "Embedding queue backpressure", resolution: "Scaled worker replicas 3 → 6" },
  { id: "i_003", productId: "flakesting", from: m(180), to: m(95), severity: "warn", summary: "Classifier latency spike", resolution: "Rolled back model v0.4 → v0.3" },
  { id: "i_004", productId: "vulntriage", from: m(1440), to: m(1280), severity: "crit", summary: "Upstream NVD feed 504", resolution: "Failover to mirror" },
  { id: "i_005", productId: "trustgate", from: m(2880), to: m(2820), severity: "warn", summary: "Token rotation backlog", resolution: "Cleared rotation queue" },
  { id: "i_006", productId: "signalhive", from: m(4320), to: m(4290), severity: "warn", summary: "Alert dedupe cache miss-storm", resolution: "Warmed cache via cron" },
  { id: "i_007", productId: "reviewbee", from: m(7200), to: m(7110), severity: "warn", summary: "GitHub API rate-limit", resolution: "Switched to app token" },
];

export function mttrMinutes(productId: string): number | null {
  const closed = INCIDENTS.filter((i) => i.productId === productId && i.to);
  if (closed.length === 0) return null;
  const total = closed.reduce((acc, i) => acc + (Date.parse(i.to!) - Date.parse(i.from)) / 60_000, 0);
  return Math.round(total / closed.length);
}

// Dependency edges — caller → callee.
export const DEPENDENCIES: Array<{ from: string; to: string }> = [
  { from: "reviewbee", to: "trustgate" },
  { from: "reviewbee", to: "signalhive" },
  { from: "reviewbee", to: "repomemory" },
  { from: "mergekeeper", to: "reviewbee" },
  { from: "mergekeeper", to: "flakesting" },
  { from: "repomemory", to: "reporeaper" },
  { from: "vulntriage", to: "deptriage" },
  { from: "vulntriage", to: "signalhive" },
  { from: "releasesentry", to: "mergekeeper" },
  { from: "releasesentry", to: "signalhive" },
  { from: "releasesentry", to: "vulntriage" },
  { from: "refactorscout", to: "repomemory" },
  { from: "flakesting", to: "signalhive" },
  { from: "trustgate", to: "signalhive" },
  { from: "deptriage", to: "signalhive" },
  { from: "reporeaper", to: "signalhive" },
  { from: "hivecore", to: "trustgate" },
];

// Service Level Objectives: per-product uptime target.
export interface SLO {
  productId: string;
  target: number; // e.g. 0.995
  window: string; // human label
}
export const SLOS: SLO[] = PRODUCTS.map((p) => ({
  productId: p.id,
  target: p.id === "hivecore" ? 0.999 : p.id === "vulntriage" ? 0.99 : 0.995,
  window: "30d",
}));

export function errorBudgetBurn(actual: number, target: number): number {
  // Fraction of error budget consumed. 0 = healthy, 1 = budget fully spent, >1 = breached.
  const allowed = 1 - target;
  if (allowed <= 0) return 0;
  const spent = Math.max(0, 1 - actual);
  return spent / allowed;
}

// Contract drift schemas — expected vs actual advertised capabilities.
export interface DriftSchema {
  productId: string;
  expected: string[];
  actual: string[];
  notes?: string;
}
export const DRIFT_SCHEMAS: DriftSchema[] = [
  {
    productId: "repomemory",
    expected: ["embed.commit", "search.semantic", "purge.index"],
    actual: ["embed.commit", "search.semantic"],
    notes: "purge.index advertised in v0.4 manifest but not in /capabilities response",
  },
  {
    productId: "vulntriage",
    expected: ["cve.ingest", "cve.score", "cve.notify", "audit.read"],
    actual: ["cve.ingest", "cve.score"],
    notes: "cve.notify + audit.read missing — worker pool down",
  },
];

// Token vault state
export type TokenStatus = "active" | "expiring" | "rotated" | "revoked";
export interface VaultToken {
  id: string;
  name: string;
  scope: string;
  status: TokenStatus;
  issuedAt: string; // ISO
  expiresAt: string; // ISO
  lastUsedAt: string;
}
const dPast = (days: number) => new Date(NOW - days * 86_400_000).toISOString();
const dFuture = (days: number) => new Date(NOW + days * 86_400_000).toISOString();
export const TOKENS: VaultToken[] = [
  { id: "tok_8af3", name: "reviewbee → github", scope: "repo:write", status: "active", issuedAt: dPast(18), expiresAt: dFuture(72), lastUsedAt: m(2) },
  { id: "tok_8af2", name: "mergekeeper → ci", scope: "ci:dispatch", status: "active", issuedAt: dPast(11), expiresAt: dFuture(79), lastUsedAt: m(8) },
  { id: "tok_8af1", name: "signalhive → pagerduty", scope: "incident:write", status: "active", issuedAt: dPast(40), expiresAt: dFuture(50), lastUsedAt: m(1) },
  { id: "tok_8af0", name: "vulntriage → nvd", scope: "feed:read", status: "expiring", issuedAt: dPast(83), expiresAt: dFuture(4), lastUsedAt: m(74) },
  { id: "tok_8aef", name: "releasesentry → github", scope: "release:read", status: "expiring", issuedAt: dPast(85), expiresAt: dFuture(2), lastUsedAt: m(11) },
  { id: "tok_8aee", name: "repomemory → openai", scope: "embeddings", status: "active", issuedAt: dPast(5), expiresAt: dFuture(85), lastUsedAt: m(3) },
  { id: "tok_8aed", name: "trustgate → vault-root", scope: "kms:sign", status: "rotated", issuedAt: dPast(91), expiresAt: dPast(1), lastUsedAt: dPast(1) },
  { id: "tok_8aec", name: "deptriage → snyk", scope: "audit:read", status: "active", issuedAt: dPast(22), expiresAt: dFuture(68), lastUsedAt: m(180) },
  { id: "tok_8aeb", name: "reporeaper → github-app", scope: "repo:read", status: "active", issuedAt: dPast(60), expiresAt: dFuture(30), lastUsedAt: m(45) },
  { id: "tok_8aea", name: "old-mergekeeper", scope: "ci:dispatch", status: "revoked", issuedAt: dPast(120), expiresAt: dPast(30), lastUsedAt: dPast(31) },
];

// Schemas for capability inputs/outputs, surfaced in the run drawer.
export interface CapabilitySchema {
  input: Record<string, string>;
  output: Record<string, string>;
}
export const CAPABILITY_SCHEMAS: Record<string, CapabilitySchema> = {
  "scan.repo": { input: { repo: "string", branch: "string?" }, output: { commits: "number", health: "0..1" } },
  "report.health": { input: { repo: "string" }, output: { score: "0..1", flags: "string[]" } },
  "schedule.cron": { input: { expr: "string", target: "string" }, output: { jobId: "string" } },
  "alert.dispatch": { input: { signal: "Signal", level: "warn|crit" }, output: { delivered: "boolean" } },
  "signal.normalize": { input: { raw: "object" }, output: { signal: "Signal" } },
  "token.issue": { input: { scope: "string", ttl: "number" }, output: { token: "string", id: "string" } },
  "token.rotate": { input: { id: "string" }, output: { id: "string", rotatedAt: "string" } },
  "audit.read": { input: { since: "string" }, output: { events: "Event[]" } },
  "embed.commit": { input: { sha: "string", diff: "string" }, output: { vectorId: "string" } },
  "search.semantic": { input: { q: "string", k: "number" }, output: { hits: "Hit[]" } },
  "pr.review": { input: { pr: "number" }, output: { verdict: "approve|request|comment" } },
  "pr.comment": { input: { pr: "number", body: "string" }, output: { commentId: "number" } },
  "merge.queue": { input: { pr: "number" }, output: { position: "number" } },
  "merge.gate": { input: { pr: "number" }, output: { allowed: "boolean", reasons: "string[]" } },
  "test.classify": { input: { runId: "string" }, output: { flaky: "string[]" } },
  "test.quarantine": { input: { test: "string" }, output: { quarantined: "boolean" } },
  "dep.audit": { input: { manifest: "string" }, output: { vulnerable: "Dep[]" } },
  "dep.upgrade": { input: { dep: "string", to: "string" }, output: { pr: "number" } },
  "cve.ingest": { input: { source: "nvd|ghsa" }, output: { ingested: "number" } },
  "cve.score": { input: { cve: "string" }, output: { score: "0..10", vector: "string" } },
  "scan.smells": { input: { path: "string" }, output: { findings: "Finding[]" } },
  "suggest.refactor": { input: { findingId: "string" }, output: { patch: "string" } },
  "registry.read": { input: {}, output: { products: "Product[]" } },
  "registry.poll": { input: {}, output: { polled: "number" } },
  "action.dispatch": { input: { product: "string", capability: "string", args: "object" }, output: { runId: "string" } },
  "check.github.release": { input: { repo: "string", tag: "string" }, output: { ready: "boolean", blockers: "string[]" } },
  "drift.changelog": { input: { repo: "string", tag: "string" }, output: { drift: "string[]" } },
  "blockers.scan": { input: { repo: "string" }, output: { blockers: "Blocker[]" } },
};

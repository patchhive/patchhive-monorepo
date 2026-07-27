// Extended ops data: incidents, dependencies, SLOs, drift schemas, tokens.
import { PRODUCTS, type Product, type RunEvent } from "./hive-data";

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

/**
 * Incidents, derived from the real run feed.
 *
 * These were a hand-written list of plausible-sounding outages — "CVE ingest worker
 * pool exhausted", "Upstream NVD feed 504" — anchored to page load so they always
 * looked recent. Against a mock deck that was set dressing. Against a live suite it
 * was a control plane asserting failures that never happened, in the one panel an
 * operator would trust during an actual incident.
 *
 * PatchHive has no incident *record*, so nothing here invents one. What it has is
 * run outcomes, and a run that failed is the only failure evidence the suite
 * actually holds. So an incident is a streak of failed runs for one product: it
 * opens at the first failure and closes at the first success after it. Anything
 * richer — root cause, contributing factors, resolution text — is not derivable
 * from a run summary and is therefore absent rather than guessed.
 *
 * Same rule as run-failure.ts: report what the run carries, synthesise nothing.
 */
const CRITICAL_STREAK = 3;

export function deriveIncidents(runs: RunEvent[], products: Product[]): Incident[] {
  const idByName = new Map(products.map((product) => [product.name, product.id]));

  // Seed rows carry no ISO timestamp and cannot be ordered, so they cannot
  // establish that a failure preceded a recovery. Excluded rather than assumed.
  const dated = runs
    .filter((run) => run.startedAt)
    .sort((a, b) => Date.parse(a.startedAt) - Date.parse(b.startedAt));

  const byProduct = new Map<string, RunEvent[]>();
  for (const run of dated) {
    const id = idByName.get(run.product) ?? run.product;
    const bucket = byProduct.get(id);
    if (bucket) bucket.push(run);
    else byProduct.set(id, [run]);
  }

  const incidents: Incident[] = [];

  for (const [productId, history] of byProduct) {
    let open: { from: string; failures: RunEvent[] } | null = null;

    for (const run of history) {
      if (run.status === "failed") {
        if (open) open.failures.push(run);
        else open = { from: run.startedAt, failures: [run] };
        continue;
      }
      // A run still in flight neither breaks a streak nor proves recovery.
      if (run.status !== "success") continue;
      if (open) {
        incidents.push(close(productId, open, run.startedAt));
        open = null;
      }
    }

    if (open) incidents.push(close(productId, open, null));
  }

  return incidents.sort((a, b) => Date.parse(b.from) - Date.parse(a.from));
}

function close(
  productId: string,
  open: { from: string; failures: RunEvent[] },
  to: string | null,
): Incident {
  const count = open.failures.length;
  const capabilities = [...new Set(open.failures.map((run) => run.capability))];
  const subject =
    capabilities.length === 1 ? capabilities[0] : `${capabilities.length} capabilities`;

  return {
    id: `inc_${productId}_${Date.parse(open.from)}`,
    productId,
    from: open.from,
    to,
    // Severity is streak length, and nothing else. It is a statement about how
    // many runs failed, not a judgement about impact the deck cannot make.
    severity: count >= CRITICAL_STREAK ? "crit" : "warn",
    summary:
      count === 1
        ? `Failed ${subject} run`
        : `${count} consecutive failed runs · ${subject}`,
    // Recovery is observed, not explained. There is no resolution text because the
    // suite records none; a later success is the whole of what is known.
    resolution: to ? `Recovered on the next successful ${subject} run` : undefined,
  };
}

export function mttrMinutes(productId: string, incidents: Incident[]): number | null {
  const closed = incidents.filter((i) => i.productId === productId && i.to);
  if (closed.length === 0) return null;
  const total = closed.reduce(
    (acc, i) => acc + (Date.parse(i.to as string) - Date.parse(i.from)) / 60_000,
    0,
  );
  return Math.round(total / closed.length);
}

// Safety-gating edges — who a product depends on before it may act.
//
// These are NOT RPC calls: PatchHive products do not sit in each other's request
// paths. The real coupling is authority and evidence, which is what makes "blast
// radius" a safety question rather than a topology diagram. Sourced from AGENTS.md,
// docs/hivecore-architecture.md, and the FailGuard notes.
export type EdgeKind = "authority" | "gate" | "context" | "handoff";

export interface SafetyEdge {
  /** The product that cannot proceed if `to` is unavailable. */
  from: string;
  to: string;
  kind: EdgeKind;
  /** What actually stops. */
  effect: string;
  /** false = documented intent, not yet wired. */
  live: boolean;
}

export const EDGE_LABEL: Record<EdgeKind, string> = {
  authority: "authorises",
  gate: "gates",
  context: "informs",
  handoff: "feeds",
};

export const DEPENDENCIES: SafetyEdge[] = [
  {
    from: "reporeaper",
    to: "hivecore",
    kind: "authority",
    effect: "Repository policy check and PR slot reservation fail closed; no pull request opens.",
    live: true,
  },
  {
    from: "reporeaper",
    to: "trustgate",
    kind: "gate",
    effect: "Generated patches lose their risk review before write actions proceed.",
    live: true,
  },
  {
    from: "reporeaper",
    to: "repomemory",
    kind: "context",
    effect: "Smith rejections stop becoming FailGuard lesson candidates.",
    live: true,
  },
  {
    from: "trustgate",
    to: "repomemory",
    kind: "context",
    effect: "warn/block reviews stop submitting FailGuard candidates.",
    live: true,
  },
  {
    from: "signalhive",
    to: "reporeaper",
    kind: "handoff",
    effect: "Discovered maintenance pressure has nowhere to be acted on.",
    live: false,
  },
  {
    from: "mergekeeper",
    to: "hivecore",
    kind: "authority",
    effect: "Status and comment writes lose suite policy enforcement.",
    live: false,
  },
  {
    from: "reviewbee",
    to: "hivecore",
    kind: "authority",
    effect: "Maintained-comment publishing loses suite policy enforcement.",
    live: false,
  },
  {
    from: "trustgate",
    to: "hivecore",
    kind: "authority",
    effect: "Check writes lose suite policy enforcement.",
    live: false,
  },
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

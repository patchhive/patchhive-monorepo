// Suite-level control-plane state.
//
// Everything here is a client-side shape for what HiveCore's backend owns per
// docs/hivecore-architecture.md §3. None of it is wired yet — the arrays are empty
// on purpose so the deck renders honest empty states instead of fabricated activity.
//
// Backing endpoints, once the blockers in §6 are cleared:
//   suite events      GET  /events                      (§3.3 suite_events)
//   budgets           GET  /pr-budgets                  (exists today)
//   reservations      GET  /pr-budgets                  (exists today)
//   repository policy GET  /repository-policies         (exists today, free-text → rows)
//   approvals         GET  /approvals                   (§3.5, not built — unblocks dispatch)
//   mandates          GET  /mandates                    (§3.6, not built)
//   work items        GET  /work                        (§3.8, not built)
//   procedures        GET  /setup/first-stack           (smoke tiers + fleet jobs, exists today)

import { PRODUCTS, type Product } from "./hive-data";

/* ------------------------------------------------------------------ events */

export type SuiteEventKind =
  | "policy_decision"
  | "budget"
  | "dispatch"
  | "approval"
  | "product_state"
  | "mandate"
  | "operator";

export type SuiteEventLevel = "info" | "warn" | "crit";

export interface SuiteEvent {
  id: string;
  ts: string;
  kind: SuiteEventKind;
  level: SuiteEventLevel;
  actor: string;
  productKey: string | null;
  repository: string | null;
  runId: string | null;
  operation: string;
  /** Full precedence chain for policy decisions; one line per evaluated rule. */
  reasonChain: string[];
  summary: string;
}

export const SUITE_EVENTS: SuiteEvent[] = [];

/* ----------------------------------------------------------------- budgets */

export interface ProductBudget {
  productKey: string;
  limit: number;
  used: number;
  remaining: number;
}

export interface Reservation {
  id: string;
  productKey: string;
  repository: string;
  runId: string;
  action: string;
  status: "reserved" | "committed" | "released" | "expired";
  prUrl: string;
  reason: string;
  createdAt: string;
  expiresAt: string;
}

export interface BudgetState {
  suiteLimit: number;
  suiteUsed: number;
  suiteRemaining: number;
  products: ProductBudget[];
  reservations: Reservation[];
  /** Why the most recent denial happened, and which layer won. */
  lastDenial: { at: string; productKey: string; layer: "product" | "suite"; reason: string } | null;
}

// Defaults mirror the backend: suite ceiling 10, RepoReaper 5, everything else 0
// until an operator grants capacity (products/hive-core/backend/src/db.rs).
export const BUDGETS: BudgetState = {
  suiteLimit: 10,
  suiteUsed: 0,
  suiteRemaining: 10,
  products: PRODUCTS.map((p) => ({
    productKey: p.key,
    limit: p.key === "repo-reaper" ? 5 : 0,
    used: 0,
    remaining: p.key === "repo-reaper" ? 5 : 0,
  })),
  reservations: [],
  lastDenial: null,
};

/**
 * Committed reservations are released only by the owning product's PR monitor.
 * Anything committed and long-idle is a candidate leak — blocker B5. Surfacing it
 * here is the point: today the ceiling can ratchet to zero with no error anywhere.
 */
export function stalledReservations(
  reservations: Reservation[],
  now = Date.now(),
  maxAgeHours = 72,
): Reservation[] {
  return reservations.filter(
    (r) =>
      r.status === "committed" &&
      now - Date.parse(r.createdAt) > maxAgeHours * 3_600_000,
  );
}

/* -------------------------------------------------------------- approvals */

export interface ApprovalRequest {
  id: string;
  productKey: string;
  actionId: string;
  repository: string;
  runId: string;
  /** Hash of the dispatch input — a grant is bound to exactly this payload. */
  inputHash: string;
  requestedAt: string;
  expiresAt: string;
  state: "pending" | "granted" | "denied" | "expired";
  summary: string;
}

export const APPROVALS: ApprovalRequest[] = [];

/* ---------------------------------------------------------------- policy */

export type PolicyListKind = "opt_out" | "denylist" | "allowlist" | "trusted";

export interface RepositoryPolicyEntry {
  repository: string;
  kind: PolicyListKind;
  notes: string;
  updatedAt: string;
  /** opt_out entries verified through the public patchhive.dev flow (§2, not built). */
  verified: boolean;
}

export const REPOSITORY_POLICIES: RepositoryPolicyEntry[] = [];

/** Evaluation order from docs/hivecore-repository-safety-and-pr-budgets.md. */
export const POLICY_PRECEDENCE = [
  "public repository opt-out",
  "operator denylist",
  "allowlist / directed-scope eligibility",
  "operation trust requirement",
  "product safety and approval requirements",
  "per-product PR capacity",
  "suite-wide PR capacity",
  "atomic reservation immediately before PR creation",
] as const;

export interface PolicyDecision {
  repository: string;
  productKey: string;
  operation: string;
  decision: "allowed" | "blocked";
  /** One entry per precedence step, in order, with the verdict that step reached. */
  chain: { step: string; verdict: "pass" | "block" | "skip"; detail: string }[];
}

/* -------------------------------------------------------------- mandates */

export type AutonomyLevel = "observe" | "propose" | "act_with_approval" | "act";

export const AUTONOMY_LEVELS: AutonomyLevel[] = [
  "observe",
  "propose",
  "act_with_approval",
  "act",
];

export interface Mandate {
  id: string;
  name: string;
  objective: string;
  scope: { topics: string[]; languages: string[]; minStars: number };
  allowlist: string;
  autonomy: AutonomyLevel;
  prBudget: number;
  costBudgetUsdPerDay: number;
  politeness: { perOwnerOpenPrs: number; cooldownAfterClose: string };
  enabled: boolean;
  /** Products whose unavailability blocks this mandate (§3.12 fails closed). */
  gatedBy: string[];
}

export const MANDATES: Mandate[] = [];

/* ------------------------------------------------------------ work items */

export type WorkState =
  | "discovered"
  | "triaged"
  | "gated"
  | "ready"
  | "dispatched"
  | "shipped"
  | "abandoned";

export const WORK_STATES: WorkState[] = [
  "discovered",
  "triaged",
  "gated",
  "ready",
  "dispatched",
  "shipped",
  "abandoned",
];

export interface WorkItem {
  id: string;
  mandateId: string;
  kind: string;
  repository: string;
  subjectRef: string;
  fingerprint: string;
  state: WorkState;
  attempts: number;
  blockedOn: string | null;
  outcome: "merged" | "closed_unmerged" | "stale" | null;
}

export const WORK_ITEMS: WorkItem[] = [];

/**
 * Which work stalls if a product goes away. The real dependency graph is
 * safety-gating, not RPC: gate products block, memory products defer.
 */
export function blastRadius(productKey: string): {
  blockedWork: WorkItem[];
  stalledMandates: Mandate[];
} {
  return {
    blockedWork: WORK_ITEMS.filter(
      (w) => w.state !== "shipped" && w.state !== "abandoned" && gatesWork(productKey, w),
    ),
    stalledMandates: MANDATES.filter((m) => m.enabled && m.gatedBy.includes(productKey)),
  };
}

function gatesWork(productKey: string, item: WorkItem): boolean {
  const mandate = MANDATES.find((m) => m.id === item.mandateId);
  return mandate ? mandate.gatedBy.includes(productKey) : false;
}

/* ------------------------------------------------------------ procedures */

/**
 * Operator procedures that already have real backends: the four smoke tiers
 * (first_stack_smoke_runs) and fleet launch jobs. This is what "runbook history"
 * binds to — not invented playbooks.
 */
export type ProcedureKind =
  | "smoke:first-stack"
  | "smoke:read-only-fleet"
  | "smoke:write-dry-run"
  | "smoke:release-gate"
  | "fleet:launch"
  | "pairing"
  | "reconciliation";

export interface ProcedureStep {
  key: string;
  label: string;
  phase: string;
  status: "queued" | "running" | "pass" | "warn" | "fail" | "skipped";
  message: string;
  finishedAt: string | null;
}

export interface ProcedureRun {
  id: string;
  kind: ProcedureKind;
  status: "queued" | "running" | "ready" | "attention" | "blocked" | "failed";
  startedAt: string;
  finishedAt: string | null;
  summary: string;
  steps: ProcedureStep[];
}

export const PROCEDURES: ProcedureRun[] = [];

export const PROCEDURE_LABELS: Record<ProcedureKind, string> = {
  "smoke:first-stack": "First-stack smoke",
  "smoke:read-only-fleet": "Read-only fleet smoke",
  "smoke:write-dry-run": "RepoReaper dry-run smoke",
  "smoke:release-gate": "ReleaseSentry release-gate smoke",
  "fleet:launch": "Fleet launch",
  pairing: "Service-token pairing",
  reconciliation: "PR reconciliation sweep",
};

/* --------------------------------------------------------- suite control */

export interface SuitePause {
  paused: boolean;
  scope: "suite" | "product" | "mandate" | "repository";
  target: string | null;
  since: string | null;
  reason: string;
}

export const INITIAL_PAUSE: SuitePause = {
  paused: false,
  scope: "suite",
  target: null,
  since: null,
  reason: "",
};

/* ------------------------------------------------------------------ drift */

export interface DriftFinding {
  productKey: string;
  /** Conformance dimension, not just capability presence (§3.14). */
  dimension: "capabilities" | "routes" | "health" | "safety";
  expected: string[];
  actual: string[];
  note: string;
}

/**
 * Declared-vs-observed for capabilities. Returns nothing until products have
 * actually been polled, which is correct: absence of observation is not drift.
 */
export function capabilityDrift(product: Product): DriftFinding | null {
  if (product.observed.observedAt === null) return null;
  const declared = product.declared.map((c) => c.id);
  const actual = product.observed.actions;
  const missing = declared.filter((id) => !actual.includes(id));
  const extra = actual.filter((id) => !declared.includes(id));
  if (missing.length === 0 && extra.length === 0) return null;
  return {
    productKey: product.key,
    dimension: "capabilities",
    expected: declared,
    actual,
    note:
      [
        missing.length ? `${missing.length} declared but not advertised` : "",
        extra.length ? `${extra.length} advertised but not declared` : "",
      ]
        .filter(Boolean)
        .join(" · ") || "capability set differs from manifest",
  };
}

/** A read-only product that emitted a write is a conformance failure, not a warning. */
export function safetyViolations(events: SuiteEvent[]): SuiteEvent[] {
  return events.filter((event) => {
    if (event.kind !== "dispatch" || !event.productKey) return false;
    const product = PRODUCTS.find((p) => p.key === event.productKey);
    return Boolean(product?.safety.readOnly) && /write|mutate|comment|pull_request/.test(event.operation);
  });
}

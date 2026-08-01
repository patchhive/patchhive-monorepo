// Contract conformance: what a product's manifest declares versus what its engine
// advertises at runtime.
//
// The naive version of this — diffing manifest [[capabilities]] against the action
// ids from /capabilities — is meaningless, because they are different vocabularies.
// Manifest capabilities are descriptive tags ("signal-scan", "read-only"); advertised
// actions are dispatchable operations ("scan", "run_schedule_now"). Set-differencing
// them would report total drift on every product.
//
// What *is* comparable, and what actually matters, is the safety boundary. Both
// sides state it independently: the manifest in [safety], each action in its explicit
// effect / approval / credential_requirements contract. A product exceeding its
// declared external-effect ceiling is a real conformance failure.
//
// The two directions are not symmetric, and conflating them produced a false
// positive (architecture doc § 6a):
//
//   posture is a CEILING  — an action exceeding it is critical. read_only means no
//                           action may write externally or mutate a repository.
//   posture is an EXISTENCE claim for capabilities the product offers — declaring
//                           approval or PR capability means *some* action has it,
//                           not all of them. RepoMemory gates four curation actions
//                           and deliberately leaves its unattended FailGuard intake
//                           ungated; requiring approval there would stall the loop.

import { apiFetch } from "./http";

export type Severity = "critical" | "warning";

export interface ConformanceFinding {
  productKey: string;
  severity: Severity;
  /** Short machine-ish label for grouping. */
  kind: string;
  detail: string;
  /** What the manifest claims. */
  declared: string;
  /** What the runtime advertises. */
  advertised: string;
}

export interface ProductConformance {
  productKey: string;
  productName: string;
  /** False when the engine is not mounted, so absence is not reported as drift. */
  observed: boolean;
  actionCount: number;
  findings: ConformanceFinding[];
}

interface ApiSafety {
  read_only: boolean;
  writes_external_state: boolean;
  mutates_repositories: boolean;
  opens_pull_requests: boolean;
  requires_operator_approval: boolean;
  credential_scopes: string[];
}

interface ApiProduct {
  key: string;
  name: string;
  enabled: boolean;
  safety: ApiSafety;
}

type ApiActionEffect =
  | { kind: "read_only" }
  | { kind: "writes_local_state" }
  | { kind: "writes_external_state" }
  | { kind: "mutates_repository"; opens_pull_request: boolean };

type ApiApprovalPolicy = "automatic" | "operator_required";

interface ApiAction {
  id: string;
  label: string;
  effect?: ApiActionEffect;
  approval?: ApiApprovalPolicy;
  // Rolling-upgrade compatibility only. New engines always emit effect/approval.
  read_only?: boolean;
  mutating?: boolean;
  destructive?: boolean;
  opens_pr?: boolean;
  requires_approval?: boolean;
  credential_requirements?: string[];
}

interface ApiCapabilityReport {
  key: string;
  advertised: { actions: ApiAction[] } | null;
}

function effectKind(action: ApiAction): ApiActionEffect["kind"] | null {
  if (action.effect) return action.effect.kind;
  if (action.read_only === true && action.mutating !== true) return "read_only";
  if (action.mutating === true && action.read_only !== true) {
    return action.opens_pr ? "mutates_repository" : "writes_external_state";
  }
  return null;
}

function isMutating(action: ApiAction): boolean {
  const kind = effectKind(action);
  return kind !== null && kind !== "read_only";
}

function exceedsReadOnlyBoundary(action: ApiAction): boolean {
  const kind = effectKind(action);
  return kind === "writes_external_state" || kind === "mutates_repository";
}

function opensPullRequest(action: ApiAction): boolean {
  return action.effect?.kind === "mutates_repository"
    ? action.effect.opens_pull_request
    : Boolean(action.opens_pr);
}

function requiresApproval(action: ApiAction): boolean {
  return action.approval
    ? action.approval === "operator_required"
    : Boolean(action.requires_approval);
}

/**
 * Credential scopes that only make sense if something is being written.
 *
 * A read-only action requiring one of these is contradicting itself, and that
 * contradiction is not visible to any declaration-versus-declaration check.
 * MergeKeeper's assess_github_pr declared read_only while its request body
 * defaulted publish_report to true, so dispatching it with no body wrote a comment
 * and a commit status to GitHub. Both declarations agreed with each other; only the
 * credentials gave it away.
 */
// Only genuine external-state writes. An earlier version also matched provider:ai,
// which flagged RepoReaper's dry stalk — an action that truly writes nothing but
// does spend AI credit. Cost is a real concern and a different one; folding it in
// here would have made the check cry wolf on a correct declaration.
const WRITE_SCOPE = /:write\b/;

function writeScopes(action: ApiAction): string[] {
  return (action.credential_requirements ?? []).filter((scope) => WRITE_SCOPE.test(scope));
}

function compare(product: ApiProduct, actions: ApiAction[]): ConformanceFinding[] {
  const findings: ConformanceFinding[] = [];
  const safety = product.safety;
  const scopes = new Set(safety.credential_scopes);

  for (const action of actions) {
    if (effectKind(action) === null) {
      findings.push({
        productKey: product.key,
        severity: "critical",
        kind: "missing action effect",
        detail: `Action \`${action.id}\` does not declare an unambiguous effect.`,
        declared: "explicit effect required",
        advertised: "no valid effect",
      });
    }

    // Local evidence persistence stays within a read-only product boundary;
    // external and repository effects do not.
    if (safety.read_only && exceedsReadOnlyBoundary(action)) {
      findings.push({
        productKey: product.key,
        severity: "critical",
        kind: "read-only violated",
        detail: `Action \`${action.id}\` exceeds the product's read-only external boundary.`,
        declared: "read_only = true",
        advertised: effectKind(action) ?? "invalid effect",
      });
    }

    if (!safety.opens_pull_requests && opensPullRequest(action)) {
      findings.push({
        productKey: product.key,
        severity: "critical",
        kind: "undeclared PR capability",
        detail: `Action \`${action.id}\` opens pull requests.`,
        declared: "opens_pull_requests = false",
        advertised: "opens_pr = true",
      });
    }

    // Behaviour check, not a declaration check: read-only plus a write credential
    // means one of the two is lying, and the credential is the harder thing to fake.
    if (!isMutating(action)) {
      const writes = writeScopes(action);
      if (writes.length > 0) {
        // Two distinct defects reach here and they deserve different words.
        // An action that says `read_only = true` while holding a write scope
        // contradicts itself. An action that says nothing is read as read-only by
        // every consumer of the contract, including this deck — silence is not a
        // safer default, it is an unstated claim. Reporting both as "declares
        // read-only" sends you looking for a `read_only` line that isn't there.
        findings.push({
          productKey: product.key,
          severity: "critical",
          kind: "read-only needs write credential",
          detail: `Action \`${action.id}\` declares read-only but requires ${writes.join(", ")}.`,
          declared: "read_only action",
          advertised: writes.join(", "),
        });
      }
    }

    for (const requirement of action.credential_requirements ?? []) {
      if (!scopes.has(requirement)) {
        findings.push({
          productKey: product.key,
          severity: "warning",
          kind: "undeclared credential",
          detail: `Action \`${action.id}\` requires \`${requirement}\`.`,
          declared: safety.credential_scopes.join(", ") || "no scopes declared",
          advertised: requirement,
        });
      }
    }
  }

  // Existence claims: the manifest promises a capability no action provides.
  if (safety.opens_pull_requests && !actions.some(opensPullRequest)) {
    findings.push({
      productKey: product.key,
      severity: "warning",
      kind: "declared but not offered",
      detail: "Manifest claims pull-request capability; no advertised action opens one.",
      declared: "opens_pull_requests = true",
      advertised: "no opens_pr action",
    });
  }

  if (
    safety.requires_operator_approval &&
    actions.length > 0 &&
    !actions.some(requiresApproval)
  ) {
    findings.push({
      productKey: product.key,
      severity: "warning",
      kind: "approval not implemented",
      detail:
        "Manifest declares operator approval; no advertised action carries requires_approval. Expected until the suite approval flow exists — HiveCore refuses approval-gated dispatch today.",
      declared: "requires_operator_approval = true",
      advertised: "no approval-gated action",
    });
  }

  // Under-claim: an action is gated but the manifest never says approval applies.
  if (!safety.requires_operator_approval && actions.some(requiresApproval)) {
    findings.push({
      productKey: product.key,
      severity: "warning",
      kind: "posture under-declared",
      detail: "An advertised action requires approval; the manifest does not say so.",
      declared: "requires_operator_approval = false",
      advertised: "approval-gated action",
    });
  }

  return findings;
}

export async function fetchConformance(signal?: AbortSignal): Promise<ProductConformance[]> {
  const [productsResponse, capabilitiesResponse] = await Promise.all([
    apiFetch("/api/products", { signal }),
    apiFetch("/api/products/capabilities", { signal }),
  ]);
  if (!productsResponse.ok || !capabilitiesResponse.ok) {
    throw new Error(`HTTP ${productsResponse.status}/${capabilitiesResponse.status}`);
  }

  const products = (await productsResponse.json()) as ApiProduct[];
  const reports = (await capabilitiesResponse.json()) as ApiCapabilityReport[];
  const byKey = new Map(reports.map((report) => [report.key, report]));

  return products.map((product) => {
    const report = byKey.get(product.key);
    const actions = report?.advertised?.actions ?? [];
    const observed = Boolean(report?.advertised);
    return {
      productKey: product.key,
      productName: product.name,
      observed,
      actionCount: actions.length,
      // Absence of observation is not drift.
      findings: observed ? compare(product, actions) : [],
    };
  });
}

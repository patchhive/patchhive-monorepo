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
// sides state it independently: the manifest in [safety], each action in its own
// mutating / opens_pr / requires_approval / credential_requirements flags. A product
// declaring read_only while advertising a mutating action is a real conformance
// failure — the exact case docs/hivecore-architecture.md §3.14 calls out.

import { API } from "@/config";

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

interface ApiAction {
  id: string;
  label: string;
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

function isMutating(action: ApiAction): boolean {
  return Boolean(action.mutating) || action.read_only === false;
}

function compare(product: ApiProduct, actions: ApiAction[]): ConformanceFinding[] {
  const findings: ConformanceFinding[] = [];
  const safety = product.safety;
  const scopes = new Set(safety.credential_scopes);

  for (const action of actions) {
    // A read-only product advertising a mutating action is the headline failure.
    if (safety.read_only && isMutating(action)) {
      findings.push({
        productKey: product.key,
        severity: "critical",
        kind: "read-only violated",
        detail: `Action \`${action.id}\` is mutating.`,
        declared: "read_only = true",
        advertised: "mutating action",
      });
    }

    if (!safety.opens_pull_requests && action.opens_pr) {
      findings.push({
        productKey: product.key,
        severity: "critical",
        kind: "undeclared PR capability",
        detail: `Action \`${action.id}\` opens pull requests.`,
        declared: "opens_pull_requests = false",
        advertised: "opens_pr = true",
      });
    }

    if (safety.requires_operator_approval && isMutating(action) && !action.requires_approval) {
      findings.push({
        productKey: product.key,
        severity: "warning",
        kind: "approval gap",
        detail: `Mutating action \`${action.id}\` does not require approval.`,
        declared: "requires_operator_approval = true",
        advertised: "requires_approval = false",
      });
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

  // Declared-but-not-offered: the manifest promises a boundary the engine never uses.
  if (safety.opens_pull_requests && !actions.some((action) => action.opens_pr)) {
    findings.push({
      productKey: product.key,
      severity: "warning",
      kind: "declared but not offered",
      detail: "Manifest claims pull-request capability; no advertised action opens one.",
      declared: "opens_pull_requests = true",
      advertised: "no opens_pr action",
    });
  }

  return findings;
}

export async function fetchConformance(signal?: AbortSignal): Promise<ProductConformance[]> {
  const [productsResponse, capabilitiesResponse] = await Promise.all([
    fetch(`${API}/api/products`, { signal }),
    fetch(`${API}/api/products/capabilities`, { signal }),
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

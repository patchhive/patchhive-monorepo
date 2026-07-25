// Live suite state from the backend.
//
// GET /api/products is the registry — identity, safety posture, declared
// capabilities, and current status, all served from the product manifests. That
// makes it the authority; the static table in hive-data.ts is only the fallback
// when the control plane is unreachable, so the deck degrades to "here is what the
// suite is" instead of a blank page.

import { useEffect, useState } from "react";

import { apiFetch } from "./api";
import {
  PRODUCTS as FALLBACK_PRODUCTS,
  type MigrationStage,
  type Product,
  type ProductStatus,
  type RunSummary,
} from "./hive-data";
import type { SuiteEvent, SuiteEventKind, SuiteEventLevel } from "./suite-state";

interface ApiCapability {
  id: string;
  label: string;
  description: string;
  mutating: boolean;
}

interface ApiProduct {
  key: string;
  name: string;
  code: string;
  role: string;
  enabled: boolean;
  status: string;
  migration_stage: string;
  route_prefix: string;
  capabilities: string[];
  capability_metadata: ApiCapability[];
  safety: {
    read_only: boolean;
    writes_external_state: boolean;
    mutates_repositories: boolean;
    opens_pull_requests: boolean;
    requires_operator_approval: boolean;
    credential_scopes: string[];
    evidence_required: string[];
  };
}

interface ApiEvent {
  id: string;
  kind: string;
  message: string;
  created_at: string;
}

const STATUS_MAP: Record<string, ProductStatus> = {
  online: "online",
  degraded: "degraded",
  offline: "offline",
  unconfigured: "unconfigured",
  disabled: "disabled",
  "engine-pending": "unconfigured",
};

const STAGE_MAP: Record<string, MigrationStage> = {
  integrated: "integrated",
  "in-progress": "in-progress",
  "not-started": "not-started",
};

/** Ports stay client-side: the registry API does not carry launch URLs. */
const PORTS = Object.fromEntries(
  FALLBACK_PRODUCTS.map((product) => [
    product.key,
    { frontendPort: product.frontendPort, apiPort: product.apiPort },
  ]),
);

function toProduct(item: ApiProduct, observedAt: string): Product {
  const ports = PORTS[item.key] ?? { frontendPort: 0, apiPort: 0 };
  return {
    key: item.key,
    code: item.code,
    name: item.name,
    role: item.role,
    routePrefix: item.route_prefix,
    migrationStage: STAGE_MAP[item.migration_stage] ?? "not-started",
    frontendPort: ports.frontendPort,
    apiPort: ports.apiPort,
    safety: {
      readOnly: item.safety.read_only,
      writesExternalState: item.safety.writes_external_state,
      mutatesRepositories: item.safety.mutates_repositories,
      opensPullRequests: item.safety.opens_pull_requests,
      requiresOperatorApproval: item.safety.requires_operator_approval,
      credentialScopes: item.safety.credential_scopes,
      evidenceRequired: item.safety.evidence_required,
    },
    declared: item.capability_metadata.map((capability) => ({
      id: capability.id,
      label: capability.label,
      description: capability.description,
    })),
    observed: {
      status: item.enabled ? (STATUS_MAP[item.status] ?? "unknown") : "disabled",
      actions: item.capabilities,
      // /api/products does not carry startup checks; unknown, not zero.
      startupErrors: null,
      startupWarns: null,
      driftCount: 0,
      runCount: 0,
      observedAt,
    },
  };
}

const EVENT_KIND: Record<string, SuiteEventKind> = {
  "backend.started": "operator",
  "product.dispatch": "dispatch",
  "policy.decision": "policy_decision",
  "budget.reserved": "budget",
  "budget.denied": "budget",
  "approval.requested": "approval",
  "product.state": "product_state",
};

function eventLevel(kind: string): SuiteEventLevel {
  if (kind.includes("denied") || kind.includes("failed")) return "crit";
  if (kind.includes("warn") || kind.includes("blocked")) return "warn";
  return "info";
}

function toEvent(item: ApiEvent): SuiteEvent {
  return {
    id: item.id,
    ts: item.created_at,
    kind: EVENT_KIND[item.kind] ?? "operator",
    level: eventLevel(item.kind),
    actor: "patchhive-backend",
    productKey: null,
    repository: null,
    runId: null,
    operation: item.kind,
    reasonChain: [],
    summary: item.message,
  };
}

export interface SuiteData {
  products: Product[];
  events: SuiteEvent[];
  runs: RunSummary[];
  live: boolean;
  error: string;
  loading: boolean;
}

export function useSuiteData(pollMs = 10_000): SuiteData {
  const [state, setState] = useState<SuiteData>({
    products: FALLBACK_PRODUCTS,
    events: [],
    runs: [],
    live: false,
    error: "",
    loading: true,
  });

  useEffect(() => {
    let cancelled = false;

    async function load() {
      try {
        const observedAt = new Date().toISOString();
        const [products, events] = await Promise.all([
          apiFetch<ApiProduct[]>("/api/products"),
          apiFetch<ApiEvent[]>("/api/events").catch(() => [] as ApiEvent[]),
        ]);
        if (cancelled) return;
        setState({
          products: products.map((item) => toProduct(item, observedAt)),
          events: events.map(toEvent),
          runs: [],
          live: true,
          error: "",
          loading: false,
        });
      } catch (cause) {
        if (cancelled) return;
        setState((current) => ({
          ...current,
          products: FALLBACK_PRODUCTS,
          live: false,
          loading: false,
          error: cause instanceof Error ? cause.message : "Control plane unreachable.",
        }));
      }
    }

    load();
    const timer = setInterval(load, pollMs);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [pollMs]);

  return state;
}

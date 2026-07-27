// Service-token status, read from each product's /auth/status.
//
// This is the honest version of a "token vault": HiveCore really is the suite's
// token broker — it mints, rotates, stores encrypted, and detects stale tokens
// server-side — but the browser must never receive credential material. /auth/status
// returns only posture: is a service token configured, is it scoped or legacy, has
// it expired, which scopes does it carry, when was it rotated. No token values.
//
// One request. GET /api/products/auth-status is a server-side aggregate: every
// product engine is compiled into the unified backend, so it reads each engine's
// auth posture in-process and returns them together. The browser previously fanned
// out one call per product, which is N requests per refresh and enough traffic to
// trip the sensitive-route rate limiter — blocker B2 in the architecture doc, fixed
// at the source rather than papered over with a smaller client-side pool.

import { apiFetch } from "./http";
import { PRODUCTS } from "./hive-data";

/** Slugs are kebab-case in the API; the deck's product ids are not. */
const SLUGS: Record<string, string> = {
  reporeaper: "repo-reaper",
  signalhive: "signal-hive",
  trustgate: "trust-gate",
  repomemory: "repo-memory",
  reviewbee: "review-bee",
  mergekeeper: "merge-keeper",
  flakesting: "flake-sting",
  deptriage: "dep-triage",
  vulntriage: "vuln-triage",
  refactorscout: "refactor-scout",
  releasesentry: "release-sentry",
  hivecore: "hive-core",
};

interface AuthStatusBody {
  auth_enabled?: boolean;
  service_auth_supported?: boolean;
  service_auth_configured?: boolean;
  service_auth_enabled?: boolean;
  service_auth_scoped?: boolean;
  service_auth_legacy?: boolean;
  service_auth_expired?: boolean;
  service_auth_expires_soon?: boolean;
  service_auth_scopes?: string[];
  service_auth_known_scopes?: string[];
  suite_bootstrap_enabled?: boolean;
  service_auth_token?: {
    name?: string | null;
    created_at?: string | null;
    rotated_at?: string | null;
    expires_at?: string | null;
    /** Identifier, not the secret. Truncated further before display. */
    fingerprint?: string | null;
  } | null;
}

export type TokenState =
  | "broker"
  | "rate_limited"
  | "healthy"
  | "legacy"
  | "expiring"
  | "expired"
  | "missing"
  | "unsupported"
  | "unreachable";

export interface TokenStatus {
  productId: string;
  productName: string;
  slug: string;
  state: TokenState;
  /** Scopes the token actually carries. */
  scopes: string[];
  /** Scopes the product recognises, so gaps are visible. */
  knownScopes: string[];
  name: string;
  rotatedAt: string | null;
  expiresAt: string | null;
  fingerprint: string;
  suiteBootstrap: boolean;
  detail: string;
}

export const TOKEN_STATE_LABEL: Record<TokenState, string> = {
  broker: "broker",
  rate_limited: "rate limited",
  healthy: "scoped",
  legacy: "legacy",
  expiring: "expiring",
  expired: "expired",
  missing: "not provisioned",
  unsupported: "unsupported",
  unreachable: "unreachable",
};

function classify(body: AuthStatusBody): { state: TokenState; detail: string } {
  if (!body.service_auth_supported) {
    return {
      state: "unsupported",
      detail: "Product does not advertise service-token auth.",
    };
  }
  if (!body.service_auth_configured || !body.service_auth_enabled) {
    return {
      state: "missing",
      detail: "No service token provisioned. HiveCore cannot read protected runs or dispatch actions.",
    };
  }
  if (body.service_auth_expired) {
    return { state: "expired", detail: "Token has expired. Rotate before dispatching." };
  }
  if (body.service_auth_legacy || !body.service_auth_scoped) {
    return {
      state: "legacy",
      detail: "Legacy token grants runs:read only. Rotate to gain actions:dispatch.",
    };
  }
  if (body.service_auth_expires_soon) {
    return { state: "expiring", detail: "Token expires soon. Rotate before it lapses." };
  }
  return { state: "healthy", detail: "Scoped token active." };
}

function blank(productId: string, productName: string): Omit<TokenStatus, "state" | "detail"> {
  return {
    productId,
    productName,
    slug: SLUGS[productId] ?? productId,
    scopes: [],
    knownScopes: [],
    name: "",
    rotatedAt: null,
    expiresAt: null,
    fingerprint: "",
    suiteBootstrap: false,
  };
}

/** Fingerprints identify a token; still truncated so nothing long-lived is shown. */
function shortFingerprint(raw: string | null | undefined): string {
  if (!raw) return "";
  return raw.replace(/[^A-Za-z0-9_-]/g, "").slice(0, 10);
}

interface AggregateRow {
  key: string;
  /** null when the engine is not enabled in this runtime. */
  status: AuthStatusBody | null;
}

export async function fetchTokenStatuses(signal?: AbortSignal): Promise<TokenStatus[]> {
  let rows: AggregateRow[];
  try {
    const response = await apiFetch("/api/products/auth-status", { signal });
    if (!response.ok) {
      return PRODUCTS.map((product) => ({
        ...blank(product.id, product.name),
        state: "unreachable" as const,
        detail: `HTTP ${response.status} from /api/products/auth-status.`,
      }));
    }
    rows = (await response.json()) as AggregateRow[];
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === "AbortError") throw cause;
    return PRODUCTS.map((product) => ({
      ...blank(product.id, product.name),
      state: "unreachable" as const,
      detail: "Could not reach the control plane.",
    }));
  }

  const bySlug = new Map(rows.map((row) => [row.key, row]));

  return PRODUCTS.map((product) => {
    const slug = SLUGS[product.id] ?? product.id;
    const base = { ...blank(product.id, product.name), slug };
    const row = bySlug.get(slug);

    if (!row) {
      return {
        ...base,
        state: "unsupported" as const,
        detail: "Engine is not mounted in this runtime.",
      };
    }
    if (!row.status) {
      return {
        ...base,
        state: "unsupported" as const,
        detail: "Product is not enabled in this runtime.",
      };
    }

    // HiveCore holds the other products' tokens; it is not a dispatch target for
    // itself, and its provisioning endpoint refuses self-provisioning outright.
    // Before this, it classified as "not provisioned" — technically true, but it
    // read as a gap to close and offered a Provision button that could only fail.
    // A component that cannot be in a state should not render a control for it.
    if (slug === "hive-core") {
      return {
        ...base,
        state: "broker" as const,
        detail:
          "HiveCore issues service tokens; it does not hold one for itself. Operators authenticate to it with the suite key.",
        knownScopes: row.status.service_auth_known_scopes ?? [],
      };
    }

    const body = row.status;
    const { state, detail } = classify(body);
    return {
      ...base,
      state,
      detail,
      scopes: body.service_auth_scopes ?? [],
      knownScopes: body.service_auth_known_scopes ?? [],
      name: body.service_auth_token?.name ?? "",
      rotatedAt: body.service_auth_token?.rotated_at ?? body.service_auth_token?.created_at ?? null,
      expiresAt: body.service_auth_token?.expires_at ?? null,
      fingerprint: shortFingerprint(body.service_auth_token?.fingerprint),
      suiteBootstrap: Boolean(body.suite_bootstrap_enabled),
    };
  });
}

// The suite-level provisioning helper lived here and is gone: it minted a token the
// backend then discarded, so the product ended up authenticated while HiveCore still
// could not dispatch to it. Provisioning now goes through HiveCore's own route,
// which stores the token where its dispatcher reads from — see lib/dispatch.ts.

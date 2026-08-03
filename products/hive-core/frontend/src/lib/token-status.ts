// Service-token status, read from each product's /auth/status.
//
// This is the honest version of a "token vault": HiveCore really is the suite's
// token broker — it mints, rotates, stores encrypted, and detects stale tokens
// server-side — but the browser must never receive credential material. /auth/status
// returns only posture: is a service token configured, is it scoped or legacy, has
// it expired, which scopes does it carry, when was it rotated. No token values.
//
// One request reads HiveCore's durable runtime snapshot. Its background poller
// observes each product through the normal HTTP router, including authentication,
// rate limiting, and telemetry, so the browser neither fans out nor bypasses product
// middleware.

import { apiFetch } from "./http";
import { PRODUCTS } from "./hive-data";
import { registerProductSlug, slugForId } from "./product-slugs";

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
    slug: slugForId(productId),
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
  slug: string;
  title: string;
  auth_status:
    | { state: "observed"; value: AuthStatusBody }
    | { state: "failed" | "not_observed" | "not_applicable"; reason: string };
}

export async function fetchTokenStatuses(signal?: AbortSignal): Promise<TokenStatus[]> {
  let rows: AggregateRow[];
  try {
    const response = await apiFetch("/api/products/runtime", { signal });
    if (!response.ok) {
      return PRODUCTS.map((product) => ({
        ...blank(product.id, product.name),
        state: "unreachable" as const,
        detail: `HTTP ${response.status} from /api/products/runtime.`,
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

  return rows.map((row) => {
    const slug = row.slug;
    const productId = registerProductSlug(slug, row.title);
    const base = { ...blank(productId, row.title), slug };
    if (row.auth_status.state !== "observed") {
      return {
        ...base,
        state: row.auth_status.state === "not_applicable" ? ("unsupported" as const) : ("unreachable" as const),
        detail: row.auth_status.reason,
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
        knownScopes: row.auth_status.value.service_auth_known_scopes ?? [],
      };
    }

    const body = row.auth_status.value;
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

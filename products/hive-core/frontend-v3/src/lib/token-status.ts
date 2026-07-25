// Service-token status, read from each product's /auth/status.
//
// This is the honest version of a "token vault": HiveCore really is the suite's
// token broker — it mints, rotates, stores encrypted, and detects stale tokens
// server-side — but the browser must never receive credential material. /auth/status
// returns only posture: is a service token configured, is it scoped or legacy, has
// it expired, which scopes does it carry, when was it rotated. No token values.
//
// The fan-out is per product because the backend has no aggregate endpoint yet.
// That is the same N-call problem the architecture doc calls blocker B2; when a
// server-side snapshot exists this collapses to one request.

import { API } from "@/config";
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

/** Fingerprints identify a token; still truncated so nothing long-lived is shown. */
function shortFingerprint(raw: string | null | undefined): string {
  if (!raw) return "";
  return raw.replace(/[^A-Za-z0-9_-]/g, "").slice(0, 10);
}

/** Small pool: responsive enough, low enough to stay under the sensitive-route budget. */
const CONCURRENCY = 3;

async function pooled<T, R>(items: T[], worker: (item: T) => Promise<R>, limit = CONCURRENCY): Promise<R[]> {
  const results = new Array<R>(items.length);
  let cursor = 0;
  const runners = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (cursor < items.length) {
      const index = cursor;
      cursor += 1;
      results[index] = await worker(items[index]);
    }
  });
  await Promise.all(runners);
  return results;
}

export async function fetchTokenStatuses(signal?: AbortSignal): Promise<TokenStatus[]> {
  const results = await pooled(
    PRODUCTS,
    async (product): Promise<TokenStatus> => {
      const slug = SLUGS[product.id] ?? product.id;
      const base: Omit<TokenStatus, "state" | "detail"> = {
        productId: product.id,
        productName: product.name,
        slug,
        scopes: [],
        knownScopes: [],
        name: "",
        rotatedAt: null,
        expiresAt: null,
        fingerprint: "",
        suiteBootstrap: false,
      };

      try {
        const response = await fetch(`${API}/api/products/${slug}/auth/status`, { signal });
        if (response.status === 429) {
          return {
            ...base,
            state: "rate_limited",
            detail:
              "Rate limited. /auth/status counts against the sensitive-route budget; this panel needs a server-side aggregate.",
          };
        }
        if (!response.ok) {
          return { ...base, state: "unreachable", detail: `HTTP ${response.status} from /auth/status.` };
        }
        const body = (await response.json()) as AuthStatusBody;
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
      } catch (cause) {
        if (cause instanceof DOMException && cause.name === "AbortError") throw cause;
        return { ...base, state: "unreachable", detail: "Could not reach /auth/status." };
      }
    },
  );

  return results;
}

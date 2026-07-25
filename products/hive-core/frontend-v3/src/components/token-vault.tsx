import { useEffect, useMemo, useState } from "react";
import { KeyRound, Loader2, RefreshCw, ShieldAlert } from "lucide-react";

import {
  fetchTokenStatuses,
  TOKEN_STATE_LABEL,
  type TokenState,
  type TokenStatus,
} from "@/lib/token-status";

/**
 * Service-token posture across the suite.
 *
 * Not a vault: no token values, no secrets, nothing the browser should not hold.
 * Only what /auth/status reports — configured, scoped or legacy, expired, which
 * scopes it carries — because that is the operational question. A legacy token that
 * only grants runs:read means HiveCore silently cannot dispatch to that product.
 */
const stateMeta: Record<TokenState, { dot: string; text: string; ring: string }> = {
  healthy: {
    dot: "bg-[var(--ok)] shadow-[0_0_8px_var(--ok)]",
    text: "text-[var(--ok)]",
    ring: "border-[var(--ok)]/40",
  },
  legacy: {
    dot: "bg-[var(--warn)] shadow-[0_0_8px_var(--warn)]",
    text: "text-[var(--warn)]",
    ring: "border-[var(--warn)]/40",
  },
  expiring: {
    dot: "bg-[var(--warn)] shadow-[0_0_8px_var(--warn)]",
    text: "text-[var(--warn)]",
    ring: "border-[var(--warn)]/40",
  },
  expired: {
    dot: "bg-[var(--crit)] shadow-[0_0_8px_var(--crit)]",
    text: "text-[var(--crit)]",
    ring: "border-[var(--crit)]/40",
  },
  missing: {
    dot: "bg-[var(--crit)] shadow-[0_0_8px_var(--crit)]",
    text: "text-[var(--crit)]",
    ring: "border-[var(--crit)]/40",
  },
  rate_limited: {
    dot: "bg-[var(--warn)] shadow-[0_0_8px_var(--warn)]",
    text: "text-[var(--warn)]",
    ring: "border-[var(--warn)]/40",
  },
  unsupported: { dot: "bg-muted-foreground", text: "text-muted-foreground", ring: "border-border" },
  unreachable: { dot: "bg-muted-foreground", text: "text-muted-foreground", ring: "border-border" },
};

const REQUIRED_SCOPE = "actions:dispatch";

export function TokenVault() {
  const [rows, setRows] = useState<TokenStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    fetchTokenStatuses(controller.signal)
      .then((next) => setRows(next))
      .catch(() => undefined)
      .finally(() => setLoading(false));
    return () => controller.abort();
  }, [nonce]);

  const summary = useMemo(() => {
    const count = (state: TokenState) => rows.filter((row) => row.state === state).length;
    return {
      healthy: count("healthy"),
      needsRotation: count("legacy") + count("expiring") + count("expired"),
      missing: count("missing"),
      cannotDispatch: rows.filter(
        (row) =>
          row.state !== "unsupported" &&
          row.state !== "rate_limited" &&
          row.state !== "unreachable" &&
          !row.scopes.includes(REQUIRED_SCOPE),
      ).length,
    };
  }, [rows]);

  return (
    <section
      id="tokens"
      className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur"
    >
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <KeyRound className="h-4 w-4 text-[var(--honey)]" /> Service Tokens
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Per-product token posture from <code className="font-mono text-[11px]">/auth/status</code>.
            Status only — no token material reaches the browser.
          </p>
        </div>
        <div className="flex items-center gap-2">
          {summary.cannotDispatch > 0 && (
            <span className="inline-flex items-center gap-1 rounded border border-[var(--warn)]/40 bg-[var(--warn)]/10 px-2 py-1 font-display text-[10px] uppercase tracking-wider text-[var(--warn)]">
              <ShieldAlert className="h-3 w-3" />
              {summary.cannotDispatch} cannot dispatch
            </span>
          )}
          <button
            onClick={() => setNonce((value) => value + 1)}
            className="inline-flex items-center gap-1.5 rounded border border-border bg-card/60 px-2 py-1 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
          >
            {loading ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <RefreshCw className="h-3 w-3" />
            )}
            Refresh
          </button>
        </div>
      </div>

      {loading && rows.length === 0 ? (
        <div className="flex items-center justify-center gap-2 rounded-lg border border-dashed border-border/70 py-10 text-xs text-muted-foreground">
          <Loader2 className="h-3.5 w-3.5 animate-spin" /> Polling each product for token posture…
        </div>
      ) : (
        <div className="grid gap-2 md:grid-cols-2">
          {rows.map((row) => {
            const meta = stateMeta[row.state];
            const canDispatch = row.scopes.includes(REQUIRED_SCOPE);
            return (
              <div
                key={row.productId}
                className={`rounded-lg border ${meta.ring} bg-background/40 p-3`}
              >
                <div className="flex flex-wrap items-center gap-2">
                  <span className={`h-1.5 w-1.5 rounded-full ${meta.dot}`} />
                  <span className="font-display text-xs font-bold text-foreground">
                    {row.productName}
                  </span>
                  <span
                    className={`rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${meta.ring} ${meta.text}`}
                  >
                    {TOKEN_STATE_LABEL[row.state]}
                  </span>
                  {row.fingerprint && (
                    <code
                      title="Token fingerprint — an identifier, not the secret"
                      className="font-mono text-[9px] text-muted-foreground"
                    >
                      {row.fingerprint}
                    </code>
                  )}
                </div>

                <p className="mt-1 text-[11px] leading-snug text-muted-foreground">{row.detail}</p>

                {row.knownScopes.length > 0 && (
                  <div className="mt-2 flex flex-wrap gap-1">
                    {row.knownScopes.map((scope) => {
                      const held = row.scopes.includes(scope);
                      return (
                        <span
                          key={scope}
                          title={held ? "Granted" : "Not granted by this token"}
                          className={`rounded border px-1.5 py-0.5 font-mono text-[9px] ${
                            held
                              ? "border-[var(--ok)]/40 text-[var(--ok)]"
                              : "border-border text-muted-foreground line-through"
                          }`}
                        >
                          {scope}
                        </span>
                      );
                    })}
                  </div>
                )}

                <div className="mt-2 flex flex-wrap gap-3 font-mono text-[9px] text-muted-foreground">
                  {row.rotatedAt && (
                    <span>rotated {new Date(row.rotatedAt).toLocaleDateString()}</span>
                  )}
                  {row.expiresAt && (
                    <span>expires {new Date(row.expiresAt).toLocaleDateString()}</span>
                  )}
                  {!canDispatch &&
                    row.state !== "unsupported" &&
                    row.state !== "rate_limited" &&
                    row.state !== "unreachable" && (
                      <span className="text-[var(--warn)]">dispatch blocked</span>
                    )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      <p className="mt-4 text-[11px] text-muted-foreground">
        Provisioning and rotation are server-side operations owned by HiveCore. This panel reports
        posture; it does not hold, display, or transmit tokens.
      </p>
    </section>
  );
}

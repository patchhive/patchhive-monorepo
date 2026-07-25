import { useEffect, useMemo, useState } from "react";
import { KeyRound, RefreshCw, ShieldAlert } from "lucide-react";
import { toast } from "sonner";
import { TOKENS, type TokenStatus, type VaultToken } from "@/lib/hive-extra";

const statusMeta: Record<TokenStatus, { label: string; dot: string; text: string; ring: string }> = {
  active:   { label: "active",   dot: "bg-[var(--ok)] shadow-[0_0_8px_var(--ok)]",     text: "text-[var(--ok)]",   ring: "border-[var(--ok)]/40" },
  expiring: { label: "expiring", dot: "bg-[var(--warn)] shadow-[0_0_8px_var(--warn)]", text: "text-[var(--warn)]", ring: "border-[var(--warn)]/40" },
  rotated:  { label: "rotated",  dot: "bg-muted-foreground",                           text: "text-muted-foreground", ring: "border-border" },
  revoked:  { label: "revoked",  dot: "bg-[var(--crit)] shadow-[0_0_8px_var(--crit)]", text: "text-[var(--crit)]", ring: "border-[var(--crit)]/40" },
};

function daysUntil(iso: string) {
  return Math.round((Date.parse(iso) - Date.now()) / 86_400_000);
}

export function TokenVault() {
  const [filter, setFilter] = useState<TokenStatus | "all">("all");
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  const filtered = useMemo(
    () => (filter === "all" ? TOKENS : TOKENS.filter((t) => t.status === filter)),
    [filter],
  );
  const counts = useMemo(() => {
    return {
      active: TOKENS.filter((t) => t.status === "active").length,
      expiring: TOKENS.filter((t) => t.status === "expiring").length,
      rotated: TOKENS.filter((t) => t.status === "rotated").length,
      revoked: TOKENS.filter((t) => t.status === "revoked").length,
    };
  }, []);

  const rotate = (tok: VaultToken) => {
    toast.success(`${tok.name} rotated`, { description: `New token issued · scope ${tok.scope}` });
  };

  return (
    <section id="tokens" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <KeyRound className="h-4 w-4 text-[var(--honey)]" /> Token Vault
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Service tokens issued, rotated, or revoked through TrustGate. Expiring tokens are auto-flagged 7 days out.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2 font-display text-[10px] uppercase tracking-wider">
          {(["all","active","expiring","rotated","revoked"] as const).map((f) => (
            <button
              key={f}
              onClick={() => setFilter(f)}
              className={`rounded border px-2 py-0.5 transition ${
                filter === f
                  ? "border-[var(--honey)]/60 bg-[var(--honey)]/15 text-[var(--honey)]"
                  : "border-border text-muted-foreground hover:text-foreground"
              }`}
            >
              {f}
              {f !== "all" && <span className="ml-1 text-muted-foreground/80">{counts[f]}</span>}
            </button>
          ))}
        </div>
      </div>

      {counts.expiring > 0 && (
        <div className="mb-4 flex items-start gap-3 rounded-lg border border-[var(--warn)]/40 bg-[var(--warn)]/10 p-3">
          <ShieldAlert className="mt-0.5 h-4 w-4 shrink-0 text-[var(--warn)]" />
          <div className="text-xs">
            <span className="font-display font-bold uppercase tracking-wider text-[var(--warn)]">
              {counts.expiring} token{counts.expiring === 1 ? "" : "s"} expiring
            </span>{" "}
            <span className="text-muted-foreground">— rotate before TTL hits zero to avoid silent auth failures.</span>
          </div>
        </div>
      )}

      <div className="overflow-x-auto rounded-lg border border-border bg-background/30">
        <table className="w-full min-w-[640px] border-collapse text-left">
          <thead>
            <tr className="border-b border-border/60 bg-card/40 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
              <th className="px-3 py-2">Token</th>
              <th className="px-3 py-2">Scope</th>
              <th className="px-3 py-2">Status</th>
              <th className="px-3 py-2">Expires</th>
              <th className="px-3 py-2">Last used</th>
              <th className="px-3 py-2 text-right">Action</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((t) => {
              const meta = statusMeta[t.status];
              const days = daysUntil(t.expiresAt);
              return (
                <tr key={t.id} className="border-b border-border/30 transition hover:bg-muted/20">
                  <td className="px-3 py-2">
                    <div className="flex items-center gap-2">
                      <span className={`h-1.5 w-1.5 rounded-full ${meta.dot}`} />
                      <span className="font-display text-xs font-semibold">{t.name}</span>
                    </div>
                    <code className="ml-3.5 font-display text-[10px] text-muted-foreground">{t.id}</code>
                  </td>
                  <td className="px-3 py-2">
                    <code className="rounded border border-border bg-muted/40 px-1.5 py-0.5 font-display text-[10px] text-[var(--honey)]">
                      {t.scope}
                    </code>
                  </td>
                  <td className="px-3 py-2">
                    <span className={`rounded border ${meta.ring} px-1.5 py-0.5 font-display text-[10px] uppercase tracking-wider ${meta.text}`}>
                      {meta.label}
                    </span>
                  </td>
                  <td className="px-3 py-2 font-display text-[11px]">
                    {!mounted ? (
                      <span className="text-muted-foreground">—</span>
                    ) : t.status === "revoked" || t.status === "rotated" ? (
                      <span className="text-muted-foreground">—</span>
                    ) : days < 0 ? (
                      <span className="text-[var(--crit)]">{Math.abs(days)}d overdue</span>
                    ) : (
                      <span className={days <= 7 ? "text-[var(--warn)]" : "text-foreground/80"}>
                        in {days}d
                      </span>
                    )}
                  </td>
                  <td className="px-3 py-2 font-display text-[10px] text-muted-foreground">
                    {mounted ? new Date(t.lastUsedAt).toLocaleString(undefined, { dateStyle: "short", timeStyle: "short" }) : "—"}
                  </td>
                  <td className="px-3 py-2 text-right">
                    <button
                      onClick={() => rotate(t)}
                      disabled={t.status === "revoked"}
                      className="inline-flex items-center gap-1 rounded border border-border bg-card/60 px-2 py-0.5 font-display text-[10px] uppercase tracking-wider text-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)] disabled:opacity-40"
                    >
                      <RefreshCw className="h-3 w-3" /> rotate
                    </button>
                  </td>
                </tr>
              );
            })}
            {filtered.length === 0 && (
              <tr>
                <td colSpan={6} className="px-3 py-6 text-center font-display text-[10px] uppercase tracking-wider text-muted-foreground">
                  no tokens match this filter
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}

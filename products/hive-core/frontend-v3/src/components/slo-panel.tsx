import { useMemo } from "react";
import { Gauge } from "lucide-react";
import { PRODUCTS } from "@/lib/hive-data";
import { SLOS, errorBudgetBurn } from "@/lib/hive-extra";

export function SLOPanel() {
  const rows = useMemo(() => {
    const byId = Object.fromEntries(PRODUCTS.map((p) => [p.id, p]));
    return SLOS.map((s) => {
      const p = byId[s.productId];
      const burn = errorBudgetBurn(p?.uptime ?? 1, s.target);
      return { slo: s, product: p, burn };
    }).sort((a, b) => b.burn - a.burn);
  }, []);

  return (
    <section id="slo" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-5 flex items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <Gauge className="h-4 w-4 text-[var(--honey)]" /> SLO &amp; Error Budget
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Per-product uptime targets vs measured 30-day actuals. Burn = fraction of error budget consumed.
          </p>
        </div>
        <div className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          {rows.filter((r) => r.burn > 1).length} breached · {rows.filter((r) => r.burn > 0.5 && r.burn <= 1).length} at risk
        </div>
      </div>

      <div className="grid gap-px overflow-hidden rounded-lg bg-border/40 md:grid-cols-2">
        {rows.map(({ slo, product, burn }) => {
          if (!product) return null;
          const tone =
            burn > 1 ? "var(--crit)" : burn > 0.5 ? "var(--warn)" : "var(--ok)";
          const pct = Math.min(120, Math.round(burn * 100));
          return (
            <div key={slo.productId} className="bg-card/80 p-4">
              <div className="flex items-center justify-between gap-2">
                <span className="font-display text-xs font-bold">{product.name}</span>
                <span className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">
                  window {slo.window}
                </span>
              </div>
              <div className="mt-2 grid grid-cols-3 gap-2 font-display text-[10px]">
                <div className="rounded border border-border/60 bg-background/40 px-2 py-1.5">
                  <div className="text-[8px] uppercase tracking-wider text-muted-foreground">target</div>
                  <div className="mt-0.5 text-xs font-bold">{(slo.target * 100).toFixed(2)}%</div>
                </div>
                <div className="rounded border border-border/60 bg-background/40 px-2 py-1.5">
                  <div className="text-[8px] uppercase tracking-wider text-muted-foreground">actual</div>
                  <div className="mt-0.5 text-xs font-bold" style={{ color: tone }}>
                    {(product.uptime * 100).toFixed(2)}%
                  </div>
                </div>
                <div className="rounded border border-border/60 bg-background/40 px-2 py-1.5">
                  <div className="text-[8px] uppercase tracking-wider text-muted-foreground">budget</div>
                  <div className="mt-0.5 text-xs font-bold" style={{ color: tone }}>
                    {burn > 1 ? "breached" : `${Math.round((1 - burn) * 100)}% left`}
                  </div>
                </div>
              </div>
              <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-background/60">
                <div
                  className="h-full transition-all"
                  style={{
                    width: `${Math.min(100, pct)}%`,
                    background: tone,
                    boxShadow: `0 0 12px ${tone}`,
                  }}
                />
              </div>
              <div className="mt-1.5 flex items-center justify-between font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                <span>burn</span>
                <span style={{ color: tone }}>{(burn * 100).toFixed(0)}%</span>
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

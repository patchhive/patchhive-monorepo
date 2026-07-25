import { useMemo } from "react";
import { AlertTriangle, GitCompare, Minus, Plus } from "lucide-react";
import { PRODUCTS } from "@/lib/hive-data";
import { DRIFT_SCHEMAS } from "@/lib/hive-extra";

export function ContractDrift({ syncVersion = 0 }: { syncVersion?: number }) {
  const byId = useMemo(() => Object.fromEntries(PRODUCTS.map((p) => [p.id, p])), [syncVersion]);
  const drifted = PRODUCTS.filter((p) => p.contractDrift > 0);
  const schemaById = useMemo(
    () => Object.fromEntries(DRIFT_SCHEMAS.map((s) => [s.productId, s])),
    [],
  );

  return (
    <section id="drift" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-5 flex items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <GitCompare className="h-4 w-4 text-[var(--honey)]" /> Contract Drift Inspector
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Capabilities declared in the manifest vs what each product currently advertises on /capabilities.
          </p>
        </div>
        <span className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          {drifted.length} of {PRODUCTS.length} drifted
        </span>
      </div>

      {drifted.length === 0 && (
        <p className="rounded-lg border border-border bg-background/40 p-4 text-xs text-muted-foreground">
          No drift detected. All advertised capabilities match declared contracts.
        </p>
      )}

      <div className="grid gap-3">
        {drifted.map((p) => {
          const schema = schemaById[p.id];
          if (!schema) return null;
          const expected = new Set(schema.expected);
          const actual = new Set(schema.actual);
          const missing = schema.expected.filter((c) => !actual.has(c));
          const extra = schema.actual.filter((c) => !expected.has(c));
          const matched = schema.expected.filter((c) => actual.has(c));
          return (
            <div key={p.id} className="rounded-lg border border-[var(--warn)]/40 bg-[var(--warn)]/5 p-4">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-display text-xs font-bold">{p.name}</span>
                <span className="rounded border border-[var(--warn)]/40 bg-[var(--warn)]/15 px-1.5 py-0.5 font-display text-[10px] uppercase tracking-wider text-[var(--warn)]">
                  drift {p.contractDrift}
                </span>
                {schema.notes && (
                  <span className="ml-auto inline-flex items-center gap-1 font-display text-[10px] text-muted-foreground">
                    <AlertTriangle className="h-3 w-3 text-[var(--warn)]" /> {schema.notes}
                  </span>
                )}
              </div>
              <div className="mt-3 grid gap-2 md:grid-cols-2">
                <div>
                  <div className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">declared</div>
                  <div className="mt-1 flex flex-wrap gap-1">
                    {schema.expected.map((c) => (
                      <span
                        key={c}
                        className={`rounded border px-1.5 py-0.5 font-display text-[10px] ${
                          actual.has(c)
                            ? "border-border bg-muted/40 text-muted-foreground"
                            : "border-[var(--crit)]/50 bg-[var(--crit)]/10 text-[var(--crit)] line-through"
                        }`}
                      >
                        {c}
                      </span>
                    ))}
                  </div>
                </div>
                <div>
                  <div className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">advertised</div>
                  <div className="mt-1 flex flex-wrap gap-1">
                    {schema.actual.map((c) => (
                      <span
                        key={c}
                        className={`rounded border px-1.5 py-0.5 font-display text-[10px] ${
                          expected.has(c)
                            ? "border-border bg-muted/40 text-muted-foreground"
                            : "border-[var(--honey)]/50 bg-[var(--honey)]/10 text-[var(--honey)]"
                        }`}
                      >
                        {c}
                      </span>
                    ))}
                  </div>
                </div>
              </div>
              <div className="mt-3 flex flex-wrap gap-3 border-t border-border/40 pt-2 font-display text-[10px]">
                <span className="inline-flex items-center gap-1 text-[var(--crit)]">
                  <Minus className="h-3 w-3" /> missing: {missing.length}
                </span>
                <span className="inline-flex items-center gap-1 text-[var(--honey)]">
                  <Plus className="h-3 w-3" /> extra: {extra.length}
                </span>
                <span className="inline-flex items-center gap-1 text-muted-foreground">
                  matched: {matched.length}
                </span>
              </div>
            </div>
          );
        })}
      </div>
      <p className="mt-3 font-display text-[10px] text-muted-foreground">
        {Object.keys(byId).length} products inspected.
      </p>
    </section>
  );
}

import { useMemo } from "react";
import { GitBranch } from "lucide-react";
import { DEPENDENCIES } from "@/lib/hive-extra";
import { PRODUCTS } from "@/lib/hive-data";

/** Downstreams a given product transitively calls. */
function downstreams(productId: string): string[] {
  const seen = new Set<string>();
  const queue = [productId];
  while (queue.length) {
    const cur = queue.shift()!;
    for (const e of DEPENDENCIES) {
      if (e.from === cur && !seen.has(e.to) && e.to !== productId) {
        seen.add(e.to);
        queue.push(e.to);
      }
    }
  }
  return [...seen];
}

export function BlastRadius({ productName }: { productName: string }) {
  const product = PRODUCTS.find((p) => p.name === productName);
  const ids = useMemo(() => (product ? downstreams(product.id) : []), [product]);
  const byId = Object.fromEntries(PRODUCTS.map((p) => [p.id, p]));

  if (!product || ids.length === 0) {
    return (
      <div className="rounded-lg border border-border bg-muted/20 p-3">
        <div className="mb-1 flex items-center gap-2 font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          <GitBranch className="h-3 w-3 text-[var(--honey)]" /> Blast radius
        </div>
        <div className="font-display text-[11px] text-muted-foreground">Isolated — no downstream services.</div>
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border bg-muted/20 p-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          <GitBranch className="h-3 w-3 text-[var(--honey)]" /> Blast radius
        </div>
        <span className="rounded border border-[var(--warn)]/40 bg-[var(--warn)]/10 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-[var(--warn)]">
          {ids.length} downstream
        </span>
      </div>
      <div className="flex flex-wrap gap-1.5">
        {ids.map((id) => {
          const p = byId[id];
          if (!p) return null;
          const tone =
            p.status === "crit" ? "var(--crit)" : p.status === "warn" ? "var(--warn)" : "var(--ok)";
          return (
            <span
              key={id}
              className="inline-flex items-center gap-1 rounded border border-border bg-background/60 px-2 py-0.5 font-display text-[10px]"
            >
              <span className="h-1.5 w-1.5 rounded-full" style={{ background: tone, boxShadow: `0 0 6px ${tone}` }} />
              {p.name}
            </span>
          );
        })}
      </div>
    </div>
  );
}

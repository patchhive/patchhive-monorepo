import { useMemo } from "react";
import { GitBranch, ShieldCheck, ShieldOff } from "lucide-react";
import { DEPENDENCIES, EDGE_LABEL, type SafetyEdge } from "@/lib/hive-extra";
import { PRODUCTS } from "@/lib/hive-data";

/**
 * What stops if this product is unavailable.
 *
 * Two directions matter and they are not the same question:
 *   upstream  — what this product needs before it may act
 *   dependents — what stops when this product goes away
 *
 * The rule these encode: a gated action never proceeds ungated and is never
 * rerouted. If TrustGate is down, work needing review stops.
 */
function upstreamOf(productId: string): SafetyEdge[] {
  return DEPENDENCIES.filter((edge) => edge.from === productId);
}

function dependentsOf(productId: string): SafetyEdge[] {
  return DEPENDENCIES.filter((edge) => edge.to === productId);
}

const kindTone: Record<SafetyEdge["kind"], string> = {
  authority: "var(--crit)",
  gate: "var(--warn)",
  context: "var(--honey)",
  handoff: "var(--ok)",
};

export function BlastRadius({ productName }: { productName: string }) {
  const product = PRODUCTS.find((p) => p.name === productName);
  const byId = useMemo(() => Object.fromEntries(PRODUCTS.map((p) => [p.id, p])), []);
  const upstream = useMemo(() => (product ? upstreamOf(product.id) : []), [product]);
  const dependents = useMemo(() => (product ? dependentsOf(product.id) : []), [product]);

  if (!product || (upstream.length === 0 && dependents.length === 0)) {
    return (
      <div className="rounded-lg border border-border bg-muted/20 p-3">
        <div className="mb-1 flex items-center gap-2 font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          <GitBranch className="h-3 w-3 text-[var(--honey)]" /> Blast radius
        </div>
        <div className="font-display text-[11px] text-muted-foreground">
          Isolated — nothing gates this product and nothing depends on it.
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border bg-muted/20 p-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
          <GitBranch className="h-3 w-3 text-[var(--honey)]" /> Blast radius
        </div>
        {dependents.length > 0 && (
          <span className="rounded border border-[var(--warn)]/40 bg-[var(--warn)]/10 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-[var(--warn)]">
            {dependents.length} would stall
          </span>
        )}
      </div>

      {dependents.length > 0 && (
        <div className="mb-2">
          <div className="mb-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground">
            If {product.name} is unavailable
          </div>
          <ul className="space-y-1">
            {dependents.map((edge) => (
              <EdgeRow key={`${edge.from}->${edge.to}`} edge={edge} name={byId[edge.from]?.name ?? edge.from} />
            ))}
          </ul>
        </div>
      )}

      {upstream.length > 0 && (
        <div>
          <div className="mb-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground">
            {product.name} depends on
          </div>
          <ul className="space-y-1">
            {upstream.map((edge) => (
              <EdgeRow key={`${edge.from}->${edge.to}`} edge={edge} name={byId[edge.to]?.name ?? edge.to} />
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

function EdgeRow({ edge, name }: { edge: SafetyEdge; name: string }) {
  const tone = kindTone[edge.kind];
  return (
    <li className="rounded border border-border/60 bg-background/50 px-2 py-1.5">
      <div className="flex flex-wrap items-center gap-1.5">
        <span
          className="h-1.5 w-1.5 rounded-full"
          style={{ background: tone, boxShadow: `0 0 6px ${tone}` }}
        />
        <span className="font-display text-[10px] font-bold text-foreground">{name}</span>
        <span className="font-display text-[9px] uppercase tracking-wider" style={{ color: tone }}>
          {EDGE_LABEL[edge.kind]}
        </span>
        {edge.live ? (
          <span
            title="Enforced today"
            className="inline-flex items-center gap-0.5 font-display text-[9px] uppercase tracking-wider text-[var(--ok)]"
          >
            <ShieldCheck className="h-2.5 w-2.5" /> live
          </span>
        ) : (
          <span
            title="Documented intent; not wired yet"
            className="inline-flex items-center gap-0.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground"
          >
            <ShieldOff className="h-2.5 w-2.5" /> planned
          </span>
        )}
      </div>
      <div className="mt-0.5 text-[10px] leading-snug text-muted-foreground">{edge.effect}</div>
    </li>
  );
}

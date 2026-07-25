import { useMemo, useState } from "react";
import { GitBranch } from "lucide-react";

import { PRODUCTS, PRODUCTS_BY_KEY } from "@/lib/hive-data";
import { blastRadius } from "@/lib/suite-state";
import { Chip, EmptyDeck, Panel, Section } from "./deck-ui";

/**
 * Not an RPC call graph — PatchHive products do not call each other in a request
 * path. The real dependency is safety-gating: TrustGate gates diffs, RepoMemory
 * supplies context, the kernel authorizes. So the question this answers is "what
 * stalls if this product goes away", which is §3.12 made visible.
 */
export function BlastRadius() {
  const [productKey, setProductKey] = useState(PRODUCTS[0]?.key ?? "");
  const impact = useMemo(() => blastRadius(productKey), [productKey]);
  const product = PRODUCTS_BY_KEY[productKey];

  return (
    <Section
      id="blast-radius"
      title="Blast radius"
      kicker="If this product is unavailable, gated work stops. It never proceeds ungated and it is never rerouted."
      actions={
        <select
          value={productKey}
          onChange={(event) => setProductKey(event.target.value)}
          className="rounded border border-border bg-background/60 px-2 py-1 font-display text-[10px] uppercase tracking-wider text-muted-foreground outline-none"
        >
          {PRODUCTS.map((item) => (
            <option key={item.key} value={item.key}>
              {item.name}
            </option>
          ))}
        </select>
      }
    >
      <div className="grid gap-4 lg:grid-cols-2">
        <Panel>
          <div className="flex items-center gap-2 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
            <GitBranch className="h-3 w-3" /> Work blocked
          </div>
          {impact.blockedWork.length === 0 ? (
            <p className="mt-2 text-xs text-muted-foreground">
              No work items are currently gated on {product?.name ?? productKey}.
            </p>
          ) : (
            <ul className="mt-2 space-y-1">
              {impact.blockedWork.map((item) => (
                <li key={item.id} className="flex items-center gap-2 text-xs">
                  <Chip tone="warn">{item.state}</Chip>
                  <span className="truncate font-mono text-[11px] text-foreground">
                    {item.repository}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Panel>
        <Panel>
          <div className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">
            Mandates stalled
          </div>
          {impact.stalledMandates.length === 0 ? (
            <p className="mt-2 text-xs text-muted-foreground">
              No active mandate declares {product?.name ?? productKey} as a gate.
            </p>
          ) : (
            <ul className="mt-2 space-y-1">
              {impact.stalledMandates.map((mandate) => (
                <li key={mandate.id} className="text-xs text-foreground">
                  {mandate.name}
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>
      {PRODUCTS.length > 0 && impact.blockedWork.length === 0 && impact.stalledMandates.length === 0 && (
        <div className="mt-4">
          <EmptyDeck
            title="Nothing to stall yet"
            detail="Blast radius is computed over live work items and mandates. Both arrive with the conductor."
            source="work_items · mandates (architecture doc §3.8, §3.6)"
          />
        </div>
      )}
    </Section>
  );
}

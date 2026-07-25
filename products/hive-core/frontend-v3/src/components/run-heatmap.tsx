import { useMemo, useState } from "react";
import { Activity } from "lucide-react";

import type { Product } from "@/lib/hive-data";
import type { SuiteEvent } from "@/lib/suite-state";
import { Chip, EmptyDeck, Section } from "./deck-ui";

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const HOURS = Array.from({ length: 24 }, (_, hour) => hour);

type Lens = "dispatch" | "denial";

/**
 * When work actually happens. For an unattended system this answers real questions:
 * did the nightly mandate fire, why is everything clustered at 03:00, and where does
 * GitHub rate-limit pressure concentrate. Denials share the grid so throttling is
 * visible rather than showing up as absence.
 */
function grid(lens: Lens, productKey: string, events: SuiteEvent[]): number[][] {
  const cells = DAYS.map(() => HOURS.map(() => 0));
  for (const event of events) {
    if (productKey !== "all" && event.productKey !== productKey) continue;
    const isDenial = event.kind === "policy_decision" || event.kind === "budget";
    if (lens === "denial" ? !isDenial : event.kind !== "dispatch") continue;
    const at = new Date(event.ts);
    cells[at.getDay()][at.getHours()] += 1;
  }
  return cells;
}

export function RunHeatmap({
  products,
  events,
}: {
  products: Product[];
  events: SuiteEvent[];
}) {
  const [lens, setLens] = useState<Lens>("dispatch");
  const [productKey, setProductKey] = useState("all");
  const cells = useMemo(() => grid(lens, productKey, events), [lens, productKey, events]);
  const max = Math.max(...cells.flat(), 1);
  const total = cells.flat().reduce((sum, value) => sum + value, 0);
  const accent = lens === "denial" ? "var(--warn)" : "var(--honey)";

  return (
    <Section
      id="heatmap"
      title="Run volume"
      kicker="Dispatches and policy denials by weekday and hour."
      actions={
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex items-center gap-1">
            {(["dispatch", "denial"] as const).map((value) => (
              <button
                key={value}
                onClick={() => setLens(value)}
                className={`rounded px-2 py-1 font-display text-[10px] uppercase tracking-wider transition ${
                  lens === value
                    ? "bg-[color-mix(in_oklab,var(--honey)_18%,transparent)] text-[var(--honey)]"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {value === "dispatch" ? "Dispatches" : "Denials"}
              </button>
            ))}
          </div>
          <select
            value={productKey}
            onChange={(event) => setProductKey(event.target.value)}
            className="rounded border border-border bg-background/60 px-2 py-1 font-display text-[10px] uppercase tracking-wider text-muted-foreground outline-none"
          >
            <option value="all">All products</option>
            {products.map((product) => (
              <option key={product.key} value={product.key}>
                {product.name}
              </option>
            ))}
          </select>
          <Chip tone="neutral">{total} total</Chip>
        </div>
      }
    >
      {total === 0 ? (
        <EmptyDeck
          title="No activity to plot"
          detail="Populated from the suite event ledger. Until schedules and mandates are dispatching, there is nothing to cluster."
          source="GET /events"
        />
      ) : (
        <div className="overflow-x-auto">
          <div className="min-w-[640px]">
            <div className="mb-1 flex gap-[3px] pl-9">
              {HOURS.map((hour) => (
                <div
                  key={hour}
                  className="w-3.5 text-center font-mono text-[8px] text-muted-foreground"
                >
                  {hour % 6 === 0 ? hour : ""}
                </div>
              ))}
            </div>
            {cells.map((row, dayIndex) => (
              <div key={DAYS[dayIndex]} className="mb-[3px] flex items-center gap-[3px]">
                <div className="w-9 font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                  {DAYS[dayIndex]}
                </div>
                {row.map((value, hour) => (
                  <div
                    key={hour}
                    title={`${DAYS[dayIndex]} ${hour}:00 — ${value}`}
                    className="h-3.5 w-3.5 rounded-[2px] border border-border/40"
                    style={{
                      background:
                        value === 0
                          ? "transparent"
                          : `color-mix(in oklab, ${accent} ${Math.round((value / max) * 85) + 15}%, transparent)`,
                    }}
                  />
                ))}
              </div>
            ))}
            <div className="mt-3 flex items-center gap-2 text-[10px] text-muted-foreground">
              <Activity className="h-3 w-3" />
              peak {max} in a single hour
            </div>
          </div>
        </div>
      )}
    </Section>
  );
}

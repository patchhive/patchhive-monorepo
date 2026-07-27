import { useMemo, useState } from "react";
import { Activity } from "lucide-react";
import { PRODUCTS } from "@/lib/hive-data";
import { runHeatmap } from "@/lib/hive-metrics";

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

export function RunHeatmap({
  syncVersion = 0,
  lockedProductName,
}: {
  syncVersion?: number;
  /** When set, the heatmap covers only this product and the selector is hidden. */
  lockedProductName?: string;
}) {
  const [productId, setProductId] = useState<string>("all");
  // Bucketed from the real run feed. RUNS is mutated in place by the live sync, so
  // syncVersion is the dependency that matters — the array identity never changes.
  const heatmap = useMemo(() => {
    if (lockedProductName) return runHeatmap(lockedProductName);
    const name = PRODUCTS.find((product) => product.id === productId)?.name;
    return runHeatmap(productId === "all" ? undefined : name);
  }, [productId, syncVersion, lockedProductName]);
  const grid = heatmap.grid;
  const max = Math.max(...grid.flat(), 1);
  const total = heatmap.counted;
  const [hover, setHover] = useState<{ d: number; h: number; v: number } | null>(null);

  return (
    <section id="heatmap" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <Activity className="h-4 w-4 text-[var(--honey)]" /> Run Volume Heatmap
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Runs the suite has reported, bucketed by weekday and hour. The feed carries
            each product's recent runs rather than full history, so sparse is expected —
            an empty cell means nothing was reported in that hour, not that nothing ran.
          </p>
        </div>
        <div className="flex items-center gap-2 font-display text-[10px] uppercase tracking-wider">
          {!lockedProductName && (
          <select
            value={productId}
            onChange={(e) => setProductId(e.target.value)}
            className="rounded border border-border bg-background/60 px-2 py-1 text-foreground focus:border-[var(--honey)]/60 focus:outline-none"
          >
            <option value="all">All products</option>
            {PRODUCTS.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
          )}
          <span className="text-muted-foreground">
            {total.toLocaleString()} placed
            {heatmap.undated > 0 && ` · ${heatmap.undated} undated`}
          </span>
        </div>
      </div>

      <div className="overflow-x-auto">
        <div className="inline-block">
          <div className="ml-10 flex gap-[2px]">
            {Array.from({ length: 24 }, (_, h) => (
              <div key={h} className="w-3 text-center font-display text-[8px] text-muted-foreground/70">
                {h % 3 === 0 ? h : ""}
              </div>
            ))}
          </div>
          {grid.map((row, d) => (
            <div key={d} className="mt-[2px] flex items-center gap-1">
              <div className="w-8 pr-2 text-right font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                {DAYS[d]}
              </div>
              <div className="flex gap-[2px]">
                {row.map((v, h) => {
                  const intensity = v / max;
                  return (
                    <div
                      key={h}
                      onMouseEnter={() => setHover({ d, h, v })}
                      onMouseLeave={() => setHover(null)}
                      className="h-3 w-3 rounded-[2px] border border-border/40 transition"
                      style={{
                        background: `color-mix(in oklab, var(--honey) ${Math.round(intensity * 90)}%, var(--card))`,
                      }}
                      title={`${DAYS[d]} ${h}:00 · ${v} runs`}
                    />
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="mt-3 flex items-center justify-between font-display text-[10px] uppercase tracking-wider text-muted-foreground">
        <div className="flex items-center gap-2">
          <span>less</span>
          {[0.05, 0.25, 0.5, 0.75, 1].map((v) => (
            <span
              key={v}
              className="h-2 w-3 rounded-[2px] border border-border/40"
              style={{ background: `color-mix(in oklab, var(--honey) ${Math.round(v * 90)}%, var(--card))` }}
            />
          ))}
          <span>more</span>
        </div>
        <div>
          {hover ? `${DAYS[hover.d]} ${hover.h}:00 · ${hover.v} runs` : "hover a cell for detail"}
        </div>
      </div>
    </section>
  );
}

import { useEffect, useMemo, useRef, useState } from "react";
import { Pause, Play, Radio } from "lucide-react";
import { PRODUCTS, RUNS, type RunEvent } from "@/lib/hive-data";

const STATUS_POOL: RunEvent["status"][] = ["success", "success", "success", "success", "running", "failed"];

function randomTick(seed: number): RunEvent {
  const p = PRODUCTS[Math.floor(((seed * 9301 + 49297) % 233280) / 233280 * PRODUCTS.length)];
  const cap = p.capabilities[Math.floor(((seed * 1103515245 + 12345) % 2147483647) / 2147483647 * p.capabilities.length)];
  const status = STATUS_POOL[seed % STATUS_POOL.length];
  return {
    id: `r_${(0x8b00 + seed).toString(16)}`,
    product: p.name,
    capability: cap,
    status,
    durationMs: status === "running" ? 0 : 40 + (seed * 17) % 4200,
    ts: "just now",
  };
}

export function LiveTail() {
  const [paused, setPaused] = useState(false);
  const seedRef = useRef(1);
  const [items, setItems] = useState<RunEvent[]>(() => RUNS.slice(0, 14));

  useEffect(() => {
    if (paused) return;
    const i = window.setInterval(() => {
      seedRef.current += 1;
      setItems((cur) => [randomTick(seedRef.current), ...cur].slice(0, 30));
    }, 1800);
    return () => window.clearInterval(i);
  }, [paused]);

  const visible = useMemo(() => items.slice(0, 24), [items]);

  return (
    <div className="fixed inset-x-0 bottom-0 z-30 border-t border-border/60 bg-background/85 backdrop-blur supports-[backdrop-filter]:bg-background/65">
      <div className="mx-auto flex max-w-[1400px] items-center gap-3 px-4 py-1.5">
        <div className="flex shrink-0 items-center gap-1.5 font-display text-[10px] uppercase tracking-[0.25em] text-[var(--honey)]">
          <Radio className={`h-3 w-3 ${paused ? "" : "animate-pulse-dot"}`} />
          {paused ? "tail paused" : "live tail"}
        </div>
        <button
          onClick={() => setPaused((p) => !p)}
          className="shrink-0 rounded border border-border bg-card/60 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:text-[var(--honey)]"
          aria-label={paused ? "Resume live tail" : "Pause live tail"}
        >
          {paused ? <Play className="inline h-2.5 w-2.5" /> : <Pause className="inline h-2.5 w-2.5" />}
        </button>
        <div className="relative flex-1 overflow-hidden">
          <div className="flex gap-4 whitespace-nowrap font-display text-[10px]">
            {visible.map((r, i) => {
              const tone =
                r.status === "success"
                  ? "text-[var(--ok)]"
                  : r.status === "failed"
                  ? "text-[var(--crit)]"
                  : "text-[var(--honey)]";
              return (
                <span key={`${r.id}-${i}`} className="inline-flex items-center gap-1.5 text-muted-foreground">
                  <span className={`h-1 w-1 rounded-full ${
                    r.status === "success" ? "bg-[var(--ok)]" : r.status === "failed" ? "bg-[var(--crit)]" : "bg-[var(--honey)] animate-pulse-dot"
                  }`} />
                  <span className="text-foreground/80">{r.product}</span>
                  <code className={tone}>{r.capability}</code>
                  <span className="text-muted-foreground/70">
                    {r.status === "running" ? "…" : `${r.durationMs}ms`}
                  </span>
                  <span className="text-muted-foreground/40">·</span>
                </span>
              );
            })}
          </div>
          <div className="pointer-events-none absolute inset-y-0 right-0 w-12 bg-gradient-to-l from-background to-transparent" />
        </div>
      </div>
    </div>
  );
}

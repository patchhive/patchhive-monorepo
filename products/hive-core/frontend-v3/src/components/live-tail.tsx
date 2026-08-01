import { useEffect, useMemo, useRef, useState } from "react";
import { Pause, Play, Radio } from "lucide-react";
import { RUNS, type RunEvent } from "@/lib/hive-data";

/**
 * The ticker streams the real run index rather than inventing events.
 *
 * It previously synthesised a fake run every 1.8s, which read as constant activity
 * whether or not the suite was doing anything — the single most misleading thing on
 * the deck once the rest became live. Now it cycles the newest real runs, so an idle
 * suite looks idle.
 */
export function LiveTail() {
  const [paused, setPaused] = useState(false);
  const offsetRef = useRef(0);
  const [items, setItems] = useState<RunEvent[]>(() => RUNS.slice(0, 24));

  useEffect(() => {
    if (paused) return;
    const i = window.setInterval(() => {
      // Rotate the window over real runs so the marquee keeps moving without
      // implying new work has happened.
      offsetRef.current = RUNS.length > 0 ? (offsetRef.current + 1) % RUNS.length : 0;
      const start = offsetRef.current;
      const window_ = RUNS.length
        ? Array.from({ length: Math.min(24, RUNS.length) }, (_, index) => RUNS[(start + index) % RUNS.length])
        : [];
      setItems(window_);
    }, 2400);
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
                r.status === "completed"
                  ? "text-[var(--ok)]"
                  : r.status === "failed" || r.status === "cancelled"
                  ? "text-[var(--crit)]"
                  : r.status === "running" || r.status === "queued"
                    ? "text-[var(--honey)]"
                    : "text-muted-foreground";
              return (
                <span key={`${r.id}-${i}`} className="inline-flex items-center gap-1.5 text-muted-foreground">
                  <span className={`h-1 w-1 rounded-full ${
                    r.status === "completed"
                      ? "bg-[var(--ok)]"
                      : r.status === "failed" || r.status === "cancelled"
                        ? "bg-[var(--crit)]"
                        : r.status === "running" || r.status === "queued"
                          ? "bg-[var(--honey)] animate-pulse-dot"
                          : "bg-muted-foreground"
                  }`} />
                  <span className="text-foreground/80">{r.product}</span>
                  <code className={tone}>{r.capability}</code>
                  <span className="text-muted-foreground/70">
                    {r.durationMs === null ? "—" : `${r.durationMs}ms`}
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

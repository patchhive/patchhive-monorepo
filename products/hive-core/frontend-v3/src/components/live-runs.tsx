import { useMemo, useState } from "react";
import { Terminal } from "lucide-react";

import { PRODUCTS_BY_KEY, type RunSummary } from "@/lib/hive-data";
import { Chip, EmptyDeck, Section, type ChipTone } from "./deck-ui";

const statusTone: Record<RunSummary["status"], ChipTone> = {
  queued: "neutral",
  running: "honey",
  completed: "ok",
  failed: "crit",
  cancelled: "neutral",
};

export function LiveRuns({
  runs,
  onSelect,
}: {
  runs: RunSummary[];
  onSelect?: (run: RunSummary) => void;
}) {
  const [only, setOnly] = useState<"all" | RunSummary["status"]>("all");
  const rows = useMemo(
    () => (only === "all" ? runs : runs.filter((run) => run.status === only)),
    [only, runs],
  );

  return (
    <Section
      id="runs"
      title="Live runs"
      kicker="Every product run across the suite, newest first. One timeline, not twelve."
      actions={
        <div className="flex items-center gap-1">
          {(["all", "running", "failed", "completed"] as const).map((value) => (
            <button
              key={value}
              onClick={() => setOnly(value)}
              className={`rounded px-2 py-1 font-display text-[10px] uppercase tracking-wider transition ${
                only === value
                  ? "bg-[color-mix(in_oklab,var(--honey)_18%,transparent)] text-[var(--honey)]"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              {value}
            </button>
          ))}
        </div>
      }
    >
      {rows.length === 0 ? (
        <EmptyDeck
          title="No suite run history yet"
          detail="HiveCore does not persist product runs today — it proxies them live per request. This fills in once the materialized run index exists (architecture doc, blocker B2)."
          source="GET /products/:slug/runs"
        />
      ) : (
        <ul className="divide-y divide-border/40">
          {rows.map((run) => {
            const product = PRODUCTS_BY_KEY[run.productKey];
            return (
              <li key={run.id}>
                <button
                  onClick={() => onSelect?.(run)}
                  className="flex w-full items-center gap-3 py-2.5 text-left transition hover:bg-background/40"
                >
                  <Terminal className="h-3.5 w-3.5 flex-shrink-0 text-muted-foreground" />
                  <span className="font-mono text-[11px] text-muted-foreground">{run.id}</span>
                  <span className="font-display text-xs font-bold text-foreground">
                    {product?.name ?? run.productKey}
                  </span>
                  <span className="flex-1 truncate text-xs text-muted-foreground">
                    {run.title}
                  </span>
                  <Chip tone="neutral">{run.triggerMode}</Chip>
                  <Chip tone="neutral">{run.targetSelectionMode}</Chip>
                  <Chip tone={statusTone[run.status]}>{run.status}</Chip>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </Section>
  );
}

import { useEffect, useState } from "react";
import { AlertTriangle, BookOpen, CheckCircle2, XCircle } from "lucide-react";
import { fetchRunbookRuns, RUN_TONE, type RunbookRun } from "@/lib/runbooks";
import { idForSlug } from "@/lib/product-slugs";
import { useHiveCommand } from "./hive-command";
import { EmptyHex } from "./empty-hex";

/**
 * Past runbook executions, read from the control plane.
 *
 * This used to render React state, so the history vanished on reload — and what it
 * recorded was fabricated anyway: dry-run versus "live" execution, where live meant a
 * timer. Both dimensions are gone. Every entry is a real diagnostic pass with a real
 * outcome, and it survives a refresh because the record is the point.
 */
export function RunbookHistory({ syncVersion = 0 }: { syncVersion?: number }) {
  const { openRunbook } = useHiveCommand();
  const [runs, setRuns] = useState<RunbookRun[]>([]);

  useEffect(() => {
    const controller = new AbortController();
    fetchRunbookRuns(controller.signal)
      .then(setRuns)
      .catch(() => undefined);
    return () => controller.abort();
  }, [syncVersion]);

  return (
    <section
      id="runbook-history"
      className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur"
    >
      <div className="mb-4 flex items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <BookOpen className="h-4 w-4 text-[var(--honey)]" /> Runbook History
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Recorded diagnostic passes over each product. Read-only checks: reachability,
            startup, contract conformance, service-token posture, recent run outcomes.
          </p>
        </div>
        <div className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          {runs.length} run{runs.length === 1 ? "" : "s"}
        </div>
      </div>

      {runs.length === 0 ? (
        <EmptyHex
          title="No runbooks run yet"
          hint="Open a runbook from an incident or product card"
        />
      ) : (
        <ul className="divide-y divide-border/40 rounded-lg border border-border bg-background/30">
          {runs.map((run) => {
            const Icon =
              run.status === "ok"
                ? CheckCircle2
                : run.status === "failed"
                  ? XCircle
                  : AlertTriangle;
            const tone = RUN_TONE[run.status] ?? "text-muted-foreground border-border";
            return (
              <li
                key={run.id}
                className="flex cursor-pointer items-center gap-3 px-4 py-2.5 transition hover:bg-muted/20"
                onClick={() => openRunbook(idForSlug(run.product_slug))}
              >
                <Icon className={`h-4 w-4 shrink-0 ${tone.split(" ")[0]}`} />
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline justify-between gap-2">
                    <span className="font-display text-xs font-semibold">
                      {run.product_title}
                    </span>
                    <span className="font-display text-[10px] text-muted-foreground">
                      {new Date(run.started_at).toLocaleString()}
                    </span>
                  </div>
                  <div className="mt-0.5 truncate font-display text-[10px] text-muted-foreground">
                    {run.summary}
                  </div>
                </div>
                <span
                  className={`shrink-0 rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${tone}`}
                >
                  {run.status}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

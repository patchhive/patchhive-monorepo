import { BookOpen, CheckCircle2, ShieldAlert } from "lucide-react";
import { useHiveCommand } from "./hive-command";
import { EmptyHex } from "./empty-hex";

export function RunbookHistory() {
  const { runbookHistory, openRunbook } = useHiveCommand();

  return (
    <section id="runbook-history" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-4 flex items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <BookOpen className="h-4 w-4 text-[var(--honey)]" /> Runbook History
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Past runbook executions across the mesh. Dry-runs and live runs tracked separately.
          </p>
        </div>
        <div className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          {runbookHistory.length} execution{runbookHistory.length === 1 ? "" : "s"}
        </div>
      </div>

      {runbookHistory.length === 0 ? (
        <EmptyHex title="No runbooks executed yet" hint="Open a runbook from an incident or product card" />
      ) : (
        <ul className="divide-y divide-border/40 rounded-lg border border-border bg-background/30">
          {runbookHistory.map((h) => {
            const mode = h.dryRun ? "dry-run" : "live";
            const modeCls = h.dryRun
              ? "border-[var(--ok)]/40 bg-[var(--ok)]/10 text-[var(--ok)]"
              : "border-[var(--crit)]/40 bg-[var(--crit)]/10 text-[var(--crit)]";
            return (
              <li
                key={h.id}
                className="flex cursor-pointer items-center gap-3 px-4 py-2.5 transition hover:bg-muted/20"
                onClick={() => openRunbook(h.productId)}
              >
                {h.dryRun ? (
                  <CheckCircle2 className="h-4 w-4 shrink-0 text-[var(--ok)]" />
                ) : (
                  <ShieldAlert className="h-4 w-4 shrink-0 text-[var(--crit)]" />
                )}
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline justify-between gap-2">
                    <span className="font-display text-xs font-semibold">{h.productName}</span>
                    <span className="font-display text-[10px] text-muted-foreground">
                      {new Date(h.at).toLocaleTimeString(undefined, { hour12: false })}
                    </span>
                  </div>
                  <div className="mt-0.5 flex items-center gap-2 font-display text-[10px] text-muted-foreground">
                    <span>{h.steps} steps</span>
                    <span>·</span>
                    <span>{h.actor}</span>
                  </div>
                </div>
                <span className={`shrink-0 rounded border ${modeCls} px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider`}>
                  {mode}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

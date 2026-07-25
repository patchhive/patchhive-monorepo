import { Clock, ScrollText, Trash2 } from "lucide-react";
import { useHiveCommand, type AuditKind } from "./hive-command";

const toneCls: Record<AuditKind, string> = {
  info: "border-border text-muted-foreground",
  action: "border-[var(--honey)]/40 text-[var(--honey)]",
  destructive: "border-[var(--crit)]/40 text-[var(--crit)]",
  ai: "border-[var(--ok)]/40 text-[var(--ok)]",
};

export function AuditLog() {
  const { auditLog, clearAudit } = useHiveCommand();
  return (
    <section id="audit" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-4 flex items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <ScrollText className="h-4 w-4 text-[var(--honey)]" /> Change Log
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Every operator action from the palette, drawer, or hotkeys — actor, target, and diff.
          </p>
        </div>
        <button
          onClick={clearAudit}
          disabled={auditLog.length === 0}
          className="inline-flex items-center gap-1 rounded border border-border bg-card/60 px-2 py-1 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)] disabled:opacity-40"
        >
          <Trash2 className="h-3 w-3" /> clear
        </button>
      </div>

      {auditLog.length === 0 ? (
        <div className="rounded-lg border border-dashed border-border p-8 text-center font-display text-[11px] uppercase tracking-wider text-muted-foreground">
          No actions yet. Fire something from the ⌘K palette.
        </div>
      ) : (
        <ol className="space-y-2">
          {auditLog.map((e) => (
            <li key={e.id} className="rounded-lg border border-border bg-background/40 p-3">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <span className={`rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${toneCls[e.kind]}`}>
                    {e.kind}
                  </span>
                  <span className="font-display text-xs font-bold">{e.title}</span>
                </div>
                <span className="flex items-center gap-1 font-display text-[10px] text-muted-foreground">
                  <Clock className="h-3 w-3" /> {e.actor} · {timeAgo(e.at)}
                </span>
              </div>
              {e.detail && <div className="mt-1 font-display text-[11px] text-muted-foreground">{e.detail}</div>}
              {e.diff && (
                <div className="mt-2 flex items-center gap-2 rounded border border-border/60 bg-muted/20 p-1.5 font-display text-[10px]">
                  <span className="text-[var(--crit)]/80">− {e.diff.before}</span>
                  <span className="text-muted-foreground">→</span>
                  <span className="text-[var(--ok)]">+ {e.diff.after}</span>
                </div>
              )}
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

function timeAgo(ts: number) {
  const s = Math.max(0, Math.round((Date.now() - ts) / 1000));
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  return `${Math.floor(s / 3600)}h ago`;
}

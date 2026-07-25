import { useEffect, useMemo, useState } from "react";
import { AlertOctagon, AlertTriangle, BookOpen, CheckCircle2, Timer } from "lucide-react";
import { PRODUCTS } from "@/lib/hive-data";
import { INCIDENTS, mttrMinutes } from "@/lib/hive-extra";
import { useHiveCommand } from "./hive-command";
import { IncidentSummary } from "./incident-summary";

function fmtAgo(iso: string, now: number) {
  const diff = Math.max(0, now - Date.parse(iso));
  const mins = Math.floor(diff / 60_000);
  if (mins < 60) return `${mins}m ago`;
  const h = Math.floor(mins / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}
function fmtDur(fromIso: string, toIso: string | null, now: number) {
  const end = toIso ? Date.parse(toIso) : now;
  const mins = Math.max(1, Math.round((end - Date.parse(fromIso)) / 60_000));
  if (mins < 60) return `${mins}m`;
  const h = Math.floor(mins / 60);
  return `${h}h ${mins % 60}m`;
}

export function IncidentTimeline() {
  const { openRunbook } = useHiveCommand();
  const [now, setNow] = useState<number | null>(null);
  useEffect(() => {
    setNow(Date.now());
    const i = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(i);
  }, []);

  const byProduct = useMemo(() => Object.fromEntries(PRODUCTS.map((p) => [p.id, p])), []);
  const sorted = useMemo(
    () => [...INCIDENTS].sort((a, b) => Date.parse(b.from) - Date.parse(a.from)),
    [],
  );

  const ongoing = sorted.filter((i) => !i.to).length;
  const totalMttrParts = PRODUCTS.map((p) => mttrMinutes(p.id)).filter((v): v is number => v !== null);
  const fleetMttr = totalMttrParts.length
    ? Math.round(totalMttrParts.reduce((a, b) => a + b, 0) / totalMttrParts.length)
    : null;

  return (
    <section id="incidents" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-5 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="font-display text-sm font-bold uppercase tracking-[0.2em]">Incident Timeline</h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Every ok → warn / crit transition the mesh has observed. MTTR is fleet average across closed incidents.
          </p>
        </div>
        <div className="flex items-center gap-4 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          <span className="flex items-center gap-1.5"><AlertOctagon className="h-3 w-3 text-[var(--crit)]" /> {ongoing} ongoing</span>
          <span className="flex items-center gap-1.5"><Timer className="h-3 w-3 text-[var(--honey)]" /> fleet MTTR {fleetMttr ?? "—"}m</span>
          <span className="flex items-center gap-1.5"><CheckCircle2 className="h-3 w-3 text-[var(--ok)]" /> {sorted.filter((i) => i.to).length} resolved</span>
        </div>
      </div>

      <ol className="relative space-y-3 border-l border-border/50 pl-5">
        {sorted.map((i) => {
          const p = byProduct[i.productId];
          const ongoingThis = !i.to;
          const Icon = i.severity === "crit" ? AlertOctagon : AlertTriangle;
          const tone =
            i.severity === "crit" ? "text-[var(--crit)] border-[var(--crit)]/40 bg-[var(--crit)]/10" : "text-[var(--warn)] border-[var(--warn)]/40 bg-[var(--warn)]/10";
          return (
            <li key={i.id} className="relative">
              <span
                className={`absolute -left-[26px] top-1.5 grid h-3.5 w-3.5 place-items-center rounded-full ${
                  ongoingThis
                    ? "bg-[var(--crit)] shadow-[0_0_10px_var(--crit)] animate-pulse-dot"
                    : i.severity === "crit"
                    ? "bg-[var(--crit)]/60"
                    : "bg-[var(--warn)]/60"
                }`}
              />
              <div className="rounded-lg border border-border bg-background/40 p-3">
                <div className="flex flex-wrap items-center gap-2">
                  <span className={`inline-flex items-center gap-1 rounded border px-1.5 py-0.5 font-display text-[10px] uppercase tracking-wider ${tone}`}>
                    <Icon className="h-3 w-3" />
                    {ongoingThis ? "open" : "resolved"} · {i.severity}
                  </span>
                  <span className="font-display text-xs font-bold">{p?.name ?? i.productId}</span>
                  <button
                    onClick={() => openRunbook(i.productId)}
                    className="inline-flex items-center gap-1 rounded border border-[var(--honey)]/40 bg-[var(--honey)]/10 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-[var(--honey)] transition hover:brightness-110"
                    title="Open runbook"
                  >
                    <BookOpen className="h-2.5 w-2.5" /> runbook
                  </button>
                  <span
                    className="ml-auto font-display text-[10px] text-muted-foreground"
                    suppressHydrationWarning
                  >
                    {now ? `${fmtAgo(i.from, now)} · ${fmtDur(i.from, i.to, now)}` : "—"}
                  </span>
                </div>
                <p className="mt-1.5 text-xs text-foreground/90">{i.summary}</p>
                {i.resolution && (
                  <p className="mt-1 font-display text-[10px] text-[var(--ok)]/85">
                    ↳ {i.resolution}
                  </p>
                )}
                <IncidentSummary incident={i} product={p} />
              </div>
            </li>
          );
        })}
      </ol>
    </section>
  );
}

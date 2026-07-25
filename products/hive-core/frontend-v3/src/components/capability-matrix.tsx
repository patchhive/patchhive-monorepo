import { useMemo, useState } from "react";
import { Cpu, Flame, Timer, AlertTriangle } from "lucide-react";
import { capabilityMatrix, type CapabilityAgg } from "@/lib/hive-metrics";

type SortKey = "calls" | "avgMs" | "p95Ms" | "failRate";

export function CapabilityMatrix() {
  const rows = useMemo(() => capabilityMatrix(), []);
  const [sort, setSort] = useState<SortKey>("calls");

  const sorted = useMemo(
    () => [...rows].sort((a, b) => (b[sort] as number) - (a[sort] as number)),
    [rows, sort],
  );
  const hottest = [...rows].sort((a, b) => b.calls - a.calls)[0];
  const slowest = [...rows].sort((a, b) => b.p95Ms - a.p95Ms)[0];
  const flakiest = [...rows].sort((a, b) => b.failRate - a.failRate)[0];

  return (
    <section id="capabilities" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <Cpu className="h-4 w-4 text-[var(--honey)]" /> Capability Call Matrix
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Aggregated across every product advertising the capability. Sort to find hotspots.
          </p>
        </div>
      </div>

      <div className="mb-4 grid gap-2 sm:grid-cols-3">
        <Highlight icon={Flame} label="hottest" row={hottest} tone="var(--honey)" metric={`${hottest.calls} calls`} />
        <Highlight icon={Timer} label="slowest" row={slowest} tone="var(--warn)" metric={`${slowest.p95Ms}ms p95`} />
        <Highlight
          icon={AlertTriangle}
          label="flakiest"
          row={flakiest}
          tone="var(--crit)"
          metric={`${(flakiest.failRate * 100).toFixed(1)}% fail`}
        />
      </div>

      <div className="overflow-x-auto rounded-lg border border-border bg-background/30">
        <table className="w-full min-w-[640px] border-collapse text-left">
          <thead>
            <tr className="border-b border-border/60 bg-card/40 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
              <th className="px-3 py-2">Capability</th>
              <th className="px-3 py-2">Product</th>
              <SortTh k="calls" cur={sort} set={setSort}>Calls</SortTh>
              <SortTh k="avgMs" cur={sort} set={setSort}>Avg ms</SortTh>
              <SortTh k="p95Ms" cur={sort} set={setSort}>p95 ms</SortTh>
              <SortTh k="failRate" cur={sort} set={setSort}>Fail %</SortTh>
            </tr>
          </thead>
          <tbody>
            {sorted.map((r) => {
              const tone =
                r.failRate > 0.15 ? "var(--crit)" : r.failRate > 0.03 ? "var(--warn)" : "var(--ok)";
              return (
                <tr key={r.product + r.capability} className="border-b border-border/30 transition hover:bg-muted/20">
                  <td className="px-3 py-2 font-display text-[11px] text-[var(--honey)]">{r.capability}</td>
                  <td className="px-3 py-2 font-display text-[11px]">{r.product}</td>
                  <td className="px-3 py-2 font-display text-[11px]">{r.calls.toLocaleString()}</td>
                  <td className="px-3 py-2 font-display text-[11px]">{r.avgMs}</td>
                  <td className="px-3 py-2 font-display text-[11px]">{r.p95Ms}</td>
                  <td className="px-3 py-2 font-display text-[11px]" style={{ color: tone }}>
                    {(r.failRate * 100).toFixed(1)}%
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function SortTh({ k, cur, set, children }: { k: SortKey; cur: SortKey; set: (k: SortKey) => void; children: React.ReactNode }) {
  return (
    <th
      onClick={() => set(k)}
      className={`cursor-pointer px-3 py-2 select-none ${cur === k ? "text-[var(--honey)]" : ""}`}
    >
      {children} {cur === k ? "▼" : ""}
    </th>
  );
}

function Highlight({
  icon: Icon,
  label,
  row,
  tone,
  metric,
}: {
  icon: typeof Flame;
  label: string;
  row: CapabilityAgg;
  tone: string;
  metric: string;
}) {
  return (
    <div className="rounded-lg border border-border bg-background/40 p-3">
      <div className="flex items-center gap-1.5 font-display text-[9px] uppercase tracking-[0.2em] text-muted-foreground">
        <Icon className="h-3 w-3" style={{ color: tone }} /> {label}
      </div>
      <div className="mt-1 truncate font-display text-sm font-bold text-[var(--honey)]">{row.capability}</div>
      <div className="mt-0.5 flex items-center justify-between font-display text-[10px] text-muted-foreground">
        <span>{row.product}</span>
        <span style={{ color: tone }}>{metric}</span>
      </div>
    </div>
  );
}

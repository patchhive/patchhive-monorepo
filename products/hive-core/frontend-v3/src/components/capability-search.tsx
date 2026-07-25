import { useMemo, useState } from "react";
import { Search } from "lucide-react";
import { PRODUCTS } from "@/lib/hive-data";

interface CapEntry {
  capability: string;
  providers: { id: string; name: string }[];
}

export function CapabilitySearch() {
  const [q, setQ] = useState("");
  const allCaps = useMemo<CapEntry[]>(() => {
    const map = new Map<string, { id: string; name: string }[]>();
    PRODUCTS.forEach((p) => {
      p.capabilities.forEach((c) => {
        const arr = map.get(c) ?? [];
        arr.push({ id: p.id, name: p.name });
        map.set(c, arr);
      });
    });
    return [...map.entries()]
      .map(([capability, providers]) => ({ capability, providers }))
      .sort((a, b) => a.capability.localeCompare(b.capability));
  }, []);

  const query = q.trim().toLowerCase();
  const visible = query
    ? allCaps.filter(
        (e) =>
          e.capability.toLowerCase().includes(query) ||
          e.providers.some((p) => p.name.toLowerCase().includes(query)),
      )
    : allCaps;

  return (
    <section id="cap-search" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <Search className="h-4 w-4 text-[var(--honey)]" /> Capability Search
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Global search across every product's advertised capabilities. Try “token”, “scan”, or a product name.
          </p>
        </div>
        <span className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          {visible.length}/{allCaps.length} caps
        </span>
      </div>
      <div className="relative mb-4">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="e.g. rotate, scan.repo, MergeKeeper…"
          className="w-full rounded-lg border border-border bg-background/60 py-2 pl-9 pr-3 font-display text-xs text-foreground placeholder:text-muted-foreground/60 focus:border-[var(--honey)]/60 focus:outline-none"
          aria-label="Search capabilities"
        />
      </div>
      <div className="max-h-80 overflow-y-auto rounded-lg border border-border bg-background/30">
        {visible.length === 0 ? (
          <div className="p-6 text-center font-display text-[10px] uppercase tracking-wider text-muted-foreground">
            no capabilities match “{q}”
          </div>
        ) : (
          <ul className="divide-y divide-border/40">
            {visible.map((e) => (
              <li key={e.capability} className="flex items-center justify-between gap-3 px-4 py-2">
                <code className="truncate font-display text-xs text-[var(--honey)]">{e.capability}</code>
                <div className="flex flex-wrap items-center justify-end gap-1">
                  {e.providers.map((p) => (
                    <span
                      key={p.id}
                      className="rounded border border-border bg-card/60 px-1.5 py-0.5 font-display text-[10px] text-foreground/80"
                    >
                      {p.name}
                    </span>
                  ))}
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

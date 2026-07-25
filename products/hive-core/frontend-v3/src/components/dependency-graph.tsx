import { useMemo, useState } from "react";
import { GitBranch } from "lucide-react";
import { PRODUCTS, type Status } from "@/lib/hive-data";
import { DEPENDENCIES } from "@/lib/hive-extra";

const PRODUCT_IDS_ORDER = [
  ["releasesentry", "mergekeeper", "reviewbee"],
  ["refactorscout", "repomemory", "vulntriage"],
  ["hivecore", "trustgate", "deptriage"],
  ["reporeaper", "signalhive", "flakesting"],
];

function statusFill(s: Status) {
  return s === "ok"
    ? "var(--ok)"
    : s === "warn"
    ? "var(--warn)"
    : s === "crit"
    ? "var(--crit)"
    : "var(--muted-foreground)";
}

export function DependencyGraph() {
  const byId = useMemo(() => Object.fromEntries(PRODUCTS.map((p) => [p.id, p])), []);
  const [hover, setHover] = useState<string | null>(null);

  const W = 760;
  const H = 460;
  const cols = 3;
  const rows = PRODUCT_IDS_ORDER.length;
  const cellW = W / cols;
  const cellH = H / rows;

  const positions = useMemo(() => {
    const map: Record<string, { x: number; y: number }> = {};
    PRODUCT_IDS_ORDER.forEach((row, r) => {
      row.forEach((id, c) => {
        if (!byId[id]) return;
        map[id] = { x: cellW * c + cellW / 2, y: cellH * r + cellH / 2 };
      });
    });
    // Any product not placed (safety): drop in bottom row
    PRODUCTS.forEach((p, i) => {
      if (!map[p.id]) map[p.id] = { x: 60 + (i % cols) * cellW, y: H - 30 };
    });
    return map;
  }, [byId, cellW, cellH]);

  const visibleEdges = DEPENDENCIES.filter((e) => positions[e.from] && positions[e.to]);

  const relatedFor = (id: string | null) => {
    if (!id) return new Set<string>();
    const s = new Set<string>([id]);
    DEPENDENCIES.forEach((e) => {
      if (e.from === id) s.add(e.to);
      if (e.to === id) s.add(e.from);
    });
    return s;
  };
  const related = relatedFor(hover);

  return (
    <section id="dependencies" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-5 flex items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <GitBranch className="h-4 w-4 text-[var(--honey)]" /> Dependency Graph
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Caller → callee edges. Hover any node to isolate its neighbourhood.
          </p>
        </div>
        <span className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          {visibleEdges.length} edges · {Object.keys(positions).length} nodes
        </span>
      </div>

      <div className="overflow-x-auto">
        <svg viewBox={`0 0 ${W} ${H}`} className="h-auto w-full min-w-[640px]" role="img" aria-label="Dependency graph">
          <defs>
            <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
              <path d="M0,0 L10,5 L0,10 z" fill="color-mix(in oklab, var(--honey) 70%, transparent)" />
            </marker>
            <marker id="arrow-dim" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
              <path d="M0,0 L10,5 L0,10 z" fill="color-mix(in oklab, var(--honey) 20%, transparent)" />
            </marker>
          </defs>

          {visibleEdges.map((e, i) => {
            const a = positions[e.from];
            const b = positions[e.to];
            const isLit = !hover || (related.has(e.from) && related.has(e.to));
            const midX = (a.x + b.x) / 2;
            const curve = (b.y - a.y) * 0.15;
            const d = `M ${a.x} ${a.y} Q ${midX} ${(a.y + b.y) / 2 + curve} ${b.x} ${b.y}`;
            return (
              <path
                key={i}
                d={d}
                fill="none"
                stroke={isLit ? "color-mix(in oklab, var(--honey) 55%, transparent)" : "color-mix(in oklab, var(--honey) 12%, transparent)"}
                strokeWidth={isLit ? 1.4 : 1}
                markerEnd={isLit ? "url(#arrow)" : "url(#arrow-dim)"}
              />
            );
          })}

          {Object.entries(positions).map(([id, pos]) => {
            const p = byId[id];
            if (!p) return null;
            const dim = hover && !related.has(id);
            const fill = statusFill(p.status);
            const r = id === hover ? 30 : 26;
            return (
              <g
                key={id}
                transform={`translate(${pos.x}, ${pos.y})`}
                onMouseEnter={() => setHover(id)}
                onMouseLeave={() => setHover(null)}
                style={{ cursor: "pointer", opacity: dim ? 0.35 : 1, transition: "opacity 150ms" }}
              >
                <circle r={r + 4} fill="none" stroke={fill} strokeOpacity={0.25} strokeWidth={1} />
                <circle
                  r={r}
                  fill="var(--card)"
                  stroke={fill}
                  strokeWidth={id === hover ? 2 : 1.4}
                />
                <text
                  textAnchor="middle"
                  y={-2}
                  fontSize={10}
                  fontFamily="JetBrains Mono, monospace"
                  fontWeight={700}
                  fill="var(--foreground)"
                >
                  {p.name.length > 11 ? p.name.slice(0, 10) + "…" : p.name}
                </text>
                <text
                  textAnchor="middle"
                  y={12}
                  fontSize={8}
                  fontFamily="JetBrains Mono, monospace"
                  fill={fill}
                >
                  {p.status.toUpperCase()}
                </text>
              </g>
            );
          })}
        </svg>
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-4 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
        <span className="flex items-center gap-1.5"><span className="h-2 w-2 rounded-full bg-[var(--ok)]" /> ok</span>
        <span className="flex items-center gap-1.5"><span className="h-2 w-2 rounded-full bg-[var(--warn)]" /> warn</span>
        <span className="flex items-center gap-1.5"><span className="h-2 w-2 rounded-full bg-[var(--crit)]" /> crit</span>
      </div>
    </section>
  );
}

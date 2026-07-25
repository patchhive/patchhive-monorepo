import { useMemo, useState } from "react";
import { Cpu, Search } from "lucide-react";

import { PRODUCTS, isWriteCapable } from "@/lib/hive-data";
import { Chip, EmptyDeck, Section } from "./deck-ui";

interface Row {
  id: string;
  label: string;
  description: string;
  productKey: string;
  productName: string;
  write: boolean;
  advertised: boolean;
  observed: boolean;
}

/**
 * Every capability the suite declares, in one searchable table, with whether the
 * product has actually advertised it at runtime.
 */
function buildRows(): Row[] {
  return PRODUCTS.flatMap((product) =>
    product.declared.map((capability) => ({
      id: capability.id,
      label: capability.label,
      description: capability.description,
      productKey: product.key,
      productName: product.name,
      write: isWriteCapable(product),
      advertised: product.observed.actions.includes(capability.id),
      observed: product.observed.observedAt !== null,
    })),
  );
}

export function CapabilityMatrix() {
  const [query, setQuery] = useState("");
  const rows = useMemo(buildRows, []);
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return rows;
    return rows.filter((row) =>
      [row.id, row.label, row.description, row.productName]
        .join(" ")
        .toLowerCase()
        .includes(needle),
    );
  }, [query, rows]);

  return (
    <Section
      id="capabilities"
      title="Capability matrix"
      kicker="What the suite can do, who owns it, and whether it is actually advertised."
      actions={
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1.5 rounded border border-border bg-background/60 px-2 py-1">
            <Search className="h-3 w-3 text-muted-foreground" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Filter capabilities"
              className="w-40 bg-transparent text-xs text-foreground outline-none"
            />
          </div>
          <Chip tone="neutral">{filtered.length}</Chip>
        </div>
      }
    >
      {filtered.length === 0 ? (
        <EmptyDeck title="No capabilities match" detail="Nothing in the registry matches that filter." />
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[720px] border-collapse text-left">
            <thead>
              <tr className="border-b border-border/60 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
                <th className="py-2 pr-3 font-medium">Capability</th>
                <th className="py-2 pr-3 font-medium">Owner</th>
                <th className="py-2 pr-3 font-medium">Effect</th>
                <th className="py-2 font-medium">Advertised</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((row) => (
                <tr
                  key={`${row.productKey}-${row.id}`}
                  className="border-b border-border/30 text-xs"
                >
                  <td className="py-2 pr-3">
                    <div className="flex items-center gap-2">
                      <Cpu className="h-3 w-3 text-muted-foreground" />
                      <code className="font-mono text-[11px] text-foreground">{row.id}</code>
                    </div>
                    <div className="ml-5 text-[11px] text-muted-foreground">{row.description}</div>
                  </td>
                  <td className="py-2 pr-3 font-display text-[11px] font-bold text-foreground">
                    {row.productName}
                  </td>
                  <td className="py-2 pr-3">
                    <Chip tone={row.write ? "warn" : "ok"}>{row.write ? "write" : "read"}</Chip>
                  </td>
                  <td className="py-2">
                    {!row.observed ? (
                      <span className="font-mono text-[11px] text-muted-foreground">
                        not polled
                      </span>
                    ) : row.advertised ? (
                      <Chip tone="ok">yes</Chip>
                    ) : (
                      <Chip tone="crit">missing</Chip>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Section>
  );
}

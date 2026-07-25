import { useMemo, useState } from "react";
import { ExternalLink, Filter, ShieldAlert, ShieldCheck } from "lucide-react";

import {
  PRODUCTS,
  isWriteCapable,
  safetyLabel,
  type Product,
  type ProductStatus,
} from "@/lib/hive-data";
import { Chip, EmptyDeck, Section, StatusDot, type ChipTone } from "./deck-ui";

const statusTone: Record<ProductStatus, ChipTone> = {
  online: "ok",
  degraded: "warn",
  offline: "crit",
  unconfigured: "honey",
  disabled: "neutral",
  unknown: "neutral",
};

type Lens = "all" | "write" | "drift" | "unobserved";

const LENSES: { id: Lens; label: string }[] = [
  { id: "all", label: "All" },
  { id: "write", label: "Write-capable" },
  { id: "drift", label: "Drifted" },
  { id: "unobserved", label: "Never observed" },
];

function matches(product: Product, lens: Lens): boolean {
  switch (lens) {
    case "write":
      return isWriteCapable(product);
    case "drift":
      return product.observed.driftCount > 0;
    case "unobserved":
      return product.observed.observedAt === null;
    default:
      return true;
  }
}

function observedLabel(product: Product): string {
  if (product.observed.observedAt === null) return "never observed";
  const age = Date.now() - Date.parse(product.observed.observedAt);
  if (age < 60_000) return "just now";
  if (age < 3_600_000) return `${Math.round(age / 60_000)}m ago`;
  return `${Math.round(age / 3_600_000)}h ago`;
}

export function ProductRegistry() {
  const [lens, setLens] = useState<Lens>("all");
  const rows = useMemo(() => PRODUCTS.filter((p) => matches(p, lens)), [lens]);

  return (
    <Section
      id="registry"
      title="Product registry"
      kicker="Identity, safety posture, and declared capabilities from the backend product manifests."
      actions={
        <div className="flex items-center gap-1">
          <Filter className="h-3 w-3 text-muted-foreground" />
          {LENSES.map((item) => (
            <button
              key={item.id}
              onClick={() => setLens(item.id)}
              className={`rounded px-2 py-1 font-display text-[10px] uppercase tracking-wider transition ${
                lens === item.id
                  ? "bg-[color-mix(in_oklab,var(--honey)_18%,transparent)] text-[var(--honey)]"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              {item.label}
            </button>
          ))}
        </div>
      }
    >
      {rows.length === 0 ? (
        <EmptyDeck
          title="No products match"
          detail="Nothing in the registry matches this lens."
        />
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[860px] border-collapse text-left">
            <thead>
              <tr className="border-b border-border/60 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
                <th className="py-2 pr-3 font-medium">Product</th>
                <th className="py-2 pr-3 font-medium">Posture</th>
                <th className="py-2 pr-3 font-medium">Stage</th>
                <th className="py-2 pr-3 font-medium">Declared</th>
                <th className="py-2 pr-3 font-medium">Startup</th>
                <th className="py-2 pr-3 font-medium">Drift</th>
                <th className="py-2 pr-3 font-medium">Observed</th>
                <th className="py-2 font-medium">Links</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((product) => (
                <tr
                  key={product.key}
                  className="border-b border-border/30 text-xs transition hover:bg-background/40"
                >
                  <td className="py-2.5 pr-3">
                    <div className="flex items-center gap-2">
                      <StatusDot tone={statusTone[product.observed.status]} />
                      <div>
                        <div className="font-display text-xs font-bold text-foreground">
                          {product.name}
                          <span className="ml-1.5 font-mono text-[10px] text-muted-foreground">
                            {product.code}
                          </span>
                        </div>
                        <div className="text-[11px] text-muted-foreground">{product.role}</div>
                      </div>
                    </div>
                  </td>
                  <td className="py-2.5 pr-3">
                    <Chip
                      tone={isWriteCapable(product) ? "warn" : "ok"}
                      title={`credential scopes: ${product.safety.credentialScopes.join(", ")}`}
                    >
                      {isWriteCapable(product) ? (
                        <ShieldAlert className="h-2.5 w-2.5" />
                      ) : (
                        <ShieldCheck className="h-2.5 w-2.5" />
                      )}
                      {safetyLabel(product.safety)}
                    </Chip>
                    {product.safety.requiresOperatorApproval && (
                      <Chip tone="honey">approval</Chip>
                    )}
                  </td>
                  <td className="py-2.5 pr-3">
                    <Chip tone={product.migrationStage === "integrated" ? "ok" : "neutral"}>
                      {product.migrationStage}
                    </Chip>
                  </td>
                  <td className="py-2.5 pr-3 font-mono text-[11px] text-muted-foreground">
                    {product.declared.length}
                  </td>
                  <td className="py-2.5 pr-3 font-mono text-[11px]">
                    {product.observed.observedAt === null ? (
                      <span className="text-muted-foreground">—</span>
                    ) : (
                      <>
                        <span
                          className={
                            product.observed.startupErrors > 0
                              ? "text-[var(--crit)]"
                              : "text-muted-foreground"
                          }
                        >
                          {product.observed.startupErrors}E
                        </span>
                        <span className="mx-1 text-muted-foreground/40">/</span>
                        <span
                          className={
                            product.observed.startupWarns > 0
                              ? "text-[var(--warn)]"
                              : "text-muted-foreground"
                          }
                        >
                          {product.observed.startupWarns}W
                        </span>
                      </>
                    )}
                  </td>
                  <td className="py-2.5 pr-3">
                    {product.observed.driftCount > 0 ? (
                      <Chip tone="warn">{product.observed.driftCount}</Chip>
                    ) : (
                      <span className="font-mono text-[11px] text-muted-foreground">—</span>
                    )}
                  </td>
                  <td className="py-2.5 pr-3 text-[11px] text-muted-foreground">
                    {observedLabel(product)}
                  </td>
                  <td className="py-2.5">
                    <div className="flex items-center gap-2">
                      <a
                        href={`http://localhost:${product.frontendPort}`}
                        target="_blank"
                        rel="noreferrer"
                        className="inline-flex items-center gap-1 font-mono text-[10px] text-muted-foreground transition hover:text-[var(--honey)]"
                      >
                        :{product.frontendPort} <ExternalLink className="h-2.5 w-2.5" />
                      </a>
                      <span className="font-mono text-[10px] text-muted-foreground/50">
                        api :{product.apiPort}
                      </span>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="mt-4 text-[11px] text-muted-foreground">
        Posture, stage, and declared capabilities mirror{" "}
        <code className="rounded bg-muted px-1 py-0.5 font-mono text-[10px]">
          registry/products/*.toml
        </code>
        . Observed columns stay empty until the deck is wired to{" "}
        <code className="rounded bg-muted px-1 py-0.5 font-mono text-[10px]">GET /products</code>.
      </p>
    </Section>
  );
}

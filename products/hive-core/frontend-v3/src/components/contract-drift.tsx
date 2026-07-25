import { useMemo } from "react";
import { GitCompare, Minus, Plus, ShieldX } from "lucide-react";

import type { Product } from "@/lib/hive-data";
import { capabilityDrift, safetyViolations, type SuiteEvent } from "@/lib/suite-state";
import { Chip, EmptyDeck, Panel, Section } from "./deck-ui";

/**
 * Conformance, not just capability presence. A product whose declared safety posture
 * disagrees with its observed behavior is a failure, not a warning
 * (docs/hivecore-architecture.md §3.14).
 */
export function ContractDrift({
  products,
  events,
}: {
  products: Product[];
  events: SuiteEvent[];
}) {
  const findings = useMemo(
    () => products.map(capabilityDrift).filter((finding) => finding !== null),
    [products],
  );
  const violations = useMemo(() => safetyViolations(events), [events]);
  const observed = products.filter((p) => p.observed.observedAt !== null).length;

  return (
    <Section
      id="drift"
      title="Contract drift inspector"
      kicker="Declared in the manifest vs advertised at runtime. Absence of observation is not drift."
      actions={
        <Chip tone={observed === products.length ? "ok" : "neutral"}>
          {observed}/{products.length} polled
        </Chip>
      }
    >
      {violations.length > 0 && (
        <Panel className="mb-4 border-[var(--crit)]/40 bg-[var(--crit)]/[0.06]">
          <div className="flex items-center gap-2 font-display text-[10px] font-bold uppercase tracking-wider text-[var(--crit)]">
            <ShieldX className="h-3 w-3" /> Safety posture violated
          </div>
          <ul className="mt-2 space-y-1">
            {violations.map((event) => (
              <li key={event.id} className="text-xs text-foreground">
                <span className="font-display font-bold">{event.productKey}</span> declares
                read-only but emitted{" "}
                <code className="rounded bg-background/60 px-1 font-mono text-[10px]">
                  {event.operation}
                </code>
              </li>
            ))}
          </ul>
        </Panel>
      )}

      {findings.length === 0 ? (
        <EmptyDeck
          title={observed === 0 ? "Nothing polled yet" : "No drift detected"}
          detail={
            observed === 0
              ? "Drift is computed by comparing each product's manifest against its live /capabilities response. Wire the poller and this fills in."
              : "Every polled product advertises exactly the capabilities its manifest declares."
          }
          source="registry/products/*.toml vs GET /capabilities"
        />
      ) : (
        <div className="space-y-3">
          {findings.map((finding) => {
            const missing = finding.expected.filter((id) => !finding.actual.includes(id));
            const extra = finding.actual.filter((id) => !finding.expected.includes(id));
            return (
              <Panel key={`${finding.productKey}-${finding.dimension}`}>
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2">
                    <GitCompare className="h-3.5 w-3.5 text-[var(--warn)]" />
                    <span className="font-display text-xs font-bold text-foreground">
                      {finding.productKey}
                    </span>
                    <Chip tone="neutral">{finding.dimension}</Chip>
                  </div>
                  <span className="text-[11px] text-muted-foreground">{finding.note}</span>
                </div>
                <div className="mt-2 flex flex-wrap gap-1.5">
                  {missing.map((id) => (
                    <span
                      key={`m-${id}`}
                      className="inline-flex items-center gap-1 rounded border border-[var(--crit)]/40 px-1.5 py-0.5 font-mono text-[10px] text-[var(--crit)]"
                    >
                      <Minus className="h-2.5 w-2.5" />
                      {id}
                    </span>
                  ))}
                  {extra.map((id) => (
                    <span
                      key={`x-${id}`}
                      className="inline-flex items-center gap-1 rounded border border-[var(--warn)]/40 px-1.5 py-0.5 font-mono text-[10px] text-[var(--warn)]"
                    >
                      <Plus className="h-2.5 w-2.5" />
                      {id}
                    </span>
                  ))}
                </div>
              </Panel>
            );
          })}
        </div>
      )}
    </Section>
  );
}

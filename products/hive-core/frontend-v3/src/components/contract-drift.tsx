import { useEffect, useState } from "react";
import { AlertTriangle, GitCompare, Loader2, ShieldCheck } from "lucide-react";

import {
  fetchConformance,
  type ConformanceFinding,
  type ProductConformance,
} from "@/lib/conformance";

/**
 * Manifest vs runtime, on the axis that matters: the safety boundary.
 *
 * Both sides declare it independently — the manifest in [safety], each advertised
 * action in its own flags — so disagreement is a real conformance failure rather
 * than a naming mismatch between two different vocabularies.
 */
const severityStyle = {
  critical: {
    ring: "border-[var(--crit)]/40",
    text: "text-[var(--crit)]",
    bg: "bg-[var(--crit)]/[0.06]",
  },
  warning: {
    ring: "border-[var(--warn)]/40",
    text: "text-[var(--warn)]",
    bg: "bg-[var(--warn)]/[0.06]",
  },
} as const;

export function ContractDrift({ syncVersion = 0 }: { syncVersion?: number }) {
  const [rows, setRows] = useState<ProductConformance[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    fetchConformance(controller.signal)
      .then((next) => {
        setRows(next);
        setError("");
      })
      .catch((cause) => {
        if (cause instanceof DOMException && cause.name === "AbortError") return;
        setError(cause instanceof Error ? cause.message : "Could not read conformance.");
      })
      .finally(() => setLoading(false));
    return () => controller.abort();
  }, [syncVersion]);

  const observed = rows.filter((row) => row.observed);
  const drifted = observed.filter((row) => row.findings.length > 0);
  const critical = drifted.reduce(
    (sum, row) => sum + row.findings.filter((f) => f.severity === "critical").length,
    0,
  );

  return (
    <section
      id="drift"
      className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur"
    >
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <GitCompare className="h-4 w-4 text-[var(--honey)]" /> Contract Drift Inspector
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Declared safety boundary versus advertised actions. A read-only product offering a
            mutating action is a failure, not a warning.
          </p>
        </div>
        <div className="flex items-center gap-2 font-display text-[10px] uppercase tracking-wider">
          {loading && <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />}
          {critical > 0 && (
            <span className="rounded border border-[var(--crit)]/40 bg-[var(--crit)]/10 px-2 py-1 text-[var(--crit)]">
              {critical} critical
            </span>
          )}
          <span className="rounded border border-border px-2 py-1 text-muted-foreground">
            {observed.length}/{rows.length} engines mounted
          </span>
        </div>
      </div>

      {error ? (
        <div className="rounded-lg border border-[var(--crit)]/40 bg-[var(--crit)]/[0.06] p-4 text-xs text-muted-foreground">
          {error} Conformance needs both <code className="font-mono">/api/products</code> and{" "}
          <code className="font-mono">/api/products/capabilities</code>.
        </div>
      ) : drifted.length === 0 && !loading ? (
        <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed border-border/70 py-10 text-center">
          <ShieldCheck className="h-5 w-5 text-[var(--ok)]" />
          <div className="font-display text-xs font-bold uppercase tracking-wider text-foreground">
            No drift detected
          </div>
          <p className="max-w-md text-xs text-muted-foreground">
            Every mounted engine advertises actions consistent with the safety boundary its
            manifest declares.
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {drifted.map((row) => (
            <div
              key={row.productKey}
              className="rounded-lg border border-border/70 bg-background/40 p-4"
            >
              <div className="flex flex-wrap items-center gap-2">
                <AlertTriangle className="h-3.5 w-3.5 text-[var(--warn)]" />
                <span className="font-display text-xs font-bold text-foreground">
                  {row.productName}
                </span>
                <span className="font-mono text-[10px] text-muted-foreground">
                  {row.actionCount} advertised action{row.actionCount === 1 ? "" : "s"}
                </span>
              </div>
              <ul className="mt-2 space-y-1.5">
                {row.findings.map((finding, index) => (
                  <FindingRow key={`${finding.kind}-${index}`} finding={finding} />
                ))}
              </ul>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function FindingRow({ finding }: { finding: ConformanceFinding }) {
  const style = severityStyle[finding.severity];
  return (
    <li className={`rounded border ${style.ring} ${style.bg} px-2 py-1.5`}>
      <div className="flex flex-wrap items-center gap-2">
        <span className={`font-display text-[9px] uppercase tracking-wider ${style.text}`}>
          {finding.severity}
        </span>
        <span className="font-display text-[10px] uppercase tracking-wider text-foreground">
          {finding.kind}
        </span>
      </div>
      <p className="mt-0.5 text-[11px] text-muted-foreground">{finding.detail}</p>
      <div className="mt-1 flex flex-wrap gap-3 font-mono text-[9px] text-muted-foreground">
        <span>manifest: {finding.declared}</span>
        <span>runtime: {finding.advertised}</span>
      </div>
    </li>
  );
}

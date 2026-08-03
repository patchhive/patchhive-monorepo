import { useEffect, useMemo, useState } from "react";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { CheckCircle2, CircleDot, Clipboard, Loader2, Sparkles, XCircle } from "lucide-react";
import { toast } from "sonner";
import { PRODUCTS, type RunEvent } from "@/lib/hive-data";
import { slugForId } from "@/lib/product-slugs";
import { extractLogLines, fetchRunDetail, type RunDetail } from "@/lib/run-detail";
import { runAnomalyZ } from "@/lib/hive-metrics";
import { explainFailure } from "@/lib/ai.functions";
import { BlastRadius } from "./blast-radius";

/**
 * What one run actually recorded.
 *
 * This drawer used to generate its contents from a seeded PRNG: a payload with an
 * invented trace id, args and retry count; a log stream containing lines like
 * "retry 2/3 after 1200ms" and "upstream_timeout (504) — giving up"; and a timeline
 * with fabricated queue and start times — all attached to real runs.
 *
 * Two things compounded it. "Explain failure" sent those generated log lines to
 * HiveCore's model endpoint as grounding, so the model produced a careful explanation
 * of events that never happened and the drawer presented it as analysis. And "Retry"
 * set a 1400ms timer and then reported "Retry dispatched — New run queued", having
 * dispatched nothing.
 *
 * Everything now comes from the run summary the feed carries plus the product's own
 * record, fetched through HiveCore. When a product stores no detail, the drawer says
 * so: an empty panel is a fact about the product, a populated one made of guesses is a
 * lie about the run.
 *
 * There is no retry button. Re-running an action needs its id and payload, which a run
 * summary does not carry, and dispatching from here would sidestep the approval and
 * credential guards that the dispatch dialog and suite runs enforce.
 */
interface Props {
  run: RunEvent | null;
  onClose: () => void;
}

const statusIcon = {
  standby: CircleDot,
  queued: CircleDot,
  running: CircleDot,
  completed: CheckCircle2,
  failed: XCircle,
  cancelled: XCircle,
  held: CircleDot,
  skipped: CircleDot,
  unknown: CircleDot,
} as const;

const statusTone = {
  standby: "text-muted-foreground",
  queued: "text-[var(--honey)]",
  running: "text-[var(--honey)]",
  completed: "text-[var(--ok)]",
  failed: "text-[var(--crit)]",
  cancelled: "text-[var(--crit)]",
  held: "text-[var(--warn)]",
  skipped: "text-muted-foreground",
  unknown: "text-muted-foreground",
} as const;

/** The run feed labels runs by product display name; the API wants a slug. */
function slugForProductName(name: string): string {
  const product = PRODUCTS.find((item) => item.name === name);
  // No transformation guess for an unknown product: let the API answer "unknown
  // product" rather than have the browser invent a slug that half-works.
  return product ? slugForId(product.id) : name.toLowerCase();
}

export function RunDetailDrawer({ run, onClose }: Props) {
  const [detail, setDetail] = useState<RunDetail | null>(null);
  const [detailError, setDetailError] = useState("");
  const [loading, setLoading] = useState(false);
  const [explaining, setExplaining] = useState(false);
  const [explanation, setExplanation] = useState<string | null>(null);

  useEffect(() => {
    setExplanation(null);
    setDetail(null);
    setDetailError("");
    if (!run) return;

    const controller = new AbortController();
    setLoading(true);
    fetchRunDetail(slugForProductName(run.product), run.id, controller.signal)
      .then((result) => {
        setDetail(result.detail);
        setDetailError(result.message);
      })
      .catch(() => undefined)
      .finally(() => setLoading(false));
    return () => controller.abort();
  }, [run]);

  const logs = useMemo(() => extractLogLines(detail?.detail), [detail]);

  if (!run) return null;

  const Icon = statusIcon[run.status];
  const anomalyZ = runAnomalyZ(run);
  const isAnomaly = anomalyZ !== null && Math.abs(anomalyZ) > 2;
  const hasDetail = Boolean(detail?.detail_ok && detail?.detail);

  // Explaining requires something to explain. Enabled against an empty record, this
  // would hand the model a product name and a status and invite it to fill the gap —
  // the exact failure this rewrite exists to remove.
  const canExplain = run.status === "failed" && hasDetail;

  async function handleExplain(): Promise<void> {
    if (!run || !canExplain) return;
    setExplaining(true);
    setExplanation(null);
    try {
      const result = await explainFailure({
        product: run.product,
        capability: run.capability,
        // Absent rather than invented: a run summary carries no error code or stage,
        // and the backend prompt is written to say the cause is not established
        // rather than guess one.
        error_code: "",
        stage: "",
        message: `${run.product} reported a failed ${run.capability} run.`,
        logs,
        inputs: {
          run_id: run.id,
          ...(run.durationMs === null ? {} : { duration_ms: run.durationMs }),
        },
      });
      setExplanation(result.text);
    } catch (cause) {
      toast.error("Explain failed", {
        description: cause instanceof Error ? cause.message : String(cause),
      });
    } finally {
      setExplaining(false);
    }
  }

  function copyDetail(): void {
    const payload = hasDetail ? detail?.detail : { run_id: run?.id, note: "no product detail" };
    navigator.clipboard
      ?.writeText(JSON.stringify(payload, null, 2))
      .then(() => toast.success("Copied run detail"))
      .catch(() => toast.error("Could not copy"));
  }

  return (
    <Sheet open={Boolean(run)} onOpenChange={(open) => !open && onClose()}>
      <SheetContent className="w-full overflow-y-auto border-l-[var(--honey)]/40 bg-background/95 sm:max-w-xl">
        <SheetHeader className="text-left">
          <div className="flex flex-wrap items-center gap-2">
            <Icon className={`h-4 w-4 ${statusTone[run.status]}`} />
            <SheetTitle className="font-mono text-sm text-[var(--honey)]">{run.id}</SheetTitle>
            {isAnomaly && anomalyZ !== null && (
              <span
                className="rounded border border-[var(--warn)]/50 bg-[var(--warn)]/10 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-[var(--warn)]"
                title={`z = ${anomalyZ.toFixed(2)} against observed runs of this capability`}
              >
                anomaly {anomalyZ > 0 ? "+" : ""}
                {anomalyZ.toFixed(1)}σ
              </span>
            )}
          </div>
          <SheetDescription className="font-display text-[11px] uppercase tracking-wider">
            {run.product} · {run.capability} · {run.status} · {run.ts}
          </SheetDescription>
        </SheetHeader>

        {/* The four fields the run feed actually carries. */}
        <dl className="mt-5 grid grid-cols-2 gap-2">
          <Field label="product" value={run.product} />
          <Field label="capability" value={run.capability} />
          <Field label="status" value={run.status} />
          <Field
            label="duration"
            value={run.durationMs === null ? "not reported" : `${run.durationMs}ms`}
          />
        </dl>

        <div className="mt-4 flex flex-wrap gap-2">
          <Button variant="outline" size="sm" onClick={copyDetail}>
            <Clipboard className="mr-1.5 h-3.5 w-3.5" /> Copy detail
          </Button>
          {run.status === "failed" && (
            <Button
              size="sm"
              onClick={handleExplain}
              disabled={!canExplain || explaining}
              title={
                canExplain
                  ? "Ask HiveCore's model to explain this failure"
                  : "No product-recorded detail to explain"
              }
              className="bg-[var(--honey)] text-background hover:bg-[var(--honey)]/90"
            >
              {explaining ? (
                <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
              ) : (
                <Sparkles className="mr-1.5 h-3.5 w-3.5" />
              )}
              Explain
            </Button>
          )}
        </div>

        {explanation && (
          <div className="mt-3 rounded border border-[var(--honey)]/30 bg-[var(--honey)]/[0.05] p-2">
            <div className="mb-1 font-display text-[9px] uppercase tracking-[0.2em] text-[var(--honey)]">
              explanation · draft
            </div>
            <p className="whitespace-pre-wrap text-xs text-foreground/90">{explanation}</p>
          </div>
        )}

        <section className="mt-5">
          <h3 className="font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            Product record
          </h3>

          {loading ? (
            <p className="mt-2 font-display text-[11px] uppercase tracking-wider text-muted-foreground">
              Reading…
            </p>
          ) : hasDetail ? (
            <>
              {logs.length > 0 && (
                <pre className="mt-2 max-h-56 overflow-auto rounded border border-border/60 bg-background/60 p-2 font-mono text-[10px] leading-relaxed text-foreground/85">
                  {logs.join("\n")}
                </pre>
              )}
              <pre className="mt-2 max-h-72 overflow-auto rounded border border-border/60 bg-background/60 p-2 font-mono text-[10px] text-foreground/80">
                {JSON.stringify(detail?.detail, null, 2)}
              </pre>
            </>
          ) : (
            <p className="mt-2 rounded border border-dashed border-border p-4 text-[11px] text-muted-foreground">
              {detailError ||
                detail?.error ||
                `${run.product} exposes no stored detail for this run${
                  detail?.detail_path ? ` at ${detail.detail_path}` : ""
                }. The summary above is everything the suite recorded.`}
            </p>
          )}
        </section>

        <section className="mt-5">
          <h3 className="mb-2 font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            Blast radius
          </h3>
          <BlastRadius productName={run.product} />
        </section>
      </SheetContent>
    </Sheet>
  );
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded border border-border/60 bg-background/40 p-2">
      <dt className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">
        {label}
      </dt>
      <dd className="mt-0.5 truncate font-mono text-[11px] text-foreground">{value}</dd>
    </div>
  );
}

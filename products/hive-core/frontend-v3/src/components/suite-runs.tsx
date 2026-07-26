import { useEffect, useState } from "react";
import { CheckCircle2, ChevronDown, ChevronRight, Layers, Loader2, Play, XCircle } from "lucide-react";
import { toast } from "sonner";

import {
  fetchDispatchableActions,
  refusalReason,
  type ProductActions,
} from "@/lib/dispatch";
import {
  fetchSuiteRuns,
  startSuiteRun,
  type SuiteRun,
  type SuiteRunStep,
  type SuiteRunStepInput,
} from "@/lib/suite-runs";

/**
 * Compose and run a sequence of product actions.
 *
 * Only actions HiveCore would actually dispatch are offerable — anything
 * destructive, approval-gated or PR-opening is excluded here for the same reason
 * the dispatch dialog disables it, so a suite run cannot become a side door around
 * a guard that blocks the manual path.
 */
const stepTone: Record<string, string> = {
  dispatched: "text-[var(--ok)] border-[var(--ok)]/40",
  failed: "text-[var(--crit)] border-[var(--crit)]/40",
  skipped: "text-muted-foreground border-border",
  queued: "text-muted-foreground border-border",
  running: "text-[var(--honey)] border-[var(--honey)]/40",
};

const runTone: Record<string, string> = {
  completed: "text-[var(--ok)] border-[var(--ok)]/40",
  failed: "text-[var(--crit)] border-[var(--crit)]/40",
  halted: "text-[var(--warn)] border-[var(--warn)]/40",
  running: "text-[var(--honey)] border-[var(--honey)]/40",
};

export function SuiteRuns({ syncVersion = 0 }: { syncVersion?: number }) {
  const [catalog, setCatalog] = useState<ProductActions[]>([]);
  const [runs, setRuns] = useState<SuiteRun[]>([]);
  const [steps, setSteps] = useState<SuiteRunStepInput[]>([]);
  const [name, setName] = useState("");
  const [continueOnFailure, setContinueOnFailure] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([
      fetchDispatchableActions(controller.signal).catch(() => [] as ProductActions[]),
      fetchSuiteRuns(controller.signal).catch(() => [] as SuiteRun[]),
    ])
      .then(([actions, history]) => {
        setCatalog(actions);
        setRuns(history);
      })
      .catch(() => undefined);
    return () => controller.abort();
  }, [syncVersion]);

  function toggle(product: string, action: string) {
    setSteps((current) => {
      const index = current.findIndex(
        (step) => step.product === product && step.action === action,
      );
      if (index >= 0) return current.filter((_, position) => position !== index);
      return [...current, { product, action }];
    });
  }

  async function start() {
    if (steps.length === 0 || busy) return;
    setBusy(true);
    setError("");
    const result = await startSuiteRun(name.trim(), steps, continueOnFailure);
    setBusy(false);

    if (!result.run) {
      setError(result.message);
      toast.error("Suite run did not start", { description: result.message });
      return;
    }

    setRuns((current) => [result.run as SuiteRun, ...current]);
    if (result.ok) {
      toast.success(result.run.name, { description: result.message });
    } else {
      toast.error(`${result.run.name} — ${result.run.status}`, { description: result.message });
    }
  }

  const selectable = catalog
    .map((product) => ({
      ...product,
      actions: product.actions.filter((action) => !refusalReason(action)),
    }))
    .filter((product) => product.actions.length > 0);

  return (
    <section
      id="suite-runs"
      className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur"
    >
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <Layers className="h-4 w-4 text-[var(--honey)]" /> Suite Runs
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Run a sequence of product actions as one unit. Steps execute in order and halt on
            failure unless told otherwise.
          </p>
        </div>
        <span className="rounded border border-border px-2 py-1 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          {runs.length} recorded
        </span>
      </div>

      <div className="rounded-lg border border-border/70 bg-background/40 p-4">
        <div className="mb-2 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          Compose · {steps.length} step{steps.length === 1 ? "" : "s"}
        </div>

        {selectable.length === 0 ? (
          <p className="text-xs text-muted-foreground">
            No dispatchable actions available. Provision service tokens first — a product with no
            scoped token cannot be dispatched to.
          </p>
        ) : (
          <div className="space-y-2">
            {selectable.map((product) => (
              <div key={product.productKey}>
                <div className="mb-1 font-mono text-[10px] text-muted-foreground">
                  {product.productName}
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {product.actions.map((action) => {
                    const position = steps.findIndex(
                      (step) => step.product === product.productKey && step.action === action.id,
                    );
                    const picked = position >= 0;
                    return (
                      <button
                        key={action.id}
                        onClick={() => toggle(product.productKey, action.id)}
                        title={action.description}
                        className={`inline-flex items-center gap-1.5 rounded border px-2 py-1 font-mono text-[10px] transition ${
                          picked
                            ? "border-[var(--honey)] bg-[var(--honey)]/10 text-[var(--honey)]"
                            : "border-border text-foreground hover:border-[var(--honey)]/50"
                        }`}
                      >
                        {picked && <span className="font-display">{position + 1}</span>}
                        {action.id}
                      </button>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        )}

        <div className="mt-3 flex flex-wrap items-center gap-2">
          <input
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Run name (optional)"
            className="flex-1 rounded border border-border bg-background/60 px-2 py-1.5 text-xs text-foreground outline-none focus:border-[var(--honey)]/50"
          />
          <label className="flex items-center gap-1.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
            <input
              type="checkbox"
              checked={continueOnFailure}
              onChange={(event) => setContinueOnFailure(event.target.checked)}
            />
            Continue on failure
          </label>
          <button
            onClick={start}
            disabled={steps.length === 0 || busy}
            className="glow-honey inline-flex items-center gap-2 rounded bg-[var(--honey)] px-4 py-1.5 font-display text-[10px] font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110 disabled:opacity-40"
          >
            {busy ? <Loader2 className="h-3 w-3 animate-spin" /> : <Play className="h-3 w-3" />}
            Run
          </button>
        </div>
        {error && <p className="mt-2 text-[11px] text-[var(--crit)]">{error}</p>}
      </div>

      <div className="mt-4">
        {runs.length === 0 ? (
          <p className="rounded-lg border border-dashed border-border p-6 text-center font-display text-[11px] uppercase tracking-wider text-muted-foreground">
            No suite runs yet.
          </p>
        ) : (
          <ul className="space-y-2">
            {runs.map((run) => (
              <RunRow key={run.id} run={run} />
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

function RunRow({ run }: { run: SuiteRun }) {
  const [open, setOpen] = useState(false);
  return (
    <li className="rounded-lg border border-border/70 bg-background/40">
      <button
        onClick={() => setOpen((value) => !value)}
        className="flex w-full flex-wrap items-center gap-2 px-3 py-2 text-left"
      >
        {open ? (
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
        )}
        <span className="font-display text-xs font-bold text-foreground">{run.name}</span>
        <span
          className={`rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${runTone[run.status] ?? "border-border text-muted-foreground"}`}
        >
          {run.status}
        </span>
        <span className="flex-1 truncate text-[11px] text-muted-foreground">{run.summary}</span>
        <span className="font-mono text-[10px] text-muted-foreground">
          {new Date(run.started_at).toLocaleString()}
        </span>
      </button>
      {open && (
        <ol className="space-y-1 border-t border-border/60 px-3 py-2">
          {run.steps.map((step, index) => (
            <StepRow key={`${step.product}-${step.action}-${index}`} step={step} index={index} />
          ))}
        </ol>
      )}
    </li>
  );
}

function StepRow({ step, index }: { step: SuiteRunStep; index: number }) {
  const Icon =
    step.status === "dispatched" ? CheckCircle2 : step.status === "failed" ? XCircle : Loader2;
  return (
    <li className="flex flex-wrap items-center gap-2 text-[11px]">
      <span className="font-mono text-[10px] text-muted-foreground">{index + 1}.</span>
      <Icon
        className={`h-3 w-3 ${step.status === "dispatched" ? "text-[var(--ok)]" : step.status === "failed" ? "text-[var(--crit)]" : "text-muted-foreground"}`}
      />
      <span className="font-mono text-[10px] text-foreground">
        {step.product}/{step.action}
      </span>
      <span
        className={`rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${stepTone[step.status] ?? "border-border text-muted-foreground"}`}
      >
        {step.status}
      </span>
      <span className="flex-1 truncate text-muted-foreground">{step.message}</span>
      {step.remote_status !== null && (
        <span className="font-mono text-[10px] text-muted-foreground">HTTP {step.remote_status}</span>
      )}
    </li>
  );
}

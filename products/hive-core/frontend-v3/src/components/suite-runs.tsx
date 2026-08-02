import { useEffect, useState } from "react";
import { CheckCircle2, ChevronDown, ChevronRight, Layers, Loader2, Play, ShieldCheck, XCircle } from "lucide-react";
import { toast } from "sonner";

import {
  fetchDispatchableActions,
  refusalReason,
  type ProductActions,
} from "@/lib/dispatch";
import {
  fetchSuiteRuns,
  startSuiteRun,
  MAX_TARGETS_PER_STEP,
  TARGET_PRESETS,
  type SuiteRun,
  type SuiteRunStep,
  type SuiteRunStepInput,
} from "@/lib/suite-runs";

/**
 * Compose and run a sequence of product actions.
 *
 * Only actions HiveCore will evaluate are offerable. Destructive actions stay
 * excluded; approval-gated and PR-opening actions halt as durable pending approvals
 * instead of turning a suite run into a side door around operator authority.
 *
 * Chaining does not change that. A step expanded over ten targets is ten dispatches
 * through the same guard, not one privileged batch — and the fan-out ceiling is the
 * server's, not this form's. Everything here is a request the control plane is free
 * to clamp.
 */
const stepTone: Record<string, string> = {
  dispatched: "text-[var(--ok)] border-[var(--ok)]/40",
  failed: "text-[var(--crit)] border-[var(--crit)]/40",
  skipped: "text-muted-foreground border-border",
  queued: "text-muted-foreground border-border",
  running: "text-[var(--honey)] border-[var(--honey)]/40",
  pending_approval: "text-[var(--warn)] border-[var(--warn)]/40",
};

const runTone: Record<string, string> = {
  completed: "text-[var(--ok)] border-[var(--ok)]/40",
  failed: "text-[var(--crit)] border-[var(--crit)]/40",
  halted: "text-[var(--warn)] border-[var(--warn)]/40",
  running: "text-[var(--honey)] border-[var(--honey)]/40",
  awaiting_approval: "text-[var(--warn)] border-[var(--warn)]/40",
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
      if (index >= 0) {
        // Removing a step renumbers everything after it, so any reference pointing
        // past the removal would silently start meaning a different step. Drop those
        // references rather than let them re-aim themselves.
        const remaining = current.filter((_, position) => position !== index);
        return remaining.map((step) => {
          if (!step.targets) return step;
          if (step.targets.from_step > index) {
            const { targets: _dropped, ...rest } = step;
            return rest;
          }
          return step;
        });
      }
      return [...current, { product, action }];
    });
  }

  function update(position: number, patch: Partial<SuiteRunStepInput>) {
    setSteps((current) =>
      current.map((step, index) => (index === position ? { ...step, ...patch } : step)),
    );
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
    } else if (result.run.status === "awaiting_approval") {
      toast.warning(`${result.run.name} — approval required`, { description: result.message });
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
            failure unless told otherwise. A step can take its targets from an earlier step's
            output and run once per target, up to a cap the control plane enforces.
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

        {steps.length > 0 && (
          <ol className="mt-3 space-y-2">
            {steps.map((step, index) => (
              <StepEditor
                key={`${step.product}-${step.action}-${index}`}
                step={step}
                index={index}
                onChange={(patch) => update(index, patch)}
                onRemove={() => toggle(step.product, step.action)}
              />
            ))}
          </ol>
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

/**
 * One composed step: its payload, and optionally where its targets come from.
 *
 * The payload is edited as JSON rather than a generated form. A generated form needs
 * a schema per action, and the contract does not carry one — inventing fields here
 * would show an operator inputs the product never accepts.
 */
function StepEditor({
  step,
  index,
  onChange,
  onRemove,
}: {
  step: SuiteRunStepInput;
  index: number;
  onChange: (patch: Partial<SuiteRunStepInput>) => void;
  onRemove: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [raw, setRaw] = useState(() =>
    step.payload ? JSON.stringify(step.payload, null, 2) : "",
  );
  const [payloadError, setPayloadError] = useState("");

  function editPayload(next: string) {
    setRaw(next);
    if (!next.trim()) {
      setPayloadError("");
      onChange({ payload: undefined });
      return;
    }
    try {
      const parsed: unknown = JSON.parse(next);
      if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        setPayloadError("Payload must be a JSON object.");
        return;
      }
      setPayloadError("");
      onChange({ payload: parsed });
    } catch {
      // Keep the text so the operator can fix it; withhold it from the run so a
      // half-typed payload is never dispatched as if it were finished.
      setPayloadError("Not valid JSON yet.");
    }
  }

  const targets = step.targets;
  const canReference = index > 0;

  return (
    <li className="rounded border border-border/70 bg-background/40">
      <div className="flex flex-wrap items-center gap-2 px-2 py-1.5">
        <span className="font-display text-[10px] text-[var(--honey)]">{index + 1}</span>
        <span className="font-mono text-[10px] text-foreground">
          {step.product}/{step.action}
        </span>
        {targets && (
          <span className="rounded border border-[var(--honey)]/40 px-1.5 py-0.5 font-mono text-[9px] text-[var(--honey)]">
            ← step {targets.from_step} · {targets.assign_to}
          </span>
        )}
        {payloadError && (
          <span className="font-display text-[9px] uppercase tracking-wider text-[var(--warn)]">
            {payloadError}
          </span>
        )}
        <button
          onClick={() => setOpen((value) => !value)}
          className="ml-auto font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:text-[var(--honey)]"
        >
          {open ? "hide" : "edit"}
        </button>
        <button
          onClick={onRemove}
          className="font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:text-[var(--crit)]"
        >
          remove
        </button>
      </div>

      {open && (
        <div className="space-y-2 border-t border-border/60 px-2 py-2">
          <div>
            <label className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">
              Payload (JSON object)
            </label>
            <textarea
              value={raw}
              onChange={(event) => editPayload(event.target.value)}
              rows={3}
              spellCheck={false}
              placeholder="{}"
              className="mt-1 w-full resize-y rounded border border-border bg-background/60 p-2 font-mono text-[11px] text-foreground outline-none focus:border-[var(--honey)]/50"
            />
          </div>

          {canReference && (
            <div>
              <label className="flex items-center gap-1.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                <input
                  type="checkbox"
                  checked={Boolean(targets)}
                  onChange={(event) =>
                    onChange({
                      targets: event.target.checked
                        ? {
                            from_step: index,
                            path: TARGET_PRESETS[0].path,
                            field: TARGET_PRESETS[0].field,
                            assign_to: TARGET_PRESETS[0].assign_to,
                            max_targets: 5,
                          }
                        : undefined,
                    })
                  }
                />
                Run once per target from an earlier step
              </label>

              {targets && (
                <div className="mt-2 grid gap-2 sm:grid-cols-2">
                  <Field label="From step">
                    <select
                      value={targets.from_step}
                      onChange={(event) =>
                        onChange({ targets: { ...targets, from_step: Number(event.target.value) } })
                      }
                      className={inputClass}
                    >
                      {Array.from({ length: index }, (_, position) => position + 1).map((value) => (
                        <option key={value} value={value}>
                          Step {value}
                        </option>
                      ))}
                    </select>
                  </Field>

                  <Field label="Shape">
                    <select
                      value={`${targets.path}|${targets.field}`}
                      onChange={(event) => {
                        const preset = TARGET_PRESETS.find(
                          (item) => `${item.path}|${item.field}` === event.target.value,
                        );
                        if (preset) {
                          onChange({
                            targets: {
                              ...targets,
                              path: preset.path,
                              field: preset.field,
                              assign_to: preset.assign_to,
                            },
                          });
                        }
                      }}
                      className={inputClass}
                    >
                      {TARGET_PRESETS.map((preset) => (
                        <option key={preset.label} value={`${preset.path}|${preset.field}`}>
                          {preset.label}
                        </option>
                      ))}
                    </select>
                  </Field>

                  <Field label="Path">
                    <input
                      value={targets.path}
                      onChange={(event) =>
                        onChange({ targets: { ...targets, path: event.target.value } })
                      }
                      className={inputClass}
                    />
                  </Field>

                  <Field label="Element field">
                    <input
                      value={targets.field}
                      onChange={(event) =>
                        onChange({ targets: { ...targets, field: event.target.value } })
                      }
                      placeholder="(element is the value)"
                      className={inputClass}
                    />
                  </Field>

                  <Field label="Set payload field">
                    <input
                      value={targets.assign_to}
                      onChange={(event) =>
                        onChange({ targets: { ...targets, assign_to: event.target.value } })
                      }
                      className={inputClass}
                    />
                  </Field>

                  <Field label={`Max targets (server caps at ${MAX_TARGETS_PER_STEP})`}>
                    <input
                      type="number"
                      min={1}
                      max={MAX_TARGETS_PER_STEP}
                      value={targets.max_targets}
                      onChange={(event) =>
                        onChange({
                          targets: {
                            ...targets,
                            max_targets: Math.max(
                              1,
                              Math.min(MAX_TARGETS_PER_STEP, Number(event.target.value) || 1),
                            ),
                          },
                        })
                      }
                      className={inputClass}
                    />
                  </Field>
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </li>
  );
}

const inputClass =
  "w-full rounded border border-border bg-background/60 px-2 py-1 font-mono text-[11px] text-foreground outline-none focus:border-[var(--honey)]/50";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">
        {label}
      </span>
      <div className="mt-0.5">{children}</div>
    </label>
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
    step.status === "dispatched"
      ? CheckCircle2
      : step.status === "pending_approval"
        ? ShieldCheck
        : step.status === "failed"
          ? XCircle
          : Loader2;
  return (
    <li className="flex flex-wrap items-center gap-2 text-[11px]">
      <span className="font-mono text-[10px] text-muted-foreground">{index + 1}.</span>
      <Icon
        className={`h-3 w-3 ${step.status === "dispatched" ? "text-[var(--ok)]" : step.status === "pending_approval" ? "text-[var(--warn)]" : step.status === "failed" ? "text-[var(--crit)]" : "text-muted-foreground"}`}
      />
      <span className="font-mono text-[10px] text-foreground">
        {step.product}/{step.action}
      </span>
      {step.target && (
        <span className="rounded border border-[var(--honey)]/40 px-1 py-0.5 font-mono text-[9px] text-[var(--honey)]">
          {step.target}
        </span>
      )}
      <span
        className={`rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${stepTone[step.status] ?? "border-border text-muted-foreground"}`}
      >
        {step.status}
      </span>
      <span className="flex-1 truncate text-muted-foreground">{step.message}</span>
      {step.remote_status !== null && (
        <span className="font-mono text-[10px] text-muted-foreground">HTTP {step.remote_status}</span>
      )}
      {step.approval_id && (
        <span className="font-mono text-[10px] text-[var(--warn)]">{step.approval_id}</span>
      )}
    </li>
  );
}

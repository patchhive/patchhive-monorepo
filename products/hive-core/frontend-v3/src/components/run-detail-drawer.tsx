import { useEffect, useMemo, useState } from "react";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { AlertOctagon, CheckCircle2, CircleDot, Clipboard, Clock, FileCode, Loader2, RefreshCw, Repeat, Sparkles, Terminal, XCircle } from "lucide-react";
import { toast } from "sonner";
import type { RunEvent } from "@/lib/hive-data";
import { CAPABILITY_SCHEMAS } from "@/lib/hive-extra";
import { useHiveCommand } from "./hive-command";
import { BlastRadius } from "./blast-radius";
import { runAnomalyZ } from "@/lib/hive-metrics";
import { explainFailure } from "@/lib/ai.functions";


interface Props {
  run: RunEvent | null;
  onClose: () => void;
}

function seededRand(seed: string) {
  let h = 2166136261;
  for (let i = 0; i < seed.length; i++) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return () => {
    h ^= h << 13;
    h ^= h >>> 17;
    h ^= h << 5;
    return ((h >>> 0) % 10000) / 10000;
  };
}

function buildPayload(run: RunEvent) {
  const rand = seededRand(run.id);
  return {
    run_id: run.id,
    product: run.product,
    capability: run.capability,
    status: run.status,
    duration_ms: run.durationMs,
    started_at: new Date(Date.now() - Math.floor(rand() * 600_000)).toISOString(),
    actor: "hivecore:dispatcher",
    request: {
      trace_id: `t_${run.id.slice(2)}${Math.floor(rand() * 9999)}`,
      args: {
        target: run.product.toLowerCase(),
        mode: rand() > 0.5 ? "incremental" : "full",
        retries: Math.floor(rand() * 3),
      },
    },
    response:
      run.status === "failed"
        ? { error: "upstream_timeout", code: 504, retriable: true }
        : run.status === "running"
        ? { state: "in_progress", progress: Math.floor(rand() * 80) + 10 }
        : { ok: true, items_processed: Math.floor(rand() * 500) },
  };
}

type LogLevel = "info" | "warn" | "error";
function buildLogs(run: RunEvent): { t: string; level: LogLevel; msg: string }[] {
  const rand = seededRand(run.id + "_logs");
  const base: { level: LogLevel; msg: string }[] = [
    { level: "info", msg: `dispatch ${run.capability} → ${run.product}` },
    { level: "info", msg: `acquire lease lease_${Math.floor(rand() * 9999)}` },
    { level: "info", msg: `loaded context (${Math.floor(rand() * 40) + 4} frames)` },
  ];
  if (run.status === "failed") {
    base.push({ level: "warn", msg: "retry 1/3 after 400ms" });
    base.push({ level: "warn", msg: "retry 2/3 after 1200ms" });
    base.push({ level: "error", msg: "upstream_timeout (504) — giving up" });
  } else if (run.status === "running") {
    base.push({ level: "info", msg: "stream chunk 1 ok" });
    base.push({ level: "info", msg: "stream chunk 2 ok" });
    base.push({ level: "info", msg: "awaiting downstream ack…" });
  } else {
    base.push({ level: "info", msg: `processed ${Math.floor(rand() * 500)} items` });
    base.push({ level: "info", msg: `flushed in ${run.durationMs}ms` });
    base.push({ level: "info", msg: "release lease ok" });
  }
  const start = Date.now() - run.durationMs;
  return base.map((b, i) => ({
    ...b,
    t: new Date(start + i * Math.max(1, Math.floor(run.durationMs / base.length))).toISOString().slice(11, 23),
  }));
}
type Phase = "queued" | "running" | "succeeded" | "failed";
interface TimelineStep {
  key: Phase;
  label: string;
  state: "done" | "active" | "pending" | "failed";
  at: string | null;
  note?: string;
}

function fmtTs(d: Date) {
  return d.toISOString().slice(11, 23) + "Z";
}

interface FailureDetail {
  code: string;
  httpStatus: number;
  stage: string;
  message: string;
  hint: string;
  requestId: string;
  traceId: string;
  attempts: number;
  budgetMs: number;
  inputs: Record<string, string | number | boolean>;
}

function buildTimeline(run: RunEvent): { steps: TimelineStep[]; error: FailureDetail | null } {
  const rand = seededRand(run.id + "_tl");
  const end = Date.now();
  const queuedAt = new Date(end - run.durationMs - Math.floor(rand() * 800) - 200);
  const runningAt = new Date(queuedAt.getTime() + Math.floor(rand() * 400) + 80);
  const finishedAt = new Date(runningAt.getTime() + run.durationMs);

  if (run.status === "running") {
    return {
      steps: [
        { key: "queued", label: "Queued", state: "done", at: fmtTs(queuedAt), note: "accepted by dispatcher" },
        { key: "running", label: "Running", state: "active", at: fmtTs(runningAt), note: "streaming output…" },
        { key: "succeeded", label: "Awaiting completion", state: "pending", at: null },
      ],
      error: null,
    };
  }

  if (run.status === "failed") {
    const payload = buildPayload(run);
    const args = payload.request.args;
    return {
      steps: [
        { key: "queued", label: "Queued", state: "done", at: fmtTs(queuedAt), note: "accepted by dispatcher" },
        { key: "running", label: "Running", state: "done", at: fmtTs(runningAt), note: `${run.durationMs}ms before failure` },
        { key: "failed", label: "Failed", state: "failed", at: fmtTs(finishedAt), note: "upstream_timeout (504)" },
      ],
      error: {
        code: "UPSTREAM_TIMEOUT_504",
        httpStatus: 504,
        stage: `${run.capability} → upstream.dispatch`,
        message: `${run.product} did not respond to ${run.capability} within the 8000ms budget.`,
        hint: "Retried 2× with backoff. Check the target service's /health and re-dispatch once it stabilises.",
        requestId: `req_${run.id.slice(2)}${Math.floor(rand() * 9999).toString().padStart(4, "0")}`,
        traceId: payload.request.trace_id,
        attempts: 3,
        budgetMs: 8000,
        inputs: {
          target: args.target,
          mode: args.mode,
          retries: args.retries,
          capability: run.capability,
        },
      },
    };
  }

  return {
    steps: [
      { key: "queued", label: "Queued", state: "done", at: fmtTs(queuedAt), note: "accepted by dispatcher" },
      { key: "running", label: "Running", state: "done", at: fmtTs(runningAt), note: `executed in ${run.durationMs}ms` },
      { key: "succeeded", label: "Succeeded", state: "done", at: fmtTs(finishedAt), note: "lease released" },
    ],
    error: null,
  };
}


const levelCls = {
  info: "text-muted-foreground",
  warn: "text-[var(--warn)]",
  error: "text-[var(--crit)]",
};

const statusIcon = {
  success: CheckCircle2,
  running: Loader2,
  failed: XCircle,
};

const statusCls = {
  success: "text-[var(--ok)]",
  running: "text-[var(--honey)] animate-spin",
  failed: "text-[var(--crit)]",
};

export function RunDetailDrawer({ run, onClose }: Props) {
  const [retrying, setRetrying] = useState(false);
  const [explaining, setExplaining] = useState(false);
  const [explanation, setExplanation] = useState<string | null>(null);
  const { replayRun } = useHiveCommand();

  useEffect(() => {
    if (!run) { setRetrying(false); setExplanation(null); }
  }, [run]);

  const payload = useMemo(() => (run ? buildPayload(run) : null), [run]);
  const logs = useMemo(() => (run ? buildLogs(run) : []), [run]);
  const timeline = useMemo(() => (run ? buildTimeline(run) : null), [run]);
  const schema = run ? CAPABILITY_SCHEMAS[run.capability] : undefined;

  if (!run) return null;
  const Icon = statusIcon[run.status];
  const anomalyZ = runAnomalyZ(run);
  const isAnomaly = Math.abs(anomalyZ) > 2;

  const handleExplain = async () => {
    if (!timeline?.error) return;
    setExplaining(true);
    setExplanation(null);
    try {
      const res = await explainFailure({
          product: run.product,
          capability: run.capability,
          error_code: timeline.error.code,
          stage: timeline.error.stage,
          message: timeline.error.message,
          logs: logs.map((l) => `[${l.level}] ${l.msg}`),
          inputs: timeline.error.inputs,
      });
      setExplanation(res.text);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Explain failed", { description: msg });
      setExplanation(`AI request failed: ${msg}`);
    } finally {
      setExplaining(false);
    }
  };

  const handleRetry = () => {
    setRetrying(true);
    const toastId = toast.loading(`Retrying ${run.capability}…`, {
      description: `${run.product} · ${run.id}`,
    });
    window.setTimeout(() => {
      setRetrying(false);
      toast.success("Retry dispatched", {
        id: toastId,
        description: `New run queued for ${run.product}`,
      });
    }, 1400);
  };

  const handleReplay = () => replayRun(run);

  const copyPayload = () => {
    navigator.clipboard?.writeText(JSON.stringify(payload, null, 2));
    toast.success("Payload copied");
  };

  return (
    <Sheet open={!!run} onOpenChange={(o) => !o && onClose()}>
      <SheetContent className="w-full overflow-y-auto border-l-[var(--honey)]/40 bg-background/95 sm:max-w-xl">
        <SheetHeader className="text-left">
          <div className="flex items-center gap-2">
            <Icon className={`h-4 w-4 ${statusCls[run.status]}`} />
            <SheetTitle className="font-display uppercase tracking-wider text-[var(--honey)]">
              {run.product} · {run.capability}
            </SheetTitle>
          </div>
          <SheetDescription className="font-display text-[11px] uppercase tracking-wider">
            run <span className="text-[var(--honey)]">{run.id}</span> · {run.status} ·{" "}
            {run.status === "running" ? "in flight" : `${run.durationMs}ms`} · {run.ts}
            {isAnomaly && (
              <span className="ml-2 inline-flex items-center gap-1 rounded border border-[var(--warn)]/50 bg-[var(--warn)]/10 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-[var(--warn)]" title={`z = ${anomalyZ.toFixed(2)} vs capability baseline`}>
                ⚠︎ anomaly {anomalyZ > 0 ? "+" : ""}{anomalyZ.toFixed(1)}σ
              </span>
            )}
          </SheetDescription>
        </SheetHeader>

        <div className="mt-5 flex flex-wrap gap-2">
          <Button
            size="sm"
            onClick={handleRetry}
            disabled={retrying || run.status === "running"}
            className="bg-[var(--honey)] text-background hover:bg-[var(--honey)]/90"
          >
            <RefreshCw className={`mr-1.5 h-3.5 w-3.5 ${retrying ? "animate-spin" : ""}`} />
            {retrying ? "Dispatching…" : "Retry run"}
          </Button>
          <Button size="sm" variant="outline" onClick={handleReplay} disabled={run.status === "running"}>
            <Repeat className="mr-1.5 h-3.5 w-3.5" />
            Replay
          </Button>
          <Button size="sm" variant="outline" onClick={copyPayload}>
            <Clipboard className="mr-1.5 h-3.5 w-3.5" />
            Copy payload
          </Button>
        </div>

        <div className="mt-4">
          <BlastRadius productName={run.product} />
        </div>

        {schema && (
          <section className="mt-6">
            <h3 className="mb-2 flex items-center gap-2 font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              <FileCode className="h-3 w-3 text-[var(--honey)]" /> Capability schema
            </h3>
            <div className="grid gap-2 rounded-lg border border-border bg-muted/20 p-3 sm:grid-cols-2">
              <SchemaBlock label="input" entries={schema.input} />
              <SchemaBlock label="output" entries={schema.output} />
            </div>
          </section>
        )}


        {timeline && (
          <section className="mt-6">
            <h3 className="mb-2 flex items-center gap-2 font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
              <Clock className="h-3 w-3 text-[var(--honey)]" /> Timeline
            </h3>
            <ol className="relative space-y-3 rounded-lg border border-border bg-muted/20 p-4">
              {timeline.steps.map((s, i) => {
                const isLast = i === timeline.steps.length - 1;
                const dotCls =
                  s.state === "done"
                    ? "bg-[var(--ok)] shadow-[0_0_8px_var(--ok)]"
                    : s.state === "active"
                    ? "bg-[var(--honey)] shadow-[0_0_10px_var(--honey)] animate-pulse-dot"
                    : s.state === "failed"
                    ? "bg-[var(--crit)] shadow-[0_0_10px_var(--crit)]"
                    : "bg-muted-foreground/40";
                const labelCls =
                  s.state === "failed"
                    ? "text-[var(--crit)]"
                    : s.state === "active"
                    ? "text-[var(--honey)]"
                    : s.state === "done"
                    ? "text-foreground"
                    : "text-muted-foreground";
                return (
                  <li key={s.key} className="relative flex gap-3 pl-1">
                    <div className="flex flex-col items-center">
                      <span className={`mt-1 h-2 w-2 rounded-full ${dotCls}`} />
                      {!isLast && (
                        <span
                          className={`mt-1 w-px flex-1 ${
                            s.state === "done" ? "bg-[var(--ok)]/40" : "bg-border"
                          }`}
                          style={{ minHeight: 18 }}
                        />
                      )}
                    </div>
                    <div className="min-w-0 flex-1 pb-1">
                      <div className="flex items-baseline justify-between gap-2">
                        <span className={`font-display text-xs font-semibold uppercase tracking-wider ${labelCls}`}>
                          {s.label}
                        </span>
                        <span className="font-display text-[10px] text-muted-foreground">
                          {s.at ?? "—"}
                        </span>
                      </div>
                      {s.note && (
                        <div className="mt-0.5 font-display text-[10px] text-muted-foreground">
                          {s.note}
                        </div>
                      )}
                    </div>
                  </li>
                );
              })}
            </ol>
          </section>
        )}

        {timeline?.error && (
          <section className="mt-4">
            <div className="rounded-lg border border-[var(--crit)]/50 bg-[var(--crit)]/10 p-4">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <AlertOctagon className="h-4 w-4 text-[var(--crit)]" />
                  <span className="font-display text-xs font-bold uppercase tracking-wider text-[var(--crit)]">
                    {timeline.error.code}
                  </span>
                </div>
                <span className="rounded border border-[var(--crit)]/40 bg-background/40 px-1.5 py-0.5 font-display text-[10px] text-[var(--crit)]">
                  HTTP {timeline.error.httpStatus}
                </span>
              </div>
              <p className="mt-2 text-sm text-foreground">{timeline.error.message}</p>
              <p className="mt-1.5 font-display text-[11px] text-muted-foreground">
                {timeline.error.hint}
              </p>

              <dl className="mt-4 grid grid-cols-2 gap-x-4 gap-y-2 border-t border-[var(--crit)]/20 pt-3">
                <FailField label="Stage" value={timeline.error.stage} mono />
                <FailField
                  label="Attempts"
                  value={`${timeline.error.attempts} · budget ${timeline.error.budgetMs}ms`}
                />
                <FailField label="Request ID" value={timeline.error.requestId} mono copyable />
                <FailField label="Trace ID" value={timeline.error.traceId} mono copyable />
              </dl>

              <div className="mt-4 border-t border-[var(--crit)]/20 pt-3">
                <div className="font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
                  Relevant input
                </div>
                <div className="mt-2 flex flex-wrap gap-1.5">
                  {Object.entries(timeline.error.inputs).map(([k, v]) => (
                    <span
                      key={k}
                      className="inline-flex items-center gap-1 rounded border border-border bg-background/60 px-2 py-0.5 font-display text-[10px]"
                    >
                      <span className="text-muted-foreground">{k}</span>
                      <span className="text-[var(--honey)]">{String(v)}</span>
                    </span>
                  ))}
                </div>
              </div>

              <div className="mt-4 border-t border-[var(--crit)]/20 pt-3">
                <Button
                  size="sm"
                  onClick={handleExplain}
                  disabled={explaining}
                  className="bg-[var(--honey)]/90 text-background hover:bg-[var(--honey)]"
                >
                  {explaining ? (
                    <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <Sparkles className="mr-1.5 h-3.5 w-3.5" />
                  )}
                  {explaining ? "Analysing…" : "Explain this failure"}
                </Button>
                {explanation && (
                  <div className="mt-3 rounded-lg border border-[var(--honey)]/30 bg-[var(--honey)]/[0.05] p-3 text-sm leading-relaxed">
                    {explanation}
                  </div>
                )}
              </div>
            </div>
          </section>
        )}



        <section className="mt-6">
          <h3 className="mb-2 flex items-center gap-2 font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            <CircleDot className="h-3 w-3 text-[var(--honey)]" /> Payload
          </h3>
          <pre className="max-h-72 overflow-auto rounded-lg border border-border bg-muted/30 p-3 font-display text-[11px] leading-relaxed">
            {JSON.stringify(payload, null, 2)}
          </pre>
        </section>

        <section className="mt-6 pb-8">
          <h3 className="mb-2 flex items-center gap-2 font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
            <Terminal className="h-3 w-3 text-[var(--honey)]" /> Logs
          </h3>
          <div className="overflow-hidden rounded-lg border border-border bg-black/40 font-display text-[11px]">
            {logs.map((l, i) => (
              <div
                key={i}
                className="flex gap-3 border-b border-border/40 px-3 py-1.5 last:border-b-0"
              >
                <span className="shrink-0 text-muted-foreground/70">{l.t}</span>
                <span className={`shrink-0 uppercase ${levelCls[l.level]}`}>{l.level}</span>
                <span className="text-foreground/90">{l.msg}</span>
              </div>
            ))}
          </div>
        </section>
      </SheetContent>
    </Sheet>
  );
}

function SchemaBlock({ label, entries }: { label: string; entries: Record<string, string> }) {
  const keys = Object.keys(entries);
  return (
    <div className="rounded border border-border bg-background/50 p-2">
      <div className="mb-1 font-display text-[9px] uppercase tracking-[0.2em] text-muted-foreground">{label}</div>
      {keys.length === 0 ? (
        <div className="font-display text-[10px] text-muted-foreground">∅</div>
      ) : (
        <ul className="space-y-0.5 font-display text-[11px]">
          {keys.map((k) => (
            <li key={k} className="flex items-baseline justify-between gap-2">
              <span className="text-foreground/80">{k}</span>
              <span className="text-[var(--honey)]">{entries[k]}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}


function FailField({
  label,
  value,
  mono,
  copyable,
}: {
  label: string;
  value: string;
  mono?: boolean;
  copyable?: boolean;
}) {
  return (
    <div className="min-w-0">
      <dt className="font-display text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
        {label}
      </dt>
      <dd className="mt-0.5 flex items-center gap-1.5">
        <span className={`truncate text-xs ${mono ? "font-display text-[var(--honey)]" : "text-foreground"}`}>
          {value}
        </span>
        {copyable && (
          <button
            type="button"
            onClick={() => {
              navigator.clipboard?.writeText(value);
              toast.success(`${label} copied`);
            }}
            className="shrink-0 rounded p-0.5 text-muted-foreground transition hover:text-[var(--honey)]"
            aria-label={`Copy ${label}`}
          >
            <Clipboard className="h-3 w-3" />
          </button>
        )}
      </dd>
    </div>
  );
}

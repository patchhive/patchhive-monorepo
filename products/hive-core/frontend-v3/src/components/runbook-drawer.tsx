import { useEffect, useMemo, useState } from "react";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { BookOpen, CheckCircle2, Loader2, Play, RotateCcw, ShieldAlert } from "lucide-react";
import { PRODUCTS, type Product } from "@/lib/hive-data";
import { useHiveCommand } from "./hive-command";
import { toast } from "sonner";

interface Step {
  id: string;
  label: string;
  detail: string;
  destructive?: boolean;
}

const GENERIC: Step[] = [
  { id: "health", label: "Probe /health", detail: "Verifies target is responding before any state change." },
  { id: "drain", label: "Drain in-flight requests", detail: "Stop routing new traffic; wait for in-flight to complete (30s)." },
  { id: "rotate", label: "Rotate service token", detail: "TrustGate reissues; downstreams pick up on next call.", destructive: true },
  { id: "restart", label: "Restart worker pool", detail: "Rolling restart across replicas.", destructive: true },
  { id: "verify", label: "Verify /capabilities matches manifest", detail: "Post-condition check; abort playbook if drift persists." },
];

const OVERRIDES: Record<string, Step[]> = {
  vulntriage: [
    { id: "health", label: "Probe /health", detail: "Confirm the pod is answering." },
    { id: "failover", label: "Failover NVD feed to mirror", detail: "Point ingestion at the geo-mirror while primary recovers.", destructive: true },
    { id: "restart", label: "Restart ingest worker pool", detail: "Rolling restart across 4 replicas.", destructive: true },
    { id: "backfill", label: "Backfill missed CVE window", detail: "Replay ingest for the outage duration." },
    { id: "verify", label: "Verify score queue drains", detail: "Score queue depth must fall under 500." },
  ],
  trustgate: [
    { id: "health", label: "Probe /health", detail: "Check auth broker responsiveness." },
    { id: "flush", label: "Flush rotation queue", detail: "Clear stale rotation jobs stuck in retry." },
    { id: "rotate", label: "Force rotate leaking token", detail: "Rotate the specific token flagged by SignalHive.", destructive: true },
    { id: "audit", label: "Emit audit event", detail: "Write signed audit line for compliance." },
  ],
};

function stepsFor(product: Product): Step[] {
  return OVERRIDES[product.id] ?? GENERIC;
}

type StepState = "pending" | "running" | "done" | "skipped";

export function RunbookDrawer() {
  const { runbookProductId, closeRunbook, logAudit, recordRunbook } = useHiveCommand();
  const product = useMemo(
    () => PRODUCTS.find((p) => p.id === runbookProductId) ?? null,
    [runbookProductId],
  );
  const [dryRun, setDryRun] = useState(true);
  const [states, setStates] = useState<Record<string, StepState>>({});
  const [running, setRunning] = useState(false);

  useEffect(() => {
    if (!product) return;
    const init: Record<string, StepState> = {};
    stepsFor(product).forEach((s) => (init[s.id] = "pending"));
    setStates(init);
    setRunning(false);
  }, [product]);

  if (!product) return null;
  const steps = stepsFor(product);

  const runPlaybook = async () => {
    setRunning(true);
    for (const s of steps) {
      if (dryRun && s.destructive) {
        setStates((prev) => ({ ...prev, [s.id]: "skipped" }));
        await sleep(320);
        continue;
      }
      setStates((prev) => ({ ...prev, [s.id]: "running" }));
      await sleep(650 + Math.random() * 400);
      setStates((prev) => ({ ...prev, [s.id]: "done" }));
    }
    setRunning(false);
    logAudit({
      kind: dryRun ? "info" : "destructive",
      title: `Runbook · ${product.name}`,
      detail: `${steps.length} steps ${dryRun ? "dry-run" : "executed"}`,
    });
    recordRunbook({
      productId: product.id,
      productName: product.name,
      dryRun,
      steps: steps.length,
    });
    toast.success(`Runbook ${dryRun ? "dry-run" : "executed"} · ${product.name}`);
  };

  const reset = () => {
    const init: Record<string, StepState> = {};
    steps.forEach((s) => (init[s.id] = "pending"));
    setStates(init);
  };

  return (
    <Sheet open onOpenChange={(o) => !o && closeRunbook()}>
      <SheetContent className="w-full overflow-y-auto border-l-[var(--honey)]/40 bg-background/95 sm:max-w-lg">
        <SheetHeader className="text-left">
          <div className="flex items-center gap-2">
            <BookOpen className="h-4 w-4 text-[var(--honey)]" />
            <SheetTitle className="font-display uppercase tracking-wider text-[var(--honey)]">
              Runbook · {product.name}
            </SheetTitle>
          </div>
          <SheetDescription className="font-display text-[11px] uppercase tracking-wider">
            {steps.length} steps · {steps.filter((s) => s.destructive).length} destructive
          </SheetDescription>
        </SheetHeader>

        <div className="mt-5 flex items-center justify-between rounded-lg border border-border bg-muted/20 p-3">
          <div className="flex items-center gap-2">
            <ShieldAlert className={`h-4 w-4 ${dryRun ? "text-[var(--ok)]" : "text-[var(--crit)]"}`} />
            <div>
              <div className="font-display text-xs font-bold">{dryRun ? "Dry-run mode" : "Live execution"}</div>
              <div className="font-display text-[10px] text-muted-foreground">
                {dryRun ? "Destructive steps will be skipped." : "Destructive steps will apply."}
              </div>
            </div>
          </div>
          <button
            onClick={() => setDryRun((v) => !v)}
            className={`rounded-full border px-3 py-1 font-display text-[10px] uppercase tracking-wider transition ${
              dryRun ? "border-[var(--ok)]/50 text-[var(--ok)]" : "border-[var(--crit)]/60 text-[var(--crit)]"
            }`}
          >
            {dryRun ? "dry-run" : "live"}
          </button>
        </div>

        <div className="mt-4 flex gap-2">
          <Button
            onClick={runPlaybook}
            disabled={running}
            className="bg-[var(--honey)] text-background hover:bg-[var(--honey)]/90"
          >
            {running ? <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" /> : <Play className="mr-1.5 h-3.5 w-3.5" />}
            {running ? "Executing…" : dryRun ? "Dry-run playbook" : "Execute playbook"}
          </Button>
          <Button variant="outline" onClick={reset} disabled={running}>
            <RotateCcw className="mr-1.5 h-3.5 w-3.5" /> Reset
          </Button>
        </div>

        <ol className="mt-5 space-y-2">
          {steps.map((s) => {
            const st = states[s.id] ?? "pending";
            const dot =
              st === "done"
                ? "bg-[var(--ok)] shadow-[0_0_8px_var(--ok)]"
                : st === "running"
                ? "bg-[var(--honey)] shadow-[0_0_8px_var(--honey)] animate-pulse-dot"
                : st === "skipped"
                ? "bg-muted-foreground/60"
                : "bg-muted-foreground/30";
            return (
              <li key={s.id} className="flex items-start gap-3 rounded-lg border border-border bg-background/40 p-3">
                <span className={`mt-1 h-2 w-2 shrink-0 rounded-full ${dot}`} />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="font-display text-xs font-bold">{s.label}</span>
                    {s.destructive && (
                      <span className="rounded border border-[var(--crit)]/40 bg-[var(--crit)]/10 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-[var(--crit)]">
                        destructive
                      </span>
                    )}
                    {st === "skipped" && (
                      <span className="rounded border border-border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                        skipped (dry-run)
                      </span>
                    )}
                    {st === "done" && <CheckCircle2 className="h-3 w-3 text-[var(--ok)]" />}
                  </div>
                  <div className="mt-0.5 font-display text-[11px] text-muted-foreground">{s.detail}</div>
                </div>
              </li>
            );
          })}
        </ol>
      </SheetContent>
    </Sheet>
  );
}

function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

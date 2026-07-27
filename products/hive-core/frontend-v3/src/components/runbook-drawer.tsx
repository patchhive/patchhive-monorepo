import { useState } from "react";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import {
  AlertTriangle,
  BookOpen,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Loader2,
  Play,
  ShieldCheck,
  XCircle,
} from "lucide-react";
import { PRODUCTS } from "@/lib/hive-data";
import { slugForId } from "@/lib/product-slugs";
import { runProductRunbook, STEP_TONE, type RunbookRun, type RunbookStep } from "@/lib/runbooks";
import { useHiveCommand } from "./hive-command";
import { toast } from "sonner";

/**
 * A recorded diagnostic pass over one product.
 *
 * The previous version was theatre: hardcoded steps like "Restart worker pool" and
 * "Force rotate leaking token", executed by sleeping ~700ms each and marking them
 * done, then writing an audit entry of kind "destructive" saying it had happened. A
 * fabricated metric misleads; a fabricated audit trail of destructive operations
 * corrupts the record you would consult to find out what was actually done.
 *
 * HiveCore now performs real read-only checks and returns what it saw. The dry-run
 * toggle is gone because there is nothing to guard against — its presence implied the
 * alternative was live destructive execution, which was never true in either mode.
 *
 * If a step could act on a product it would belong in dispatch or a suite run, where
 * approval, scope and credential guards already live. A diagnostic panel must not
 * become a side door around them.
 */
export function RunbookDrawer() {
  const { runbookProductId, closeRunbook, logAudit } = useHiveCommand();
  const [running, setRunning] = useState(false);
  const [run, setRun] = useState<RunbookRun | null>(null);
  const [error, setError] = useState("");

  const product = PRODUCTS.find((item) => item.id === runbookProductId);
  if (!product) return null;

  const name = product.name;

  async function execute(): Promise<void> {
    setRunning(true);
    setError("");
    const result = await runProductRunbook(slugForId(runbookProductId as string));
    setRunning(false);

    if (!result.run) {
      setError(result.message);
      toast.error(`Runbook failed · ${name}`, { description: result.message });
      return;
    }

    setRun(result.run);
    // Logged as info, never as "destructive": nothing was changed.
    logAudit({
      kind: "info",
      title: `Runbook · ${name}`,
      detail: `${result.run.status} — ${result.run.summary}`,
    });

    if (result.run.status === "ok") {
      toast.success(`Runbook clean · ${name}`, { description: result.run.summary });
    } else {
      toast.warning(`Runbook ${result.run.status} · ${name}`, {
        description: result.run.summary,
      });
    }
  }

  return (
    <Sheet open onOpenChange={(open) => !open && closeRunbook()}>
      <SheetContent className="w-full overflow-y-auto border-l-[var(--honey)]/40 bg-background/95 sm:max-w-lg">
        <SheetHeader className="text-left">
          <div className="flex items-center gap-2">
            <BookOpen className="h-4 w-4 text-[var(--honey)]" />
            <SheetTitle className="font-display uppercase tracking-wider text-[var(--honey)]">
              Runbook · {name}
            </SheetTitle>
          </div>
          <SheetDescription className="font-display text-[11px] uppercase tracking-wider">
            {run ? run.summary : "Not run yet"}
          </SheetDescription>
        </SheetHeader>

        <div className="mt-5 flex items-start gap-2 rounded-lg border border-border bg-muted/20 p-3">
          <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-[var(--ok)]" />
          <div>
            <div className="font-display text-xs font-bold">Read-only</div>
            <p className="mt-0.5 text-[11px] text-muted-foreground">
              Every check observes; none of them change anything. Acting on a product is a
              dispatch or a suite run, where the approval and credential guards are.
            </p>
          </div>
        </div>

        <div className="mt-4">
          <Button
            onClick={execute}
            disabled={running}
            className="bg-[var(--honey)] text-background hover:bg-[var(--honey)]/90"
          >
            {running ? (
              <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
            ) : (
              <Play className="mr-1.5 h-3.5 w-3.5" />
            )}
            {running ? "Checking…" : run ? "Run again" : "Run diagnostic"}
          </Button>
        </div>

        {error && <p className="mt-3 text-[11px] text-[var(--crit)]">{error}</p>}

        {run ? (
          <ol className="mt-5 space-y-2">
            {run.steps.map((step) => (
              <StepRow key={step.id} step={step} />
            ))}
          </ol>
        ) : (
          !running && (
            <p className="mt-5 rounded-lg border border-dashed border-border p-6 text-center font-display text-[11px] uppercase tracking-wider text-muted-foreground">
              Run the diagnostic to see what HiveCore can observe about {name}.
            </p>
          )
        )}
      </SheetContent>
    </Sheet>
  );
}

function StepRow({ step }: { step: RunbookStep }) {
  const [open, setOpen] = useState(false);
  const Icon =
    step.status === "ok"
      ? CheckCircle2
      : step.status === "fail"
        ? XCircle
        : step.status === "warn"
          ? AlertTriangle
          : ChevronRight;

  const tone = STEP_TONE[step.status] ?? "text-muted-foreground border-border";

  return (
    <li className="rounded-lg border border-border bg-background/40 p-3">
      <div className="flex items-start gap-2">
        <Icon className={`mt-0.5 h-3.5 w-3.5 shrink-0 ${tone.split(" ")[0]}`} />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-display text-xs font-bold">{step.label}</span>
            <span
              className={`rounded border px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider ${tone}`}
            >
              {step.status}
            </span>
          </div>
          <p className="mt-1 text-[11px] text-muted-foreground">{step.message}</p>

          {/* Evidence is collapsed but always present: a check an operator cannot
              inspect is one they have to take on faith, which is how the previous
              version got away with reporting work it never did. */}
          <button
            onClick={() => setOpen((value) => !value)}
            className="mt-1.5 inline-flex items-center gap-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:text-[var(--honey)]"
          >
            {open ? (
              <ChevronDown className="h-2.5 w-2.5" />
            ) : (
              <ChevronRight className="h-2.5 w-2.5" />
            )}
            evidence
          </button>
          {open && (
            <pre className="mt-1 max-h-48 overflow-auto rounded border border-border/60 bg-background/60 p-2 font-mono text-[10px] text-foreground/80">
              {JSON.stringify(step.evidence, null, 2)}
            </pre>
          )}
        </div>
      </div>
    </li>
  );
}

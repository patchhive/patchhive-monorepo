import { useState } from "react";
import { BookOpen, ChevronDown, ChevronRight } from "lucide-react";

import {
  PROCEDURES,
  PROCEDURE_LABELS,
  type ProcedureRun,
  type ProcedureStep,
} from "@/lib/suite-state";
import { Chip, EmptyDeck, Panel, Section, type ChipTone } from "./deck-ui";

/**
 * Runbook history bound to procedures that already have real backends: the four
 * smoke tiers (first_stack_smoke_runs) and fleet launch jobs, both of which already
 * persist per-step evidence. Not invented playbooks — an operator procedure that can
 * execute outside the policy kernel is a safety hole.
 */
const stepTone: Record<ProcedureStep["status"], ChipTone> = {
  queued: "neutral",
  running: "honey",
  pass: "ok",
  warn: "warn",
  fail: "crit",
  skipped: "neutral",
};

const runTone: Record<ProcedureRun["status"], ChipTone> = {
  queued: "neutral",
  running: "honey",
  ready: "ok",
  attention: "warn",
  blocked: "warn",
  failed: "crit",
};

export function ProcedureHistory() {
  return (
    <Section
      id="procedures"
      title="Runbook history"
      kicker="Smoke tiers, fleet launches, pairing, and reconciliation sweeps — with the evidence each step produced."
    >
      {PROCEDURES.length === 0 ? (
        <>
          <EmptyDeck
            title="No procedure runs recorded"
            detail="Smoke tiers already persist step evidence server-side; fleet launch jobs currently live in memory and are lost on restart. Both belong here."
            source="GET /setup/first-stack · first_stack_smoke_runs"
          />
          <div className="mt-4 flex flex-wrap gap-1.5">
            {Object.entries(PROCEDURE_LABELS).map(([kind, label]) => (
              <Chip key={kind} tone="neutral">
                {label}
              </Chip>
            ))}
          </div>
        </>
      ) : (
        <ul className="space-y-2">
          {PROCEDURES.map((procedure) => (
            <ProcedureRow key={procedure.id} procedure={procedure} />
          ))}
        </ul>
      )}
    </Section>
  );
}

function ProcedureRow({ procedure }: { procedure: ProcedureRun }) {
  const [open, setOpen] = useState(false);
  return (
    <li>
      <Panel>
        <button
          onClick={() => setOpen((value) => !value)}
          className="flex w-full items-center gap-2 text-left"
        >
          {open ? (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
          )}
          <BookOpen className="h-3.5 w-3.5 text-[var(--honey)]" />
          <span className="font-display text-xs font-bold text-foreground">
            {PROCEDURE_LABELS[procedure.kind]}
          </span>
          <span className="flex-1 truncate text-xs text-muted-foreground">
            {procedure.summary}
          </span>
          <span className="font-mono text-[10px] text-muted-foreground">
            {new Date(procedure.startedAt).toLocaleString()}
          </span>
          <Chip tone={runTone[procedure.status]}>{procedure.status}</Chip>
        </button>
        {open && (
          <ol className="mt-3 space-y-1 border-l border-border/60 pl-4">
            {procedure.steps.map((step) => (
              <li key={step.key} className="flex flex-wrap items-center gap-2 text-[11px]">
                <Chip tone={stepTone[step.status]}>{step.status}</Chip>
                <span className="font-display font-bold text-foreground">{step.label}</span>
                <Chip tone="neutral">{step.phase}</Chip>
                <span className="text-muted-foreground">{step.message}</span>
              </li>
            ))}
          </ol>
        )}
      </Panel>
    </li>
  );
}

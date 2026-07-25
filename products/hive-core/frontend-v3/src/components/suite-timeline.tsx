import { useMemo, useState } from "react";
import { AlertTriangle, Clock, Coins, Gavel, Power, Rocket } from "lucide-react";

import {
  SUITE_EVENTS,
  type SuiteEvent,
  type SuiteEventKind,
  type SuiteEventLevel,
} from "@/lib/suite-state";
import { Chip, EmptyDeck, Panel, Section, type ChipTone } from "./deck-ui";

// Not an on-call incident feed. PatchHive has no pagers and no MTTR; it has
// degradations and blocks. This is the "what did the suite do overnight" surface.
const kindIcon: Record<SuiteEventKind, typeof Clock> = {
  policy_decision: Gavel,
  budget: Coins,
  dispatch: Rocket,
  approval: Gavel,
  product_state: Power,
  mandate: Clock,
  operator: AlertTriangle,
};

const levelTone: Record<SuiteEventLevel, ChipTone> = {
  info: "neutral",
  warn: "warn",
  crit: "crit",
};

const FILTERS: { id: "all" | SuiteEventKind; label: string }[] = [
  { id: "all", label: "All" },
  { id: "product_state", label: "Availability" },
  { id: "policy_decision", label: "Policy" },
  { id: "budget", label: "Budget" },
  { id: "approval", label: "Approvals" },
  { id: "mandate", label: "Mandates" },
];

export function SuiteTimeline() {
  const [filter, setFilter] = useState<"all" | SuiteEventKind>("all");
  const events = useMemo(
    () => (filter === "all" ? SUITE_EVENTS : SUITE_EVENTS.filter((e) => e.kind === filter)),
    [filter],
  );
  const open = SUITE_EVENTS.filter((e) => e.level === "crit").length;

  return (
    <Section
      id="timeline"
      title="Suite timeline"
      kicker="Degradations, blocks, denials, and pauses — the ordered record of what the suite decided."
      actions={
        <div className="flex items-center gap-1">
          {open > 0 && <Chip tone="crit">{open} open</Chip>}
          {FILTERS.map((item) => (
            <button
              key={item.id}
              onClick={() => setFilter(item.id)}
              className={`rounded px-2 py-1 font-display text-[10px] uppercase tracking-wider transition ${
                filter === item.id
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
      {events.length === 0 ? (
        <EmptyDeck
          title="No suite events recorded"
          detail="This is the append-only ledger every other panel projects from: product quarantined, gate unavailable so work blocked, ceiling hit, mandate paused, policy denied. It does not exist in the backend yet."
          source="suite_events (architecture doc §3.3)"
        />
      ) : (
        <ol className="relative space-y-2 border-l border-border/60 pl-5">
          {events.map((event) => (
            <TimelineRow key={event.id} event={event} />
          ))}
        </ol>
      )}
    </Section>
  );
}

function TimelineRow({ event }: { event: SuiteEvent }) {
  const [open, setOpen] = useState(false);
  const Icon = kindIcon[event.kind];
  return (
    <li className="relative">
      <span
        className="absolute -left-[26px] top-1.5 grid h-3.5 w-3.5 place-items-center rounded-full border border-border bg-background"
        aria-hidden="true"
      >
        <Icon className="h-2 w-2 text-muted-foreground" />
      </span>
      <button
        onClick={() => setOpen((value) => !value)}
        className="w-full rounded px-2 py-1.5 text-left transition hover:bg-background/40"
      >
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-mono text-[10px] text-muted-foreground">
            {new Date(event.ts).toLocaleTimeString()}
          </span>
          <Chip tone={levelTone[event.level]}>{event.kind.replace("_", " ")}</Chip>
          {event.productKey && <Chip tone="neutral">{event.productKey}</Chip>}
          <span className="text-xs text-foreground">{event.summary}</span>
        </div>
      </button>
      {open && event.reasonChain.length > 0 && (
        <Panel className="ml-2 mt-1">
          <div className="font-display text-[10px] uppercase tracking-wider text-muted-foreground">
            Reason chain
          </div>
          <ol className="mt-1.5 space-y-1">
            {event.reasonChain.map((reason, index) => (
              <li key={reason} className="flex gap-2 text-[11px] text-foreground">
                <span className="font-mono text-muted-foreground">{index + 1}.</span>
                {reason}
              </li>
            ))}
          </ol>
        </Panel>
      )}
    </li>
  );
}

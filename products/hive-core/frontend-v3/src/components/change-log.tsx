import { useState } from "react";
import { ScrollText } from "lucide-react";

import type { SuiteEvent } from "@/lib/suite-state";
import { Chip, EmptyDeck, Section, type ChipTone } from "./deck-ui";

/**
 * Operator-visible change log, projected from the same append-only ledger as every
 * other panel. Deliberately not "undoable": you can cancel a pending approval or
 * release a reservation, but you cannot un-open a pull request, and an affordance
 * that implies otherwise is a lie.
 */
const tone: Record<SuiteEvent["level"], ChipTone> = {
  info: "neutral",
  warn: "warn",
  crit: "crit",
};

const PAGE = 20;

export function ChangeLog({ events }: { events: SuiteEvent[] }) {
  const [limit, setLimit] = useState(PAGE);
  const all = events.filter(
    (event) => event.kind === "operator" || event.kind === "dispatch",
  );
  const entries = all.slice(0, limit);

  return (
    <Section
      id="changelog"
      title="Change log"
      kicker="Every operator action and dispatch, with the actor and the request that carried it."
      actions={<Chip tone="neutral">{all.length} entries</Chip>}
    >
      {all.length === 0 ? (
        <EmptyDeck
          title="No operator actions recorded"
          detail="HiveCore already records its own dispatches in product_action_events. Folding that into the shared event ledger makes it queryable alongside policy decisions and budget grants."
          source="GET /events"
        />
      ) : (
        <>
        <ul className="divide-y divide-border/40">
          {entries.map((event) => (
            <li key={event.id} className="flex items-center gap-3 py-2 text-xs">
              <ScrollText className="h-3.5 w-3.5 flex-shrink-0 text-muted-foreground" />
              <span className="font-mono text-[10px] text-muted-foreground">
                {new Date(event.ts).toLocaleString()}
              </span>
              <span className="font-display text-[11px] font-bold text-foreground">
                {event.actor}
              </span>
              <span className="flex-1 truncate text-muted-foreground">{event.summary}</span>
              {event.productKey && <Chip tone="neutral">{event.productKey}</Chip>}
              <Chip tone={tone[event.level]}>{event.operation}</Chip>
            </li>
          ))}
        </ul>
        {all.length > limit && (
          <button
            onClick={() => setLimit((value) => value + PAGE)}
            className="mt-3 rounded border border-border px-3 py-1.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:text-foreground"
          >
            Show {Math.min(PAGE, all.length - limit)} more of {all.length}
          </button>
        )}
        </>
      )}
    </Section>
  );
}

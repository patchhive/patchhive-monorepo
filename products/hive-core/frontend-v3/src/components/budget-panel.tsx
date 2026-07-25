import { useMemo, useState } from "react";
import { AlertTriangle, Coins } from "lucide-react";

import { PRODUCTS_BY_KEY } from "@/lib/hive-data";
import { BUDGETS, stalledReservations } from "@/lib/suite-state";
import { Chip, EmptyDeck, Metric, Panel, Section } from "./deck-ui";

/**
 * The most load-bearing safety surface in the deck. Two things must always be
 * visible: how much outbound capacity is left, and why the last denial happened.
 */
export function BudgetPanel() {
  const [showZero, setShowZero] = useState(false);
  const rows = useMemo(
    () => BUDGETS.products.filter((row) => showZero || row.limit > 0),
    [showZero],
  );
  const stalled = useMemo(() => stalledReservations(BUDGETS.reservations), []);
  const pressure = BUDGETS.suiteLimit === 0 ? 1 : BUDGETS.suiteUsed / BUDGETS.suiteLimit;

  return (
    <Section
      id="budgets"
      title="Outbound capacity"
      kicker="Concurrent open PatchHive pull requests. The suite ceiling always wins over any product maximum."
      actions={
        <button
          onClick={() => setShowZero((value) => !value)}
          className="rounded px-2 py-1 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:text-foreground"
        >
          {showZero ? "Hide unbudgeted" : "Show all products"}
        </button>
      }
    >
      <div className="grid gap-3 sm:grid-cols-3">
        <Metric
          label="Suite ceiling"
          value={BUDGETS.suiteLimit}
          tone="honey"
          detail="all products combined"
        />
        <Metric
          label="In flight"
          value={BUDGETS.suiteUsed}
          tone={pressure >= 1 ? "crit" : pressure > 0.7 ? "warn" : "ok"}
          detail="reserved + committed"
        />
        <Metric
          label="Remaining"
          value={BUDGETS.suiteRemaining}
          tone={BUDGETS.suiteRemaining === 0 ? "crit" : "ok"}
          detail="effective headroom"
        />
      </div>

      {stalled.length > 0 && (
        <Panel className="mt-4 border-[var(--crit)]/40 bg-[var(--crit)]/[0.06]">
          <div className="flex items-center gap-2 font-display text-[10px] font-bold uppercase tracking-wider text-[var(--crit)]">
            <AlertTriangle className="h-3 w-3" /> Possible leaked capacity
          </div>
          <p className="mt-1 text-xs text-foreground">
            {stalled.length} committed reservation{stalled.length === 1 ? "" : "s"} older than 72h.
            Committed slots are released only by the owning product's PR monitor; if that monitor
            missed the merge, the slot is consumed permanently and the ceiling ratchets down.
          </p>
        </Panel>
      )}

      <div className="mt-4 overflow-x-auto">
        <table className="w-full min-w-[520px] border-collapse text-left">
          <thead>
            <tr className="border-b border-border/60 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
              <th className="py-2 pr-3 font-medium">Product</th>
              <th className="py-2 pr-3 font-medium">Limit</th>
              <th className="py-2 pr-3 font-medium">Used</th>
              <th className="py-2 pr-3 font-medium">Effective</th>
              <th className="py-2 font-medium">Bound by</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => {
              const effective = Math.min(row.remaining, BUDGETS.suiteRemaining);
              const boundBySuite = BUDGETS.suiteRemaining < row.remaining;
              return (
                <tr key={row.productKey} className="border-b border-border/30 text-xs">
                  <td className="py-2 pr-3 font-display text-xs font-bold text-foreground">
                    {PRODUCTS_BY_KEY[row.productKey]?.name ?? row.productKey}
                  </td>
                  <td className="py-2 pr-3 font-mono text-[11px] text-muted-foreground">
                    {row.limit}
                  </td>
                  <td className="py-2 pr-3 font-mono text-[11px] text-muted-foreground">
                    {row.used}
                  </td>
                  <td className="py-2 pr-3 font-mono text-[11px] text-foreground">{effective}</td>
                  <td className="py-2">
                    {row.limit === 0 ? (
                      <Chip tone="neutral">no budget granted</Chip>
                    ) : boundBySuite ? (
                      <Chip tone="warn">suite ceiling</Chip>
                    ) : (
                      <Chip tone="ok">product maximum</Chip>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div className="mt-4 grid gap-3 lg:grid-cols-2">
        <div>
          <div className="mb-2 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
            Live reservations
          </div>
          {BUDGETS.reservations.length === 0 ? (
            <EmptyDeck
              title="No reservations held"
              detail="A slot is reserved immediately before PR creation and committed with the PR URL."
              source="GET /pr-budgets"
            />
          ) : (
            <ul className="space-y-1">
              {BUDGETS.reservations.map((reservation) => (
                <li
                  key={reservation.id}
                  className="flex items-center gap-2 rounded border border-border/60 px-2 py-1.5 text-xs"
                >
                  <Coins className="h-3 w-3 text-[var(--honey)]" />
                  <span className="font-mono text-[10px] text-muted-foreground">
                    {reservation.id}
                  </span>
                  <span className="flex-1 truncate text-foreground">{reservation.repository}</span>
                  <Chip tone={reservation.status === "committed" ? "ok" : "honey"}>
                    {reservation.status}
                  </Chip>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div>
          <div className="mb-2 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
            Last denial
          </div>
          {BUDGETS.lastDenial === null ? (
            <EmptyDeck
              title="No denials recorded"
              detail="When a reservation is refused, the layer that won and the exact reason belong here — a denial is product evidence, not a server error."
            />
          ) : (
            <Panel>
              <Chip tone="warn">{BUDGETS.lastDenial.layer} limit</Chip>
              <p className="mt-2 text-xs text-foreground">{BUDGETS.lastDenial.reason}</p>
            </Panel>
          )}
        </div>
      </div>
    </Section>
  );
}

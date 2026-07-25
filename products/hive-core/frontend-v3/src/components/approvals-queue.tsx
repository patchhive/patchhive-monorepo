import { Check, ShieldQuestion, X } from "lucide-react";

import { APPROVALS } from "@/lib/suite-state";
import { PRODUCTS_BY_KEY } from "@/lib/hive-data";
import { Chip, EmptyDeck, Panel, Section } from "./deck-ui";

/**
 * The queue that unblocks dispatch. HiveCore's backend currently refuses any action
 * marked requires_approval or opens_pr with a 403 that literally says the approval
 * flow does not exist (dispatch.rs). This is that flow's surface.
 *
 * A grant is bound to one product + action + repository + run + input hash. It is
 * single-use: approving run X must never authorize run Y.
 */
export function ApprovalsQueue() {
  const pending = APPROVALS.filter((item) => item.state === "pending");

  return (
    <Section
      id="approvals"
      title="Approvals"
      kicker="Mutating dispatches waiting on an operator. Grants are scoped to one dispatch and expire."
      actions={pending.length > 0 ? <Chip tone="honey">{pending.length} pending</Chip> : undefined}
    >
      {APPROVALS.length === 0 ? (
        <EmptyDeck
          title="No approvals requested"
          detail="Until approvals exist as objects, HiveCore refuses every approval-gated and PR-opening action outright. This queue is what turns that refusal into a decision."
          source="POST /approvals (architecture doc §3.5)"
        />
      ) : (
        <ul className="space-y-2">
          {APPROVALS.map((request) => (
            <li key={request.id}>
              <Panel className="flex flex-wrap items-center gap-3">
                <ShieldQuestion className="h-4 w-4 flex-shrink-0 text-[var(--honey)]" />
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-display text-xs font-bold text-foreground">
                      {PRODUCTS_BY_KEY[request.productKey]?.name ?? request.productKey}
                    </span>
                    <code className="rounded bg-muted px-1 py-0.5 font-mono text-[10px] text-foreground">
                      {request.actionId}
                    </code>
                    <Chip tone="neutral">{request.repository}</Chip>
                  </div>
                  <p className="mt-1 truncate text-xs text-muted-foreground">{request.summary}</p>
                  <div className="mt-1 flex flex-wrap items-center gap-2 font-mono text-[10px] text-muted-foreground">
                    <span>run {request.runId}</span>
                    <span>input {request.inputHash.slice(0, 12)}</span>
                    <span>expires {new Date(request.expiresAt).toLocaleString()}</span>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    className="inline-flex items-center gap-1 rounded border border-[var(--ok)]/40 px-2 py-1 font-display text-[10px] uppercase tracking-wider text-[var(--ok)] transition hover:bg-[var(--ok)]/10"
                    disabled
                    title="Wire POST /approvals/:id/grant to enable"
                  >
                    <Check className="h-3 w-3" /> Grant
                  </button>
                  <button
                    className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:text-foreground"
                    disabled
                    title="Wire POST /approvals/:id/deny to enable"
                  >
                    <X className="h-3 w-3" /> Deny
                  </button>
                </div>
              </Panel>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}

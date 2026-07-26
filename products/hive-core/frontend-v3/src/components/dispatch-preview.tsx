import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Ban, Loader2, Rocket, ShieldCheck } from "lucide-react";
import { toast } from "sonner";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  dispatchAction,
  fetchDispatchableActions,
  refusalReason,
  type DispatchableAction,
  type DispatchOutcome,
  type ProductActions,
} from "@/lib/dispatch";
import { useHiveCommand } from "./hive-command";

/**
 * Real dispatch, through HiveCore.
 *
 * This replaced a preview that computed a fabricated blast radius and toasted
 * "Dispatch queued" without calling anything. What an operator needs before firing
 * an action is what it will touch and whether it will be refused — both are known
 * from the advertised capability metadata, so both are shown up front.
 */
interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function DispatchPreview({ open, onOpenChange }: Props) {
  const { logAudit } = useHiveCommand();
  const [catalog, setCatalog] = useState<ProductActions[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [selected, setSelected] = useState<{ product: string; action: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<DispatchOutcome | null>(null);
  const [payload, setPayload] = useState("{}");
  const [payloadError, setPayloadError] = useState("");

  useEffect(() => {
    if (!open) return;
    const controller = new AbortController();
    setLoading(true);
    setOutcome(null);
    fetchDispatchableActions(controller.signal)
      .then((rows) => {
        setCatalog(rows);
        setError("");
      })
      .catch((cause) => {
        if (cause instanceof DOMException && cause.name === "AbortError") return;
        setError(cause instanceof Error ? cause.message : "Could not read capabilities.");
      })
      .finally(() => setLoading(false));
    return () => controller.abort();
  }, [open]);

  const active = useMemo(() => {
    if (!selected) return null;
    const product = catalog.find((row) => row.productKey === selected.product);
    const action = product?.actions.find((item) => item.id === selected.action);
    return product && action ? { product, action } : null;
  }, [selected, catalog]);

  const refusal = active ? refusalReason(active.action) : null;

  async function fire() {
    if (!active || refusal || busy) return;

    // Dispatching a blind {} is how an action with a dangerous default gets fired
    // without the operator ever seeing the field that mattered. MergeKeeper's
    // assess action defaulted publish_report to true, so an empty body wrote to
    // GitHub from something labelled read-only.
    let parsed: unknown;
    try {
      parsed = JSON.parse(payload.trim() || "{}");
    } catch {
      setPayloadError("Request body must be valid JSON.");
      return;
    }
    setPayloadError("");

    setBusy(true);
    const result = await dispatchAction(active.product.productKey, active.action.id, parsed);
    setBusy(false);
    setOutcome(result);
    logAudit({
      kind: result.ok ? "action" : "info",
      title: result.ok ? "Dispatched" : "Dispatch refused",
      detail: `${active.product.productName} · ${active.action.id} — ${result.message}`,
    });
    if (result.ok) {
      toast.success(`${active.product.productName} · ${active.action.id}`, {
        description: result.message,
      });
    } else {
      toast.error("Dispatch did not run", { description: result.message });
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="border-[var(--honey)]/40 bg-background/95 sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 font-display uppercase tracking-wider text-[var(--honey)]">
            <Rocket className="h-4 w-4" /> Dispatch an action
          </DialogTitle>
          <DialogDescription>
            HiveCore dispatches on your behalf using the downstream service token it holds. Only
            actions each product actually advertises appear here.
          </DialogDescription>
        </DialogHeader>

        {loading ? (
          <div className="flex items-center gap-2 py-8 text-xs text-muted-foreground">
            <Loader2 className="h-3.5 w-3.5 animate-spin" /> Reading advertised capabilities…
          </div>
        ) : error ? (
          <div className="rounded-lg border border-[var(--crit)]/40 bg-[var(--crit)]/[0.06] p-3 text-xs text-muted-foreground">
            {error}
          </div>
        ) : (
          <div className="max-h-[46vh] space-y-3 overflow-y-auto pr-1">
            {catalog.map((product) => (
              <div key={product.productKey}>
                <div className="mb-1 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
                  {product.productName}
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {product.actions.map((action) => (
                    <ActionChip
                      key={action.id}
                      action={action}
                      selected={
                        selected?.product === product.productKey && selected?.action === action.id
                      }
                      onSelect={() =>
                        {
                        setSelected({ product: product.productKey, action: action.id });
                        setPayload("{}");
                        setPayloadError("");
                        setOutcome(null);
                      }}
                    />
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}

        {active && (
          <div className="rounded-lg border border-border bg-background/60 p-3">
            <div className="font-display text-xs font-bold text-foreground">
              {active.product.productName} · {active.action.label}
            </div>
            {active.action.description && (
              <p className="mt-1 text-[11px] text-muted-foreground">{active.action.description}</p>
            )}
            <div className="mt-2 flex flex-wrap gap-3 font-mono text-[10px] text-muted-foreground">
              <span>
                {active.action.method} {active.action.path}
              </span>
              {active.action.startsRun && <span>starts a run</span>}
              {active.action.requiredScopes.length > 0 && (
                <span>scopes: {active.action.requiredScopes.join(", ")}</span>
              )}
            </div>
            {active.action.credentialRequirements.length > 0 && (
              <div className="mt-1 font-mono text-[10px] text-muted-foreground">
                credentials: {active.action.credentialRequirements.join(", ")}
              </div>
            )}

            <div className="mt-3">
              <label
                htmlFor="dispatch-payload"
                className="font-display text-[10px] uppercase tracking-wider text-muted-foreground"
              >
                Request body
              </label>
              <textarea
                id="dispatch-payload"
                value={payload}
                onChange={(event) => setPayload(event.target.value)}
                spellCheck={false}
                rows={4}
                className="mt-1 w-full rounded border border-border bg-background/60 px-2 py-1.5 font-mono text-[11px] text-foreground outline-none focus:border-[var(--honey)]/50"
              />
              <p className="mt-1 text-[10px] text-muted-foreground">
                Sent verbatim. Omitted fields take the product's own defaults, which are not
                always the safe option — check the product's docs before dispatching blind.
              </p>
              {payloadError && (
                <p className="mt-1 text-[10px] text-[var(--crit)]">{payloadError}</p>
              )}
            </div>

            {refusal ? (
              <div className="mt-3 flex items-start gap-2 rounded border border-[var(--warn)]/40 bg-[var(--warn)]/[0.06] px-2 py-1.5">
                <Ban className="mt-0.5 h-3 w-3 flex-shrink-0 text-[var(--warn)]" />
                <span className="text-[11px] text-muted-foreground">{refusal}</span>
              </div>
            ) : active.action.mutating ? (
              // Not refused is not the same as safe. This line said "Read-only and
              // dispatchable" for anything HiveCore would accept, so a mutating
              // action read as harmless — the exact claim that hid MergeKeeper's
              // GitHub write.
              <div className="mt-3 flex items-start gap-2 rounded border border-[var(--warn)]/40 bg-[var(--warn)]/[0.06] px-2 py-1.5">
                <AlertTriangle className="mt-0.5 h-3 w-3 flex-shrink-0 text-[var(--warn)]" />
                <span className="text-[11px] text-muted-foreground">
                  Mutating — this action writes external state. Dispatchable, but check the
                  request body before firing.
                </span>
              </div>
            ) : (
              <div className="mt-3 flex items-center gap-2 text-[11px] text-[var(--ok)]">
                <ShieldCheck className="h-3 w-3" /> Read-only and dispatchable.
              </div>
            )}
          </div>
        )}

        {outcome && (
          <div
            className={`rounded-lg border p-3 ${
              outcome.ok
                ? "border-[var(--ok)]/40 bg-[var(--ok)]/[0.06]"
                : "border-[var(--crit)]/40 bg-[var(--crit)]/[0.06]"
            }`}
          >
            <div className="flex items-center gap-2 font-display text-[10px] uppercase tracking-wider">
              {outcome.ok ? (
                <ShieldCheck className="h-3 w-3 text-[var(--ok)]" />
              ) : (
                <AlertTriangle className="h-3 w-3 text-[var(--crit)]" />
              )}
              <span className={outcome.ok ? "text-[var(--ok)]" : "text-[var(--crit)]"}>
                {outcome.status}
              </span>
              {outcome.remoteStatus !== null && (
                <span className="font-mono text-muted-foreground">
                  HTTP {outcome.remoteStatus}
                </span>
              )}
            </div>
            <p className="mt-1 text-[11px] text-muted-foreground">{outcome.message}</p>
            {outcome.eventId && (
              <code className="mt-1 block font-mono text-[10px] text-muted-foreground">
                {outcome.eventId}
              </code>
            )}
          </div>
        )}

        <div className="flex justify-end gap-2">
          <button
            onClick={() => onOpenChange(false)}
            className="rounded border border-border px-3 py-1.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:text-foreground"
          >
            Close
          </button>
          <button
            onClick={fire}
            disabled={!active || Boolean(refusal) || busy}
            className="glow-honey inline-flex items-center gap-2 rounded bg-[var(--honey)] px-4 py-1.5 font-display text-[10px] font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110 disabled:opacity-40"
          >
            {busy ? <Loader2 className="h-3 w-3 animate-spin" /> : <Rocket className="h-3 w-3" />}
            Dispatch
          </button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function ActionChip({
  action,
  selected,
  onSelect,
}: {
  action: DispatchableAction;
  selected: boolean;
  onSelect: () => void;
}) {
  const blocked = Boolean(refusalReason(action));
  return (
    <button
      onClick={onSelect}
      title={action.description}
      className={`inline-flex items-center gap-1.5 rounded border px-2 py-1 font-mono text-[10px] transition ${
        selected
          ? "border-[var(--honey)] bg-[var(--honey)]/10 text-[var(--honey)]"
          : blocked
            ? "border-border text-muted-foreground hover:border-[var(--warn)]/40"
            : "border-border text-foreground hover:border-[var(--honey)]/50"
      }`}
    >
      {blocked ? (
        <Ban className="h-2.5 w-2.5 text-[var(--warn)]" />
      ) : action.mutating ? (
        <AlertTriangle className="h-2.5 w-2.5 text-[var(--warn)]" />
      ) : null}
      {action.id}
    </button>
  );
}

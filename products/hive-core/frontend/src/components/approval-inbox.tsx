import { useCallback, useEffect, useMemo, useState } from "react";
import { Ban, Check, Clock3, Loader2, RotateCcw, ShieldCheck, Zap } from "lucide-react";
import { toast } from "sonner";

import {
  denyApproval,
  dispatchApproved,
  fetchApprovals,
  grantApproval,
  revokeApproval,
  type ApprovalRecord,
} from "@/lib/approvals";

const stateTone: Record<string, string> = {
  pending: "border-[var(--warn)]/40 text-[var(--warn)]",
  granted: "border-[var(--ok)]/40 text-[var(--ok)]",
  consuming: "border-[var(--honey)]/40 text-[var(--honey)]",
  consumed: "border-[var(--ok)]/40 text-[var(--ok)]",
  denied: "border-[var(--crit)]/40 text-[var(--crit)]",
  revoked: "border-[var(--crit)]/40 text-[var(--crit)]",
  expired: "border-border text-muted-foreground",
  unknown: "border-[var(--crit)]/40 text-[var(--crit)]",
};

function effectLabel(approval: ApprovalRecord): string {
  const effect = approval.subject.effect;
  if (effect.kind === "mutates_repository") {
    return effect.opens_pull_request ? "mutates repository + opens PR" : "mutates repository";
  }
  return effect.kind.replaceAll("_", " ");
}

function originLabel(approval: ApprovalRecord): string {
  const origin = approval.subject.origin;
  return origin.origin === "suite_run" ? `suite run ${origin.run_id}` : "operator dispatch";
}

export function ApprovalInbox({ syncVersion = 0 }: { syncVersion?: number }) {
  const [approvals, setApprovals] = useState<ApprovalRecord[]>([]);
  const [reasons, setReasons] = useState<Record<string, string>>({});
  const [busyId, setBusyId] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const refresh = useCallback(async (signal?: AbortSignal) => {
    try {
      setApprovals(await fetchApprovals(signal));
      setError("");
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      setError(cause instanceof Error ? cause.message : "Could not read approvals.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    void refresh(controller.signal);
    return () => controller.abort();
  }, [refresh, syncVersion]);

  const counts = useMemo(() => {
    const result = { pending: 0, granted: 0, consuming: 0 };
    for (const approval of approvals) {
      const state = approval.lifecycle.state;
      if (state === "pending" || state === "granted" || state === "consuming") result[state] += 1;
    }
    return result;
  }, [approvals]);

  async function act(approval: ApprovalRecord, action: "grant" | "deny" | "revoke" | "dispatch") {
    setBusyId(approval.id);
    try {
      const reason = reasons[approval.id]?.trim() ?? "";
      if (action === "grant") await grantApproval(approval.id);
      if (action === "deny") await denyApproval(approval.id, reason);
      if (action === "revoke") await revokeApproval(approval.id, reason);
      if (action === "dispatch") await dispatchApproved(approval.id);
      const success = {
        grant: "Approval granted once",
        deny: "Approval denied",
        revoke: "Approval revoked",
        dispatch: "Approved action dispatched",
      }[action];
      toast.success(success);
      await refresh();
    } catch (cause) {
      toast.error(`Could not ${action} approval`, {
        description: cause instanceof Error ? cause.message : "Unknown approval error.",
      });
    } finally {
      setBusyId("");
    }
  }

  return (
    <section id="approvals" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <ShieldCheck className="h-4 w-4 text-[var(--honey)]" /> Approval Inbox
          </h2>
          <p className="mt-1 max-w-3xl text-xs text-muted-foreground">
            Grant one exact product action once. Changing its input, target, origin, scopes, or safety effect changes the fingerprint and requires a new approval.
          </p>
        </div>
        <div className="flex gap-2 font-display text-[9px] uppercase tracking-wider">
          <span className="rounded border border-[var(--warn)]/40 px-2 py-1 text-[var(--warn)]">{counts.pending} pending</span>
          <span className="rounded border border-[var(--ok)]/40 px-2 py-1 text-[var(--ok)]">{counts.granted} granted</span>
          <span className="rounded border border-[var(--honey)]/40 px-2 py-1 text-[var(--honey)]">{counts.consuming} consuming</span>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center gap-2 py-8 text-xs text-muted-foreground"><Loader2 className="h-3.5 w-3.5 animate-spin" /> Reading approval state…</div>
      ) : error ? (
        <div className="rounded-lg border border-[var(--crit)]/40 bg-[var(--crit)]/[0.06] p-3 text-xs text-muted-foreground">{error}</div>
      ) : approvals.length === 0 ? (
        <div className="rounded-lg border border-dashed border-border p-8 text-center text-xs text-muted-foreground">No approval-gated dispatches have been proposed.</div>
      ) : (
        <div className="space-y-3">
          {approvals.map((approval) => {
            const state = approval.lifecycle.state;
            const reason = reasons[approval.id] ?? "";
            const busy = busyId === approval.id;
            return (
              <article key={approval.id} className="rounded-lg border border-border/70 bg-background/40 p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <div className="font-display text-xs font-bold text-foreground">{approval.subject.product} · {approval.subject.action_label}</div>
                    <div className="mt-1 font-mono text-[9px] text-muted-foreground">{approval.id}</div>
                  </div>
                  <span className={`rounded border px-2 py-1 font-display text-[9px] uppercase tracking-wider ${stateTone[state] ?? stateTone.unknown}`}>{state}</span>
                </div>

                <div className="mt-3 grid gap-2 text-[11px] sm:grid-cols-2 lg:grid-cols-4">
                  <Fact label="Effect" value={effectLabel(approval)} />
                  <Fact label="Origin" value={originLabel(approval)} />
                  <Fact label="Repository" value={approval.subject.repository ?? "from approved input/discovery"} />
                  <Fact label="Run" value={approval.subject.run_id ?? "new operator dispatch"} />
                </div>

                <div className="mt-3 flex flex-wrap gap-1.5 font-mono text-[9px] text-muted-foreground">
                  {approval.subject.required_scopes.map((scope) => <span key={scope} className="rounded border border-border px-1.5 py-0.5">{scope}</span>)}
                  <span className="rounded border border-border px-1.5 py-0.5">input {approval.subject.input_hash.slice(0, 12)}</span>
                </div>

                <details className="mt-3">
                  <summary className="cursor-pointer font-display text-[9px] uppercase tracking-wider text-[var(--honey)]">Exact dispatch input</summary>
                  <pre className="mt-2 max-h-64 overflow-auto rounded border border-border bg-background/60 p-2 font-mono text-[10px] text-muted-foreground">{JSON.stringify(approval.dispatch, null, 2)}</pre>
                </details>

                <details className="mt-2">
                  <summary className="cursor-pointer font-display text-[9px] uppercase tracking-wider text-muted-foreground">Lifecycle evidence</summary>
                  <pre className="mt-2 max-h-48 overflow-auto rounded border border-border bg-background/60 p-2 font-mono text-[10px] text-muted-foreground">{JSON.stringify(approval.lifecycle, null, 2)}</pre>
                </details>

                {(state === "pending" || state === "granted") && (
                  <div className="mt-3 flex flex-wrap items-center gap-2">
                    <input
                      value={reason}
                      onChange={(event) => setReasons((current) => ({ ...current, [approval.id]: event.target.value }))}
                      placeholder="Reason required for deny or revoke"
                      className="min-w-64 flex-1 rounded border border-border bg-background/60 px-2 py-1.5 text-[11px] outline-none focus:border-[var(--honey)]/50"
                    />
                    {state === "pending" ? (
                      <>
                        <ActionButton disabled={busy} onClick={() => void act(approval, "grant")} icon={Check}>Grant once</ActionButton>
                        <ActionButton disabled={busy || !reason.trim()} onClick={() => void act(approval, "deny")} icon={Ban}>Deny</ActionButton>
                      </>
                    ) : (
                      <>
                        <ActionButton disabled={busy} onClick={() => void act(approval, "dispatch")} icon={Zap}>Dispatch</ActionButton>
                        <ActionButton disabled={busy || !reason.trim()} onClick={() => void act(approval, "revoke")} icon={RotateCcw}>Revoke</ActionButton>
                      </>
                    )}
                  </div>
                )}

                {approval.history.length > 0 && (
                  <div className="mt-3 border-t border-border/70 pt-3">
                    <div className="mb-1 flex items-center gap-1.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground"><Clock3 className="h-3 w-3" /> Audit history</div>
                    {approval.history.map((event) => (
                      <div key={event.id} className="grid gap-2 py-1 text-[10px] sm:grid-cols-[80px_1fr_auto]">
                        <span className="font-mono text-foreground">{event.event}</span><span className="text-muted-foreground">{event.reason}</span><span className="font-mono text-muted-foreground">{new Date(event.created_at).toLocaleString()}</span>
                      </div>
                    ))}
                  </div>
                )}
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div><div className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">{label}</div><div className="mt-0.5 text-foreground">{value}</div></div>;
}

function ActionButton({ disabled, onClick, icon: Icon, children }: { disabled: boolean; onClick: () => void; icon: typeof Check; children: string }) {
  return <button disabled={disabled} onClick={onClick} className="inline-flex items-center gap-1.5 rounded border border-[var(--honey)]/40 px-2.5 py-1.5 font-display text-[9px] font-bold uppercase tracking-wider text-[var(--honey)] transition hover:bg-[var(--honey)]/10 disabled:opacity-40"><Icon className="h-3 w-3" />{children}</button>;
}

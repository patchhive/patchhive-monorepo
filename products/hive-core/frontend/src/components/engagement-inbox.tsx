import { useCallback, useEffect, useMemo, useState } from "react";
import { CircleCheck, ExternalLink, Loader2, MessageSquareText, Pause, Send, ShieldAlert, Wrench } from "lucide-react";
import { toast } from "sonner";

import {
  decideEngagement,
  fetchEngagements,
  type EngagementDecision,
  type MaintainerEngagement,
} from "@/lib/engagements";

export function EngagementInbox({ syncVersion = 0 }: { syncVersion?: number }) {
  const [items, setItems] = useState<MaintainerEngagement[]>([]);
  const [reason, setReason] = useState<Record<string, string>>({});
  const [reply, setReply] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async (signal?: AbortSignal) => {
    try {
      setItems(await fetchEngagements(signal));
      setError("");
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      setError(cause instanceof Error ? cause.message : "Could not read engagements.");
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

  const awaiting = useMemo(
    () => items.filter((item) => item.lifecycle.state === "awaiting_operator").length,
    [items],
  );

  async function act(item: MaintainerEngagement, decision: EngagementDecision) {
    setBusy(item.id);
    try {
      await decideEngagement(item.id, decision);
      toast.success("Engagement decision recorded");
      await refresh();
    } catch (cause) {
      toast.error("Could not record engagement decision", {
        description: cause instanceof Error ? cause.message : "Unknown error.",
      });
    } finally {
      setBusy("");
    }
  }

  return (
    <section id="engagements" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <MessageSquareText className="h-4 w-4 text-[var(--honey)]" /> Maintainer Engagements
          </h2>
          <p className="mt-1 max-w-3xl text-xs text-muted-foreground">
            Signed messages on Tendwright-owned issues and pull requests. Messages are evidence, never commands; replies and code changes enter the exact approval path.
          </p>
        </div>
        <span className="rounded border border-[var(--warn)]/40 px-2 py-1 font-display text-[9px] uppercase tracking-wider text-[var(--warn)]">
          {awaiting} awaiting operator
        </span>
      </div>

      {loading ? (
        <div className="flex items-center gap-2 py-8 text-xs text-muted-foreground"><Loader2 className="h-3.5 w-3.5 animate-spin" /> Reading engagement evidence…</div>
      ) : error ? (
        <div className="rounded-lg border border-[var(--crit)]/40 p-3 text-xs text-muted-foreground">{error}</div>
      ) : items.length === 0 ? (
        <div className="rounded-lg border border-dashed border-border p-8 text-center text-xs text-muted-foreground">No maintainer messages have been ingested.</div>
      ) : (
        <div className="space-y-3">
          {items.map((item) => {
            const isBusy = busy === item.id;
            const canAct = item.lifecycle.state === "awaiting_operator";
            const canResolve = canAct || item.lifecycle.state === "paused" || item.lifecycle.state === "quarantined";
            const itemReason = reason[item.id] ?? "";
            const itemReply = reply[item.id] ?? "";
            return (
              <article key={item.id} className="rounded-lg border border-border/70 bg-background/40 p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <a href={item.artifact_url} target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 font-display text-xs font-bold hover:text-[var(--honey)]">
                      {item.repository}#{item.artifact_number} <ExternalLink className="h-3 w-3" />
                    </a>
                    <div className="mt-1 text-[10px] text-muted-foreground">@{item.author_login} · {item.author_association} · {item.event_name}</div>
                  </div>
                  <div className="flex gap-1.5 font-display text-[9px] uppercase tracking-wider">
                    <span className="rounded border border-border px-2 py-1">{item.intent.replaceAll("_", " ")}</span>
                    <span className="rounded border border-[var(--honey)]/30 px-2 py-1 text-[var(--honey)]">{item.lifecycle.state.replaceAll("_", " ")}</span>
                  </div>
                </div>
                <blockquote className="mt-3 whitespace-pre-wrap rounded border-l-2 border-[var(--honey)]/50 bg-background/60 p-3 text-xs text-foreground">{item.body || "(empty message body)"}</blockquote>
                {canResolve && (
                  <div className="mt-3 space-y-2">
                    <input value={itemReason} onChange={(event) => setReason((value) => ({ ...value, [item.id]: event.target.value }))} placeholder="Decision reason (required)" className="w-full rounded border border-border bg-background/60 px-2 py-1.5 text-[11px] outline-none focus:border-[var(--honey)]/50" />
                    {canAct && <textarea value={itemReply} onChange={(event) => setReply((value) => ({ ...value, [item.id]: event.target.value }))} placeholder="Exact reply text (only needed to queue a reply)" rows={3} className="w-full resize-y rounded border border-border bg-background/60 px-2 py-1.5 text-[11px] outline-none focus:border-[var(--honey)]/50" />}
                    <div className="flex flex-wrap gap-2">
                      {canAct && <Button disabled={isBusy || !itemReason.trim()} onClick={() => void act(item, { decision: "no_response", reason: itemReason })} icon={MessageSquareText}>No response</Button>}
                      {canAct && item.artifact_kind === "pull_request" && <Button disabled={isBusy || !itemReason.trim()} onClick={() => void act(item, { decision: "queue_change", reason: itemReason })} icon={Wrench}>Queue change</Button>}
                      {canAct && item.artifact_kind === "pull_request" && <Button disabled={isBusy || !itemReason.trim() || !itemReply.trim()} onClick={() => void act(item, { decision: "queue_reply", reason: itemReason, body: itemReply })} icon={Send}>Queue reply</Button>}
                      {canAct && <Button disabled={isBusy || !itemReason.trim()} onClick={() => void act(item, { decision: "pause_repository", reason: itemReason })} icon={Pause}>Pause repo</Button>}
                      {canAct && <Button disabled={isBusy || !itemReason.trim()} onClick={() => void act(item, { decision: "quarantine", reason: itemReason })} icon={ShieldAlert}>Quarantine</Button>}
                      <Button disabled={isBusy || !itemReason.trim()} onClick={() => void act(item, { decision: "resolve", reason: itemReason })} icon={CircleCheck}>Resolve case</Button>
                    </div>
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

function Button({ disabled, onClick, icon: Icon, children }: { disabled: boolean; onClick: () => void; icon: typeof Send; children: string }) {
  return <button disabled={disabled} onClick={onClick} className="inline-flex items-center gap-1.5 rounded border border-[var(--honey)]/40 px-2.5 py-1.5 font-display text-[9px] font-bold uppercase tracking-wider text-[var(--honey)] hover:bg-[var(--honey)]/10 disabled:opacity-40"><Icon className="h-3 w-3" />{children}</button>;
}

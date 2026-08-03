import { useCallback, useEffect, useMemo, useState } from "react";
import { Eye, Fingerprint, ListTodo, Loader2, ShieldOff } from "lucide-react";

import {
  fetchFindingReceipts,
  fetchWorkItems,
  type FindingReceipt,
  type WorkItem,
  type WorkOrigin,
} from "@/lib/work-ledger";

const PAGE_SIZE = 6;

function originLabel(origin: WorkOrigin): string {
  switch (origin.origin) {
    case "operator": return "operator";
    case "product_run": return `${origin.product_slug} / ${origin.run_id}`;
    case "suite_run": return `suite run / ${origin.run_id}`;
    case "conductor_tick": return `conductor / ${origin.tick_id}`;
  }
}

export function WorkLedger({ syncVersion = 0 }: { syncVersion?: number }) {
  const [items, setItems] = useState<WorkItem[]>([]);
  const [receipts, setReceipts] = useState<FindingReceipt[]>([]);
  const [visible, setVisible] = useState(PAGE_SIZE);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const refresh = useCallback(async (signal?: AbortSignal) => {
    try {
      const [nextItems, nextReceipts] = await Promise.all([
        fetchWorkItems(signal),
        fetchFindingReceipts(signal),
      ]);
      setItems(nextItems);
      setReceipts(nextReceipts);
      setError("");
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      setError(cause instanceof Error ? cause.message : "Could not read the work ledger.");
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
    let discovered = 0;
    let unknown = 0;
    for (const item of items) {
      if (item.lifecycle.state === "discovered") discovered += 1;
      else unknown += 1;
    }
    return { discovered, unknown };
  }, [items]);

  const receiptsByItem = useMemo(() => {
    const grouped = new Map<string, FindingReceipt[]>();
    for (const receipt of receipts) {
      const current = grouped.get(receipt.work_item_id) ?? [];
      current.push(receipt);
      grouped.set(receipt.work_item_id, current);
    }
    return grouped;
  }, [receipts]);

  return (
    <section id="ledger" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <ListTodo className="h-4 w-4 text-[var(--honey)]" /> Work Ledger
          </h2>
          <p className="mt-1 max-w-3xl text-xs text-muted-foreground">
            Durable proposed work, deduplicated by kind, repository, and subject identity. This conductor slice records intent only.
          </p>
        </div>
        <div className="flex flex-wrap gap-2 font-display text-[9px] uppercase tracking-wider">
          <span className="inline-flex items-center gap-1 rounded border border-[var(--honey)]/40 px-2 py-1 text-[var(--honey)]"><Eye className="h-3 w-3" /> propose only</span>
          <span className="inline-flex items-center gap-1 rounded border border-border px-2 py-1 text-muted-foreground"><ShieldOff className="h-3 w-3" /> dispatch disabled</span>
          <span className="rounded border border-border px-2 py-1 text-muted-foreground">{counts.discovered} discovered</span>
          <span className="rounded border border-border px-2 py-1 text-muted-foreground">{receipts.length} finding receipts</span>
          {counts.unknown > 0 && <span className="rounded border border-[var(--crit)]/40 px-2 py-1 text-[var(--crit)]">{counts.unknown} unknown</span>}
        </div>
      </div>

      {loading ? (
        <div className="flex items-center gap-2 py-8 text-xs text-muted-foreground"><Loader2 className="h-3.5 w-3.5 animate-spin" /> Reading durable work state…</div>
      ) : error ? (
        <div className="rounded-lg border border-[var(--crit)]/40 bg-[var(--crit)]/[0.06] p-3 text-xs text-muted-foreground">{error}</div>
      ) : items.length === 0 ? (
        <div className="rounded-lg border border-dashed border-border p-8 text-center text-xs text-muted-foreground">No concrete repository work has been proposed. Mandate ticks record broad discovery plans separately until product findings identify a real repository and subject.</div>
      ) : (
        <>
          <div className="space-y-3">
            {items.slice(0, visible).map((item) => <WorkRow key={item.id} item={item} receipts={receiptsByItem.get(item.id) ?? []} />)}
          </div>
          {items.length > PAGE_SIZE && (
            <div className="mt-4 flex flex-wrap justify-center gap-2 font-display text-[9px] uppercase tracking-wider">
              {visible < items.length && <button className="rounded border border-border px-2.5 py-1.5 text-muted-foreground hover:border-[var(--honey)]/40 hover:text-[var(--honey)]" onClick={() => setVisible((count) => Math.min(count + PAGE_SIZE, items.length))}>Show more</button>}
              {visible < items.length && <button className="rounded border border-border px-2.5 py-1.5 text-muted-foreground hover:border-[var(--honey)]/40 hover:text-[var(--honey)]" onClick={() => setVisible(items.length)}>Show all</button>}
              {visible > PAGE_SIZE && <button className="rounded border border-border px-2.5 py-1.5 text-muted-foreground hover:border-[var(--honey)]/40 hover:text-[var(--honey)]" onClick={() => setVisible(PAGE_SIZE)}>Collapse</button>}
            </div>
          )}
        </>
      )}
    </section>
  );
}

function WorkRow({ item, receipts }: { item: WorkItem; receipts: FindingReceipt[] }) {
  const identity = item.proposal.identity;
  const dispatch = item.proposal.proposed_dispatch;
  const unknown = item.lifecycle.state === "unknown";
  return (
    <article className="rounded-lg border border-border/70 bg-background/40 p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="font-display text-xs font-bold text-foreground">{identity.repository} · {identity.subject_ref}</div>
          <div className="mt-1 text-[11px] text-muted-foreground">{item.proposal.rationale}</div>
        </div>
        <span className={`rounded border px-2 py-1 font-display text-[9px] uppercase tracking-wider ${unknown ? "border-[var(--crit)]/40 text-[var(--crit)]" : "border-[var(--honey)]/40 text-[var(--honey)]"}`}>{item.lifecycle.state}</span>
      </div>
      <div className="mt-3 grid gap-2 text-[11px] sm:grid-cols-2 lg:grid-cols-5">
        <Fact label="Kind" value={identity.kind} />
        <Fact label="Proposed dispatch" value={`${dispatch.product_slug} / ${dispatch.action_id}`} />
        <Fact label="Origin" value={originLabel(item.proposal.origin)} />
        <Fact label="Mandate" value={item.proposal.mandate_id ?? "not assigned"} />
        <Fact label="Finding receipts" value={receipts.length === 0 ? "none" : `${receipts.length} · latest ${receipts[0].finding.source.product_slug}`} />
      </div>
      <div className="mt-3 flex items-center gap-1.5 font-mono text-[9px] text-muted-foreground"><Fingerprint className="h-3 w-3" /> {item.fingerprint.slice(0, 20)}</div>
      <details className="mt-3">
        <summary className="cursor-pointer font-display text-[9px] uppercase tracking-wider text-muted-foreground">Proposal and lifecycle evidence</summary>
        <pre className="mt-2 max-h-64 overflow-auto rounded border border-border bg-background/60 p-2 font-mono text-[10px] text-muted-foreground">{JSON.stringify({ proposal: item.proposal, lifecycle: item.lifecycle, finding_receipts: receipts }, null, 2)}</pre>
      </details>
    </article>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div><div className="font-display text-[9px] uppercase tracking-wider text-muted-foreground">{label}</div><div className="mt-0.5 text-foreground">{value}</div></div>;
}

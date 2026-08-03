import { useMemo, useState } from "react";
import { Check, Columns2, GitCompare, Loader2, Pencil, Rows, Sparkles, X } from "lucide-react";
import { summarizeIncident } from "@/lib/ai.functions";
import type { Incident } from "@/lib/hive-extra";
import type { Product } from "@/lib/hive-data";
import { toast } from "sonner";
import { useHiveCommand } from "./hive-command";
import { wordDiff } from "@/lib/word-diff";

type DiffMode = "inline" | "split";

export function IncidentSummary({ incident, product }: { incident: Incident; product?: Product }) {
  const [loading, setLoading] = useState(false);
  const [draft, setDraft] = useState<string | null>(null);
  const [original, setOriginal] = useState<string | null>(null);
  const [accepted, setAccepted] = useState(false);
  const [showDiff, setShowDiff] = useState(false);
  const [diffMode, setDiffMode] = useState<DiffMode>("inline");
  const { logAudit } = useHiveCommand();

  const isClosed = !!incident.to;
  const isEdited = !!(original && draft && original !== draft);

  const generate = async () => {
    setLoading(true);
    setAccepted(false);
    setShowDiff(false);
    try {
      const res = await summarizeIncident({
          product_name: product?.name ?? incident.productId,
          severity: incident.severity,
          summary: incident.summary,
          opened_minutes_ago: Math.round((Date.now() - Date.parse(incident.from)) / 60000),
          closed: !!incident.to,
          resolution: incident.resolution,
      });
      setDraft(res.text);
      setOriginal(res.text);
      logAudit({
        kind: "ai",
        title: "Postmortem drafted",
        detail: `${product?.name ?? incident.productId} · ${incident.severity} · ${res.model}`,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Summary failed", { description: msg });
    } finally {
      setLoading(false);
    }
  };

  const accept = () => {
    if (!draft) return;
    setAccepted(true);
    logAudit({
      kind: "ai",
      title: "Postmortem accepted",
      detail: `${product?.name ?? incident.productId} · ${incident.severity}`,
      diff: original && original !== draft ? { before: original, after: draft } : undefined,
    });
    toast.success("Postmortem accepted", { description: "Added to this tab's session log" });
  };

  const discard = () => {
    setDraft(null);
    setOriginal(null);
    setAccepted(false);
    setShowDiff(false);
  };

  const tokens = useMemo(
    () => (original && draft && (showDiff || accepted) && isEdited ? wordDiff(original, draft) : null),
    [original, draft, showDiff, accepted, isEdited],
  );

  return (
    <div className="mt-2">
      {!draft && !loading && (
        <button
          onClick={generate}
          className="inline-flex items-center gap-1 rounded border border-[var(--honey)]/40 bg-[var(--honey)]/10 px-2 py-0.5 font-display text-[10px] uppercase tracking-wider text-[var(--honey)] transition hover:brightness-110"
        >
          <Sparkles className="h-3 w-3" /> Draft postmortem
        </button>
      )}
      {loading && (
        <div className="inline-flex items-center gap-2 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
          <Loader2 className="h-3 w-3 animate-spin text-[var(--honey)]" /> Drafting…
        </div>
      )}
      {draft && (
        <div className="mt-1 rounded border border-[var(--honey)]/30 bg-[var(--honey)]/[0.05] p-2">
          <div className="mb-1 flex items-center justify-between">
            <div className="flex items-center gap-1 font-display text-[9px] uppercase tracking-[0.2em] text-[var(--honey)]">
              <Sparkles className="h-2.5 w-2.5" />
              {accepted
                ? isEdited
                  ? "postmortem · accepted (edited)"
                  : "postmortem · accepted"
                : isClosed
                  ? showDiff
                    ? diffMode === "split"
                      ? "diff · split view"
                      : "diff · original → your edits"
                    : "postmortem draft · edit before accepting"
                  : "draft"}
            </div>
            <div className="flex items-center gap-2">
              {isEdited && (accepted || isClosed) && showDiff && (
                <button
                  onClick={() => setDiffMode((m) => (m === "inline" ? "split" : "inline"))}
                  className="inline-flex items-center gap-1 rounded border border-border/60 bg-card/40 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
                  title={diffMode === "inline" ? "Switch to side-by-side" : "Switch to inline"}
                >
                  {diffMode === "inline" ? <Columns2 className="h-2.5 w-2.5" /> : <Rows className="h-2.5 w-2.5" />}
                  {diffMode === "inline" ? "split" : "inline"}
                </button>
              )}
              {isEdited && (accepted || isClosed) && (
                <button
                  onClick={() => setShowDiff((v) => !v)}
                  className="inline-flex items-center gap-1 rounded border border-border/60 bg-card/40 px-1.5 py-0.5 font-display text-[9px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
                  title={showDiff ? "Show text" : "Show diff vs original"}
                >
                  {showDiff ? <Pencil className="h-2.5 w-2.5" /> : <GitCompare className="h-2.5 w-2.5" />}
                  {showDiff ? "text" : "diff"}
                </button>
              )}
              {accepted && (
                <span className="font-display text-[9px] uppercase tracking-wider text-[var(--ok)]">✓ session log</span>
              )}
            </div>
          </div>

          {tokens ? (
            diffMode === "split" ? (
              <div className="grid gap-2 sm:grid-cols-2">
                <div>
                  <div className="mb-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                    original
                  </div>
                  <pre className="whitespace-pre-wrap rounded border border-border/60 bg-background/40 p-2 font-mono text-[11px] leading-relaxed text-foreground/90">
                    {tokens.map((t, i) =>
                      t.type === "eq" ? (
                        <span key={i}>{t.text}</span>
                      ) : t.type === "del" ? (
                        <span
                          key={i}
                          className="rounded bg-[var(--crit)]/15 text-[var(--crit)]"
                        >
                          {t.text}
                        </span>
                      ) : null,
                    )}
                  </pre>
                </div>
                <div>
                  <div className="mb-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                    your edits
                  </div>
                  <pre className="whitespace-pre-wrap rounded border border-border/60 bg-background/40 p-2 font-mono text-[11px] leading-relaxed text-foreground/90">
                    {tokens.map((t, i) =>
                      t.type === "eq" ? (
                        <span key={i}>{t.text}</span>
                      ) : t.type === "add" ? (
                        <span
                          key={i}
                          className="rounded bg-[var(--ok)]/15 text-[var(--ok)]"
                        >
                          {t.text}
                        </span>
                      ) : null,
                    )}
                  </pre>
                </div>
              </div>
            ) : (
              <pre className="whitespace-pre-wrap rounded border border-border/60 bg-background/40 p-2 font-mono text-[11px] leading-relaxed text-foreground/90">
                {tokens.map((t, i) =>
                  t.type === "eq" ? (
                    <span key={i}>{t.text}</span>
                  ) : t.type === "add" ? (
                    <span key={i} className="rounded bg-[var(--ok)]/15 text-[var(--ok)] underline decoration-[var(--ok)]/40 underline-offset-2">
                      {t.text}
                    </span>
                  ) : (
                    <span key={i} className="rounded bg-[var(--crit)]/15 text-[var(--crit)] line-through decoration-[var(--crit)]/60">
                      {t.text}
                    </span>
                  ),
                )}
              </pre>
            )
          ) : accepted || !isClosed ? (
            <p className="whitespace-pre-wrap text-xs text-foreground/90">{draft}</p>
          ) : (
            <textarea
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              rows={Math.min(8, Math.max(3, draft.split("\n").length + 1))}
              className="w-full resize-y rounded border border-border bg-background/40 p-2 text-xs text-foreground/90 focus:border-[var(--honey)]/60 focus:outline-none"
            />
          )}

          {isClosed && !accepted && (
            <div className="mt-2 flex items-center gap-1.5">
              <button
                onClick={accept}
                className="inline-flex items-center gap-1 rounded border border-[var(--ok)]/40 bg-[var(--ok)]/10 px-2 py-0.5 font-display text-[10px] uppercase tracking-wider text-[var(--ok)] transition hover:brightness-110"
              >
                <Check className="h-3 w-3" /> Accept
              </button>
              <button
                onClick={generate}
                className="inline-flex items-center gap-1 rounded border border-border bg-card/60 px-2 py-0.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
              >
                <Sparkles className="h-3 w-3" /> Regenerate
              </button>
              <button
                onClick={discard}
                className="inline-flex items-center gap-1 rounded border border-border bg-card/60 px-2 py-0.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--crit)]/50 hover:text-[var(--crit)]"
              >
                <X className="h-3 w-3" /> Discard
              </button>
              {isEdited && (
                <span className="ml-1 font-display text-[9px] uppercase tracking-wider text-muted-foreground">
                  · edited
                </span>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

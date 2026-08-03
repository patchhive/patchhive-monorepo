import { useRef, useState } from "react";
import { Loader2, Repeat, Send, Sparkles, Square, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { useHiveCommand } from "./hive-command";
import { apiFetch } from "@/lib/http";

/**
 * The context is built by HiveCore, not here.
 *
 * The deck used to assemble the grounding and POST it with the question, which put the
 * model's evidence under the control of the least trustworthy participant — and it sent
 * the wrong thing: the per-product latency, uptime and 24h run counts it passed were
 * seeded constants from this codebase, not measurements. A model reasoning carefully
 * over invented inputs produces confident, well-argued, wrong answers.
 *
 * The browser now sends a question and nothing else.
 */

const SUGGESTIONS = [
  "Which products are currently degraded?",
  "What failed in the last hour?",
  "Who has capabilities to rotate tokens?",
  "What's the fleet MTTR trend?",
];

export function AskHive() {
  const { logAudit } = useHiveCommand();
  const [q, setQ] = useState("");
  const [loading, setLoading] = useState(false);
  const [answer, setAnswer] = useState<string | null>(null);
  const [streaming, setStreaming] = useState(false);
  const [lastQ, setLastQ] = useState<string | null>(null);
  const [stopped, setStopped] = useState(false);




  const abortRef = useRef<AbortController | null>(null);

  const stop = () => {
    if (abortRef.current) {
      abortRef.current.abort();
      abortRef.current = null;
    }
  };

  const runQuery = async (question: string) => {
    if (!question.trim() || loading || streaming) return;
    setLoading(true);
    setStopped(false);
    setLastQ(question);
    setAnswer(null);
    const controller = new AbortController();
    abortRef.current = controller;
    try {
      const res = await apiFetch("/api/products/hive-core/ask", {
        method: "POST",
        body: JSON.stringify({ question }),
        signal: controller.signal,
      });
      if (!res.ok || !res.body) {
        const txt = await res.text().catch(() => res.statusText);
        throw new Error(txt || `HTTP ${res.status}`);
      }
      setLoading(false);
      setStreaming(true);
      let acc = "";
      setAnswer(acc);
      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      // eslint-disable-next-line no-constant-condition
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        acc += decoder.decode(value, { stream: true });
        setAnswer(acc);
      }
      acc += decoder.decode();
      setAnswer(acc);
      setStreaming(false);
      logAudit({
        kind: "ai",
        title: "Asked the hive",
        detail: question,
      });
    } catch (e) {
      const aborted =
        (e instanceof DOMException && e.name === "AbortError") ||
        (e instanceof Error && e.name === "AbortError");
      if (aborted) {
        setStopped(true);
        setAnswer((prev) => (prev ? `${prev}\n\n_Stopped._` : "_Stopped._"));
        toast("Stopped", { description: "Streaming interrupted — re-run the question below" });
        logAudit({ kind: "ai", title: "Ask cancelled", detail: question });
      } else {
        const msg = e instanceof Error ? e.message : String(e);
        toast.error("AI request failed", { description: msg });
        setAnswer(`AI request failed: ${msg}`);
      }
      setStreaming(false);
    } finally {
      abortRef.current = null;
      setLoading(false);
    }
  };

  const ask = (question: string) => runQuery(question);

  const rerun = () => {
    if (!lastQ) return;
    setAnswer(null);
    runQuery(lastQ);
  };

  const clearContext = () => {
    setLastQ(null);
    setAnswer(null);
    setStopped(false);
    toast.info("Local answer cleared");
    logAudit({ kind: "ai", title: "Cleared local answer", detail: "" });
  };




  return (
    <section id="ask" className="mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur">
      <div className="mb-4 flex items-end justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 font-display text-sm font-bold uppercase tracking-[0.2em]">
            <Sparkles className="h-4 w-4 text-[var(--honey)]" /> Ask the Hive
          </h2>
          <p className="mt-1 text-xs text-muted-foreground">
            Natural-language query grounded on the live registry, runs, and incidents.
          </p>
        </div>
      </div>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          ask(q);
        }}
        className="flex gap-2"
      >
        <input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder='e.g. "which products failed a token.rotate in the last hour?"'
          className="flex-1 rounded-lg border border-border bg-background/60 px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:border-[var(--honey)]/60 focus:outline-none"
        />
        {loading || streaming ? (
          <button
            type="button"
            onClick={stop}
            className="inline-flex items-center gap-2 rounded-lg border border-[var(--crit)]/50 bg-[var(--crit)]/10 px-4 py-2 font-display text-xs font-bold uppercase tracking-wider text-[var(--crit)] transition hover:brightness-110"
          >
            <Square className="h-3.5 w-3.5 fill-current" />
            Stop
          </button>
        ) : (
          <button
            type="submit"
            disabled={!q.trim()}
            className="inline-flex items-center gap-2 rounded-lg bg-[var(--honey)] px-4 py-2 font-display text-xs font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110 disabled:opacity-50"
          >
            <Send className="h-3.5 w-3.5" />
            Ask
          </button>
        )}
      </form>

      <div className="mt-3 flex flex-wrap gap-1.5">
        {SUGGESTIONS.map((s) => (
          <button
            key={s}
            onClick={() => {
              setQ(s);
              ask(s);
            }}
            className="rounded-full border border-border bg-card/60 px-2.5 py-1 font-display text-[10px] text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
          >
            {s}
          </button>
        ))}
      </div>

      {(answer || loading) && (
        <div className="mt-4 rounded-lg border border-[var(--honey)]/30 bg-[var(--honey)]/[0.04] p-4">
          {lastQ && (
            <div className="mb-2 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
              → {lastQ}
            </div>
          )}
          {loading ? (
            <div className="flex items-center gap-2 font-display text-xs text-muted-foreground">
              <Loader2 className="h-3.5 w-3.5 animate-spin text-[var(--honey)]" /> Querying the hive…
            </div>
          ) : (
            <>
              <p className="text-sm leading-relaxed text-foreground">
                {answer}
                {streaming && <span className="ml-0.5 inline-block h-3 w-1.5 -translate-y-px animate-pulse bg-[var(--honey)] align-middle" />}
              </p>
              {stopped && !streaming && (() => {
                const canRerun = !!lastQ;
                if (!canRerun) {
                  return (
                    <div className="mt-3 font-display text-[10px] uppercase tracking-wider text-muted-foreground/70">
                      No previous question to re-run — ask a new question above.
                    </div>
                  );
                }
                return (
                  <div className="mt-3 flex flex-wrap items-center gap-2">
                    <button
                      onClick={rerun}
                      disabled={!canRerun}
                      className="inline-flex items-center gap-1 rounded border border-border bg-card/60 px-2 py-0.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--honey)]/50 hover:text-[var(--honey)] disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-border disabled:hover:text-muted-foreground"
                      title={canRerun ? "Re-run the same question from scratch" : "No previous question to re-run"}
                    >
                      <Repeat className="h-3 w-3" /> Re-run
                    </button>
                    <button
                      onClick={clearContext}
                      className="inline-flex items-center gap-1 rounded border border-border bg-card/60 px-2 py-0.5 font-display text-[10px] uppercase tracking-wider text-muted-foreground transition hover:border-[var(--crit)]/40 hover:text-[var(--crit)]"
                      title="Discard cached answer and start fresh"
                    >
                      <Trash2 className="h-3 w-3" /> Clear
                    </button>
                  </div>
                );
              })()}
            </>
          )}
        </div>
      )}
    </section>
  );
}

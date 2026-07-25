import { useState, type FormEvent } from "react";
import { Loader2, Sparkles } from "lucide-react";

import { apiFetch, HiveApiError } from "@/lib/api";
import { EmptyDeck, Panel, Section } from "./deck-ui";

/**
 * Grounded question answering over the suite's own state.
 *
 * The model call lives in the Rust backend behind PATCHHIVE_AI_URL — the browser
 * never talks to a provider and never holds a provider key. This is read-only by
 * construction: it explains what the ledger says and it cannot dispatch anything.
 */
interface AskResponse {
  answer: string;
  /** Event, run, or policy ids the answer was drawn from. */
  citations: string[];
}

const SUGGESTIONS = [
  "Why did nothing ship yesterday?",
  "Which products are drifting from their manifests?",
  "What is holding the last three PR slots?",
  "Which repositories were denied this week, and by which rule?",
];

export function AskHive() {
  const [question, setQuestion] = useState("");
  const [pending, setPending] = useState(false);
  const [answer, setAnswer] = useState<AskResponse | null>(null);
  const [error, setError] = useState("");

  async function submit(event: FormEvent) {
    event.preventDefault();
    const trimmed = question.trim();
    if (trimmed.length < 2 || pending) return;

    setPending(true);
    setError("");
    setAnswer(null);
    try {
      const result = await apiFetch<AskResponse>("/ask", {
        method: "POST",
        body: JSON.stringify({ question: trimmed }),
      });
      setAnswer(result);
    } catch (cause) {
      setError(
        cause instanceof HiveApiError
          ? cause.message
          : "HiveCore could not answer that question.",
      );
    } finally {
      setPending(false);
    }
  }

  return (
    <Section
      id="ask"
      title="Ask the hive"
      kicker="Natural-language questions answered from the suite's own event ledger. Read-only, grounded, and cited."
    >
      <form onSubmit={submit} className="flex gap-2">
        <input
          value={question}
          onChange={(event) => setQuestion(event.target.value)}
          placeholder="Ask about runs, drift, policy, or budgets…"
          className="flex-1 rounded border border-border bg-background/60 px-3 py-2 text-sm text-foreground outline-none focus:border-[var(--honey)]/50"
        />
        <button
          type="submit"
          disabled={pending || question.trim().length < 2}
          className="inline-flex items-center gap-2 rounded bg-[var(--honey)] px-4 py-2 font-display text-[10px] font-bold uppercase tracking-wider text-primary-foreground transition hover:brightness-110 disabled:opacity-40"
        >
          {pending ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Sparkles className="h-3.5 w-3.5" />}
          Ask
        </button>
      </form>

      <div className="mt-2 flex flex-wrap gap-1.5">
        {SUGGESTIONS.map((suggestion) => (
          <button
            key={suggestion}
            onClick={() => setQuestion(suggestion)}
            className="rounded border border-border px-2 py-1 text-[11px] text-muted-foreground transition hover:border-[var(--honey)]/40 hover:text-foreground"
          >
            {suggestion}
          </button>
        ))}
      </div>

      <div className="mt-4">
        {error && (
          <Panel className="border-[var(--warn)]/40 bg-[var(--warn)]/[0.06]">
            <p className="text-xs text-foreground">{error}</p>
            <p className="mt-1 text-[11px] text-muted-foreground">
              The answer endpoint is not implemented yet. It belongs in the Rust backend, calling
              the local gateway at PATCHHIVE_AI_URL — never a provider from the browser.
            </p>
          </Panel>
        )}
        {answer && (
          <Panel>
            <p className="text-sm text-foreground">{answer.answer}</p>
            {answer.citations.length > 0 && (
              <div className="mt-2 flex flex-wrap gap-1.5">
                {answer.citations.map((citation) => (
                  <code
                    key={citation}
                    className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground"
                  >
                    {citation}
                  </code>
                ))}
              </div>
            )}
          </Panel>
        )}
        {!error && !answer && !pending && (
          <EmptyDeck
            title="Grounded on the ledger, not on vibes"
            detail="Answers are drawn only from suite events, runs, policy decisions, and budgets, and cite what they used. If the ledger does not contain the answer, it says so."
            source="POST /ask"
          />
        )}
      </div>
    </Section>
  );
}

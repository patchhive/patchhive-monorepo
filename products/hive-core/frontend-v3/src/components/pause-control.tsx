import { useState } from "react";
import { Pause, Play } from "lucide-react";

import { INITIAL_PAUSE, type SuitePause } from "@/lib/suite-state";

/**
 * The brake. Anything that runs itself needs one the operator trusts absolutely, so
 * it lives in the chrome and never behind a tab. It drains in-flight work rather
 * than abandoning it, and takes effect within one conductor tick.
 */
export function PauseControl() {
  const [pause, setPause] = useState<SuitePause>(INITIAL_PAUSE);

  function toggle() {
    setPause((current) =>
      current.paused
        ? INITIAL_PAUSE
        : {
            paused: true,
            scope: "suite",
            target: null,
            since: new Date().toISOString(),
            reason: "Paused from the deck",
          },
    );
  }

  return (
    <button
      onClick={toggle}
      title={
        pause.paused
          ? "Resume suite-wide autonomous work"
          : "Pause all autonomous work; in-flight runs drain"
      }
      className={`inline-flex items-center gap-2 rounded-md border px-3 py-1.5 font-display text-[10px] font-bold uppercase tracking-wider transition ${
        pause.paused
          ? "border-[var(--crit)]/50 bg-[var(--crit)]/10 text-[var(--crit)]"
          : "border-border bg-card/60 text-muted-foreground hover:border-[var(--honey)]/50 hover:text-[var(--honey)]"
      }`}
    >
      {pause.paused ? <Play className="h-3.5 w-3.5" /> : <Pause className="h-3.5 w-3.5" />}
      {pause.paused ? "Paused" : "Pause suite"}
    </button>
  );
}

// Deck primitives. HiveCore deliberately does not use @patchhivehq/ui-v3 — it is a
// control plane, not a specialist product, and keeps its own visual language
// (AGENTS.md: HiveCore stays outside the v3 migration).

import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export const HEX = "polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)";

export function Section({
  id,
  title,
  kicker,
  actions,
  children,
  className,
}: {
  id?: string;
  title: string;
  kicker?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section
      id={id}
      className={cn(
        "mt-8 scroll-mt-24 rounded-xl border border-border bg-card/50 p-6 backdrop-blur",
        className,
      )}
    >
      <header className="mb-5 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="font-display text-sm font-bold uppercase tracking-[0.18em] text-foreground">
            {title}
          </h2>
          {kicker && <p className="mt-1 text-xs text-muted-foreground">{kicker}</p>}
        </div>
        {actions && <div className="flex items-center gap-2">{actions}</div>}
      </header>
      {children}
    </section>
  );
}

export function Panel({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("rounded-lg border border-border/70 bg-background/40 p-4", className)}>
      {children}
    </div>
  );
}

const chipTone = {
  neutral: "border-border text-muted-foreground",
  honey: "border-[var(--honey)]/40 text-[var(--honey)]",
  ok: "border-[var(--ok)]/40 text-[var(--ok)]",
  warn: "border-[var(--warn)]/40 text-[var(--warn)]",
  crit: "border-[var(--crit)]/40 text-[var(--crit)]",
} as const;

export type ChipTone = keyof typeof chipTone;

export function Chip({
  tone = "neutral",
  children,
  title,
}: {
  tone?: ChipTone;
  children: ReactNode;
  title?: string;
}) {
  return (
    <span
      title={title}
      className={cn(
        "inline-flex items-center gap-1 rounded border px-1.5 py-0.5 font-display text-[10px] uppercase tracking-wider",
        chipTone[tone],
      )}
    >
      {children}
    </span>
  );
}

export function StatusDot({ tone }: { tone: ChipTone }) {
  const color =
    tone === "ok"
      ? "var(--ok)"
      : tone === "warn"
        ? "var(--warn)"
        : tone === "crit"
          ? "var(--crit)"
          : tone === "honey"
            ? "var(--honey)"
            : "var(--muted-foreground)";
  return (
    <span
      className="inline-block h-1.5 w-1.5 rounded-full"
      style={{ background: color, boxShadow: `0 0 8px ${color}` }}
    />
  );
}

/**
 * Empty states carry the reason. "Not wired yet" and "wired but genuinely empty" are
 * different facts and the deck must never let them look the same.
 */
export function EmptyDeck({
  title,
  detail,
  source,
}: {
  title: string;
  detail: string;
  source?: string;
}) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border/70 px-6 py-10 text-center">
      <div
        className="h-6 w-6 border border-[var(--honey)]/40 bg-[var(--honey)]/10"
        style={{ clipPath: HEX }}
      />
      <div className="font-display text-xs font-bold uppercase tracking-wider text-foreground">
        {title}
      </div>
      <p className="max-w-md text-xs text-muted-foreground">{detail}</p>
      {source && (
        <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
          {source}
        </code>
      )}
    </div>
  );
}

export function Metric({
  label,
  value,
  tone = "neutral",
  detail,
}: {
  label: string;
  value: string | number;
  tone?: ChipTone;
  detail?: string;
}) {
  const color =
    tone === "ok"
      ? "text-[var(--ok)]"
      : tone === "warn"
        ? "text-[var(--warn)]"
        : tone === "crit"
          ? "text-[var(--crit)]"
          : tone === "honey"
            ? "text-[var(--honey)]"
            : "text-foreground";
  return (
    <div className="rounded-lg border border-border bg-card/60 px-4 py-3">
      <div className="font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
        {label}
      </div>
      <div className={cn("mt-1 font-display text-2xl font-bold leading-none", color)}>{value}</div>
      {detail && <div className="mt-1 text-[11px] text-muted-foreground">{detail}</div>}
    </div>
  );
}

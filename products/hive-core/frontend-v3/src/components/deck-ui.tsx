// Deck primitives. HiveCore deliberately does not use @patchhivehq/ui-v3 — it is a
// control plane, not a specialist product, and keeps its own visual language
// (AGENTS.md: HiveCore stays outside the v3 migration).
//
// The honey/amber treatment is load-bearing, not decoration: glass surfaces over the
// hex-grid field, ambient glow bleeding off the edges, and a scanline wash are what
// make this read as an operations deck rather than an admin table.

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
  delay = 0,
}: {
  id?: string;
  title: string;
  kicker?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  delay?: number;
}) {
  return (
    <section
      id={id}
      style={{ animationDelay: `${delay}ms` }}
      className={cn(
        "animate-float-up relative mt-8 scroll-mt-24 overflow-hidden rounded-xl border border-border",
        "bg-card/50 p-6 shadow-[0_1px_0_0_rgba(255,255,255,0.03)_inset] backdrop-blur",
        className,
      )}
    >
      {/* ambient honey bleed, top-right */}
      <div
        aria-hidden="true"
        className="pointer-events-none absolute -right-32 -top-32 h-64 w-64 rounded-full bg-[var(--honey)]/[0.07] blur-3xl"
      />
      <header className="relative mb-5 flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <span
              aria-hidden="true"
              className="h-3 w-3 border border-[var(--honey)]/50 bg-[var(--honey)]/20"
              style={{ clipPath: HEX }}
            />
            <h2 className="font-display text-sm font-bold uppercase tracking-[0.18em] text-foreground">
              {title}
            </h2>
          </div>
          {kicker && <p className="mt-1.5 text-xs text-muted-foreground">{kicker}</p>}
        </div>
        {actions && <div className="flex items-center gap-2">{actions}</div>}
      </header>
      <div className="relative">{children}</div>
    </section>
  );
}

export function Panel({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div
      className={cn(
        "rounded-lg border border-border/70 bg-background/40 p-4 backdrop-blur-sm",
        className,
      )}
    >
      {children}
    </div>
  );
}

const chipTone = {
  neutral: "border-border text-muted-foreground",
  honey: "border-[var(--honey)]/40 text-[var(--honey)] bg-[var(--honey)]/[0.06]",
  ok: "border-[var(--ok)]/40 text-[var(--ok)] bg-[var(--ok)]/[0.06]",
  warn: "border-[var(--warn)]/40 text-[var(--warn)] bg-[var(--warn)]/[0.06]",
  crit: "border-[var(--crit)]/40 text-[var(--crit)] bg-[var(--crit)]/[0.06]",
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

export function toneColor(tone: ChipTone): string {
  switch (tone) {
    case "ok":
      return "var(--ok)";
    case "warn":
      return "var(--warn)";
    case "crit":
      return "var(--crit)";
    case "honey":
      return "var(--honey)";
    default:
      return "var(--muted-foreground)";
  }
}

export function StatusDot({ tone, pulse = false }: { tone: ChipTone; pulse?: boolean }) {
  const color = toneColor(tone);
  return (
    <span
      className={cn("inline-block h-1.5 w-1.5 rounded-full", pulse && "animate-pulse-dot")}
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
    <div className="relative flex flex-col items-center justify-center gap-2 overflow-hidden rounded-lg border border-dashed border-border/70 bg-background/30 px-6 py-12 text-center">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 scanline opacity-30"
      />
      <div className="relative grid place-items-center">
        <div
          className="h-9 w-9 border border-[var(--honey)]/40 bg-[var(--honey)]/[0.08]"
          style={{ clipPath: HEX }}
        />
        <div
          className="absolute h-3.5 w-3.5 bg-[var(--honey)]/30"
          style={{ clipPath: HEX }}
        />
      </div>
      <div className="relative mt-1 font-display text-xs font-bold uppercase tracking-wider text-foreground">
        {title}
      </div>
      <p className="relative max-w-md text-xs leading-relaxed text-muted-foreground">{detail}</p>
      {source && (
        <code className="relative rounded border border-border/60 bg-background/60 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
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
  delay = 0,
}: {
  label: string;
  value: string | number;
  tone?: ChipTone;
  detail?: string;
  delay?: number;
}) {
  const color = toneColor(tone);
  return (
    <div
      style={{ animationDelay: `${delay}ms` }}
      className="animate-float-up relative overflow-hidden rounded-lg border border-border bg-card/60 px-4 py-3 backdrop-blur"
    >
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-0 top-0 h-px"
        style={{ background: `linear-gradient(90deg, transparent, ${color}, transparent)`, opacity: 0.5 }}
      />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute -right-10 -top-10 h-24 w-24 rounded-full blur-2xl"
        style={{ background: color, opacity: 0.08 }}
      />
      <div className="relative font-display text-[10px] uppercase tracking-[0.2em] text-muted-foreground">
        {label}
      </div>
      <div
        className="relative mt-1 font-display text-2xl font-bold leading-none"
        style={{ color: tone === "neutral" ? "var(--foreground)" : color }}
      >
        {value}
      </div>
      {detail && <div className="relative mt-1 text-[11px] text-muted-foreground">{detail}</div>}
    </div>
  );
}

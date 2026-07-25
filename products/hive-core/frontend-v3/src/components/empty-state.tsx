import type { ReactNode } from "react";

interface Props {
  title: string;
  body?: string;
  action?: ReactNode;
}

export function EmptyState({ title, body, action }: Props) {
  return (
    <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border/70 bg-background/30 p-8 text-center">
      <svg width="72" height="72" viewBox="0 0 72 72" className="mb-3 opacity-70">
        <defs>
          <linearGradient id="hex-empty" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="var(--honey)" stopOpacity="0.55" />
            <stop offset="100%" stopColor="var(--amber-deep)" stopOpacity="0.15" />
          </linearGradient>
        </defs>
        <g fill="none" stroke="url(#hex-empty)" strokeWidth="1.4">
          <path d="M36 4 L64 20 L64 52 L36 68 L8 52 L8 20 Z" />
          <path d="M36 20 L52 28 L52 44 L36 52 L20 44 L20 28 Z" opacity="0.6" />
          <path d="M36 32 L42 35 L42 41 L36 44 L30 41 L30 35 Z" opacity="0.9" fill="var(--honey)" fillOpacity="0.25" />
        </g>
      </svg>
      <div className="font-display text-sm font-bold uppercase tracking-wider text-foreground">{title}</div>
      {body && <p className="mt-1 max-w-xs text-xs text-muted-foreground">{body}</p>}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}

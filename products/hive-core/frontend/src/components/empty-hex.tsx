import { type ReactNode } from "react";

/** Hex-tessellation empty state art. */
export function EmptyHex({
  title,
  hint,
  action,
}: {
  title: string;
  hint?: string;
  action?: ReactNode;
}) {
  const HEX = "polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)";
  const rows = [3, 4, 3];
  return (
    <div className="flex flex-col items-center justify-center gap-4 px-6 py-8 text-center">
      <div className="flex flex-col items-center gap-0 opacity-70">
        {rows.map((n, r) => (
          <div key={r} className={`flex gap-1 ${r < rows.length - 1 ? "-mb-3" : ""}`}>
            {Array.from({ length: n }).map((_, i) => {
              const isCenter = r === 1 && i === Math.floor(n / 2);
              return (
                <div
                  key={i}
                  className={`h-8 w-7 ${
                    isCenter
                      ? "bg-[var(--honey)]/25 shadow-[0_0_10px_var(--honey)]"
                      : "bg-muted/40"
                  }`}
                  style={{
                    clipPath: HEX,
                    animation: isCenter ? "pulse 2.4s ease-in-out infinite" : undefined,
                  }}
                />
              );
            })}
          </div>
        ))}
      </div>
      <div>
        <div className="font-display text-xs font-bold uppercase tracking-[0.2em] text-foreground/80">
          {title}
        </div>
        {hint && (
          <div className="mt-1 font-display text-[10px] uppercase tracking-wider text-muted-foreground">
            {hint}
          </div>
        )}
      </div>
      {action}
    </div>
  );
}

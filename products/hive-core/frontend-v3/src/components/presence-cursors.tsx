import { useEffect, useState } from "react";
import { PRESENCE } from "@/lib/hive-metrics";
import { useHiveCommand } from "./hive-command";

interface Pos {
  x: number;
  y: number;
  tx: number;
  ty: number;
}

export function PresenceCursors() {
  const { presenceOn } = useHiveCommand();
  const [positions, setPositions] = useState<Pos[]>(() =>
    PRESENCE.map((_, i) => ({
      x: 200 + i * 240,
      y: 300 + i * 120,
      tx: 200 + i * 240,
      ty: 300 + i * 120,
    })),
  );

  useEffect(() => {
    if (!presenceOn) return;
    const retarget = () => {
      const w = window.innerWidth;
      const h = window.innerHeight;
      setPositions((prev) =>
        prev.map((p) => ({
          ...p,
          tx: Math.max(80, Math.random() * (w - 120)),
          ty: Math.max(120, Math.random() * (h - 200)),
        })),
      );
    };
    retarget();
    const retargetTimer = window.setInterval(retarget, 4200);
    const tick = window.setInterval(() => {
      setPositions((prev) =>
        prev.map((p) => ({
          ...p,
          x: p.x + (p.tx - p.x) * 0.06,
          y: p.y + (p.ty - p.y) * 0.06,
        })),
      );
    }, 40);
    return () => {
      window.clearInterval(retargetTimer);
      window.clearInterval(tick);
    };
  }, [presenceOn]);

  if (!presenceOn) return null;
  return (
    <div className="pointer-events-none fixed inset-0 z-40">
      {PRESENCE.map((op, i) => (
        <div
          key={op.id}
          className="absolute -translate-x-1 -translate-y-1 transition-transform"
          style={{ transform: `translate(${positions[i].x}px, ${positions[i].y}px)` }}
        >
          <svg width="18" height="18" viewBox="0 0 18 18">
            <path d="M2 2 L14 8 L8 10 L6 15 Z" fill={op.color} stroke="black" strokeWidth="0.6" />
          </svg>
          <span
            className="ml-3 mt-0.5 inline-block rounded-full px-2 py-0.5 font-display text-[9px] font-bold uppercase tracking-wider"
            style={{ background: op.color, color: "black" }}
          >
            {op.name}
          </span>
        </div>
      ))}
    </div>
  );
}

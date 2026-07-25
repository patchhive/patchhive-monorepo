// Deterministic derived metrics — sparklines, heatmap, capability aggregates, anomaly z-scores.
import { PRODUCTS, RUNS, type Product, type RunEvent } from "./hive-data";


/** 60 samples of latency history per product (oldest → newest). */
/**
 * Deterministic pseudo-random source for the sampled sparklines and heatmap.
 * Lives here because everything it feeds is explicitly sampled, not measured.
 */
function seededRand(seed: string) {
  let h = 2166136261;
  for (let i = 0; i < seed.length; i++) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return () => {
    h ^= h << 13;
    h ^= h >>> 17;
    h ^= h << 5;
    return ((h >>> 0) % 10000) / 10000;
  };
}

export function latencyHistory(product: Product, n = 60): number[] {
  const rand = seededRand(product.id + "_hist");
  const base = product.latencyMs || 40;
  const jitter = product.status === "warn" ? 0.55 : product.status === "crit" ? 1.2 : 0.25;
  const out: number[] = [];
  let v = base;
  for (let i = 0; i < n; i++) {
    const drift = (rand() - 0.5) * base * jitter;
    v = Math.max(2, base + drift + Math.sin(i / 6 + rand() * 3) * base * 0.15);
    out.push(Math.round(v));
  }
  return out;
}

function percentile(sorted: number[], p: number) {
  if (sorted.length === 0) return 0;
  const idx = Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length));
  return sorted[idx];
}
export function latencyStats(product: Product) {
  const h = latencyHistory(product);
  const sorted = [...h].sort((a, b) => a - b);
  const mean = h.reduce((a, b) => a + b, 0) / h.length;
  const variance = h.reduce((a, b) => a + (b - mean) ** 2, 0) / h.length;
  return {
    samples: h,
    p50: percentile(sorted, 50),
    p95: percentile(sorted, 95),
    p99: percentile(sorted, 99),
    mean,
    stdev: Math.sqrt(variance),
  };
}

/** 24x7 heatmap of run volume per product. Rows = day (Sun..Sat), cols = hour. */
export function runHeatmap(productId?: string): number[][] {
  const seed = productId ?? "all";
  const rand = seededRand("heatmap_" + seed);
  const grid: number[][] = [];
  for (let d = 0; d < 7; d++) {
    const row: number[] = [];
    for (let h = 0; h < 24; h++) {
      // workday bias: mon-fri, 9-19
      const workday = d >= 1 && d <= 5;
      const worktime = h >= 9 && h <= 19;
      const base = workday ? (worktime ? 60 : 15) : worktime ? 25 : 6;
      row.push(Math.max(0, Math.round(base + (rand() - 0.35) * base)));
    }
    grid.push(row);
  }
  return grid;
}


/** Baseline mean/std per capability for anomaly scoring. */
const baselineCache = new Map<string, { mean: number; stdev: number }>();
export function capabilityBaseline(capability: string) {
  const hit = baselineCache.get(capability);
  if (hit) return hit;
  const durs: number[] = [];
  for (const p of PRODUCTS.filter((p) => p.capabilities.includes(capability))) {
    const stats = latencyStats(p);
    durs.push(...stats.samples);
  }
  if (durs.length === 0) return { mean: 100, stdev: 40 };
  const mean = durs.reduce((a, b) => a + b, 0) / durs.length;
  const stdev = Math.sqrt(durs.reduce((a, b) => a + (b - mean) ** 2, 0) / durs.length) || 1;
  const v = { mean, stdev };
  baselineCache.set(capability, v);
  return v;
}
export function runAnomalyZ(run: RunEvent): number {
  if (run.status !== "success" || run.durationMs === 0) return 0;
  const { mean, stdev } = capabilityBaseline(run.capability);
  return (run.durationMs - mean) / stdev;
}

/** Presence — mock operators. */
export const PRESENCE = [
  { id: "op1", name: "kira", color: "var(--honey)" },
  { id: "op2", name: "nolan", color: "var(--ok)" },
  { id: "op3", name: "wren", color: "#7dd3fc" },
];

export { RUNS };

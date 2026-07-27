// Derived metrics, all of them from observations rather than generators.
//
// This module used to be built around `seededRand`: a deterministic pseudo-random
// source feeding the latency sparklines, the run-volume heatmap, and — least
// obviously — the baseline that anomaly z-scores were measured against. The heatmap
// even carried a workday bias so it would look like real traffic. A real run duration
// was being scored as "anomalous" against a distribution nobody had observed.
//
// The generator is gone. Latency comes from HiveCore's health probes (see probes.ts),
// run volume and anomaly baselines come from the run feed, and anything that cannot
// be derived from an observation is absent rather than modelled.

import { RUNS, type RunEvent } from "./hive-data";

/** Rows = day of week (Sun..Sat), cols = hour of day. */
export interface Heatmap {
  grid: number[][];
  /** Runs that carried a usable timestamp and were counted. */
  counted: number;
  /** Runs in the feed that could not be placed — no usable start time. */
  undated: number;
  /** Oldest run counted, so the window is stated rather than implied. */
  earliest: string | null;
}

/**
 * Run volume by day and hour, from the runs the suite actually reports.
 *
 * Sparse by nature: the feed carries each product's recent runs, not full history, so
 * most cells are legitimately zero. That is the honest shape — the previous version
 * filled all 168 cells with plausible-looking noise, which read as "the suite is busy"
 * regardless of whether anything had ever run.
 *
 * `counted` and `earliest` come back so the panel can state its window. A heatmap
 * without one invites the reader to assume it is complete.
 */
export function runHeatmap(productName?: string): Heatmap {
  const grid: number[][] = Array.from({ length: 7 }, () => Array.from({ length: 24 }, () => 0));
  let counted = 0;
  let undated = 0;
  let earliest: number | null = null;

  for (const run of RUNS) {
    if (productName && run.product !== productName) continue;
    if (!run.startedAt) {
      // Counted separately rather than dropped silently, so a feed carrying no
      // timestamps reads as "nothing placeable" instead of "nothing happened".
      undated += 1;
      continue;
    }
    const at = new Date(run.startedAt);
    const ms = at.getTime();
    if (Number.isNaN(ms)) {
      undated += 1;
      continue;
    }
    grid[at.getDay()][at.getHours()] += 1;
    counted += 1;
    if (earliest === null || ms < earliest) earliest = ms;
  }

  return {
    grid,
    counted,
    undated,
    earliest: earliest === null ? null : new Date(earliest).toISOString(),
  };
}

/**
 * Mean and spread of observed durations for one capability.
 *
 * Built from completed runs of that capability and nothing else. It used to be built
 * from the fake latency sparklines, which meant a real run was flagged anomalous by
 * comparison to numbers that were never measured.
 *
 * Returns null when there is too little to say. A handful of samples does not
 * establish a distribution, and a z-score computed from them is arithmetic dressed as
 * evidence.
 */
const MIN_BASELINE_SAMPLES = 5;

export function capabilityBaseline(
  capability: string,
): { mean: number; stdev: number; samples: number } | null {
  const durations = RUNS.filter(
    (run) => run.capability === capability && run.status === "success" && run.durationMs > 0,
  ).map((run) => run.durationMs);

  if (durations.length < MIN_BASELINE_SAMPLES) return null;

  const mean = durations.reduce((total, value) => total + value, 0) / durations.length;
  const variance =
    durations.reduce((total, value) => total + (value - mean) ** 2, 0) / durations.length;
  const stdev = Math.sqrt(variance);
  // A capability whose runs are all identical has no spread; a z-score against it is
  // infinite or zero depending on rounding. Report no baseline instead.
  if (stdev <= 0) return null;

  return { mean, stdev, samples: durations.length };
}

/**
 * How unusual a run's duration is, or null when there is no basis to judge.
 *
 * Null is a real answer and callers must render it as one. The previous version always
 * produced a number, because its baseline was always available — it was invented.
 */
export function runAnomalyZ(run: RunEvent): number | null {
  if (run.status !== "success" || run.durationMs === 0) return null;
  const baseline = capabilityBaseline(run.capability);
  if (!baseline) return null;
  return (run.durationMs - baseline.mean) / baseline.stdev;
}

export { RUNS };

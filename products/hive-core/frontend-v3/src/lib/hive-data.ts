export type Status = "ok" | "warn" | "crit" | "offline" | "unknown";

export interface Product {
  id: string;
  name: string;
  tagline: string;
  frontend: string;
  api: string;
  status: Status;
  latencyMs: number | null;
  uptime: number | null; // 0-1 when observed
  probeState: "observed" | "failed" | "not_observed" | "not_applicable";
  probeReason: string;
  runs24h: number | null;
  /** Descriptive manifest tags; never dispatchable action ids. */
  declaredCapabilities: string[];
  /** Dispatchable action ids from an observed runtime capability response. */
  capabilities: string[];
  capabilityState: "observed" | "failed" | "not_observed" | "not_applicable";
  capabilityReason: string;
  /** Null when conformance evidence could not be observed. */
  contractDrift: number | null;
}

// Populated from the canonical registry and HiveCore's durable runtime snapshot.
// Keeping this mutable array preserves the deck's existing data flow without
// maintaining a second product inventory in browser code.
export const PRODUCTS: Product[] = [];


export interface RunEvent {
  id: string;
  product: string;
  capability: string;
  status: RunLifecycleStatus;
  /** Null when the product did not report a complete time interval. */
  durationMs: number | null;
  /** Human-relative age, rendered directly. */
  ts: string;
  /** ISO start, for windowing and sorting. Empty on seed rows. */
  startedAt: string;
}

export type RunLifecycleStatus =
  | "standby"
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "held"
  | "skipped"
  | "unknown";

export const RUNS: RunEvent[] = [];

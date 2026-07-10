import { useSyncExternalStore } from "react";

export type Severity = "critical" | "high" | "medium" | "low";
export type Rec = "fix now" | "plan next" | "watch";
export type Status = "open" | "fixed" | "snoozed";

export type ActivityEvent = {
  id: string;
  at: string;
  actor: string;
  kind: "created" | "status" | "priority" | "assigned" | "note";
  message: string;
};

export type Finding = {
  id: string;
  title: string;
  loc: string;
  pkg: string;
  sev: Severity;
  src: string;
  score: number;
  rec: Rec;
  age: string;
  status: Status;
  owner: string | null;
  detectedAt: string;
  updatedAt: string;
  description: string;
  remediation: string;
  references: { label: string; href: string }[];
  activity: ActivityEvent[];
  snoozedUntil?: string | null;
};

export const OWNERS = [
  "unassigned",
  "avery.k",
  "priya.r",
  "sam.d",
  "morgan.t",
  "jules.b",
] as const;

const now = new Date();
const iso = (daysAgo: number, hoursAgo = 0) =>
  new Date(now.getTime() - daysAgo * 86400000 - hoursAgo * 3600000).toISOString();

const seed: Finding[] = [
  {
    id: "CVE-2024-42918",
    title: "Prototype pollution in lodash.merge",
    loc: "services/api › package-lock.json",
    pkg: "lodash@4.17.20",
    sev: "critical",
    src: "Dependabot",
    score: 9.8,
    rec: "fix now",
    age: "3d",
    status: "open",
    owner: "avery.k",
    detectedAt: iso(3),
    updatedAt: iso(0, 2),
    description:
      "A prototype pollution vulnerability in lodash.merge allows attackers to inject properties on Object.prototype via crafted JSON payloads, potentially leading to remote code execution in downstream code paths.",
    remediation:
      "Upgrade lodash to 4.17.21 or later. Audit merge() call sites for user-controlled input and adopt Object.create(null) for accumulators.",
    references: [
      { label: "NVD entry", href: "https://nvd.nist.gov/vuln/detail/CVE-2024-42918" },
      { label: "GHSA advisory", href: "https://github.com/advisories" },
    ],
    activity: [
      { id: "a1", at: iso(3), actor: "dependabot", kind: "created", message: "Finding opened from Dependabot alert #482" },
      { id: "a2", at: iso(2, 4), actor: "avery.k", kind: "assigned", message: "Assigned to avery.k" },
      { id: "a3", at: iso(0, 2), actor: "avery.k", kind: "note", message: "Confirmed reachable from /v1/import endpoint" },
    ],
  },
  {
    id: "GHSA-8x7p-6r5q",
    title: "Unbounded regex in parseCookie — ReDoS",
    loc: "packages/edge › src/cookie.ts:47",
    pkg: "cookie@0.5.0",
    sev: "high",
    src: "Code scan",
    score: 8.4,
    rec: "fix now",
    age: "1d",
    status: "open",
    owner: "priya.r",
    detectedAt: iso(1),
    updatedAt: iso(0, 5),
    description:
      "The parseCookie regex exhibits catastrophic backtracking on adversarial inputs, allowing a single request to consume CPU on the edge worker.",
    remediation: "Replace regex with a linear tokenizer, or upgrade cookie to 0.6.0 which caps expansion.",
    references: [{ label: "GHSA", href: "https://github.com/advisories" }],
    activity: [
      { id: "b1", at: iso(1), actor: "codeql", kind: "created", message: "Detected by code scanning" },
      { id: "b2", at: iso(0, 12), actor: "priya.r", kind: "priority", message: "Priority raised to fix now" },
    ],
  },
  {
    id: "CVE-2024-31989",
    title: "SSRF via unsanitized redirect URL",
    loc: "apps/web › src/routes/auth.callback.ts",
    pkg: "app-code",
    sev: "high",
    src: "Code scan",
    score: 7.9,
    rec: "plan next",
    age: "5d",
    status: "open",
    owner: null,
    detectedAt: iso(5),
    updatedAt: iso(5),
    description:
      "The auth callback follows the `next` query parameter without allowlisting hostnames, allowing an attacker to trigger internal HTTP requests through the worker.",
    remediation: "Allowlist redirect hosts and reject absolute URLs pointing outside app domains.",
    references: [],
    activity: [
      { id: "c1", at: iso(5), actor: "codeql", kind: "created", message: "Detected by code scanning" },
    ],
  },
  {
    id: "GHSA-r7q9-w8mp",
    title: "Path traversal in tar extraction",
    loc: "workers/ingest › package.json",
    pkg: "tar@6.1.11",
    sev: "medium",
    src: "Dependabot",
    score: 6.3,
    rec: "plan next",
    age: "9d",
    status: "open",
    owner: "sam.d",
    detectedAt: iso(9),
    updatedAt: iso(6),
    description: "Archive extraction can write outside the target directory when entries contain `..` segments.",
    remediation: "Upgrade tar to 6.2.1 and enable the `strict` option.",
    references: [],
    activity: [
      { id: "d1", at: iso(9), actor: "dependabot", kind: "created", message: "Finding opened from Dependabot alert #471" },
      { id: "d2", at: iso(6), actor: "sam.d", kind: "assigned", message: "Assigned to sam.d" },
    ],
  },
  {
    id: "CWE-79",
    title: "Reflected XSS in error boundary",
    loc: "apps/web › components/error-panel.tsx:112",
    pkg: "app-code",
    sev: "medium",
    src: "Code scan",
    score: 5.1,
    rec: "watch",
    age: "12d",
    status: "open",
    owner: null,
    detectedAt: iso(12),
    updatedAt: iso(12),
    description: "Error boundary renders the raw error message including query parameters, enabling reflected XSS.",
    remediation: "Escape or strip HTML from error messages before rendering.",
    references: [],
    activity: [
      { id: "e1", at: iso(12), actor: "codeql", kind: "created", message: "Detected by code scanning" },
    ],
  },
  {
    id: "CVE-2023-52428",
    title: "DoS via crafted JWT header",
    loc: "services/auth › package-lock.json",
    pkg: "jose@4.14.0",
    sev: "low",
    src: "Dependabot",
    score: 3.7,
    rec: "watch",
    age: "21d",
    status: "open",
    owner: "morgan.t",
    detectedAt: iso(21),
    updatedAt: iso(14),
    description: "A crafted PBES2 header can cause excessive CPU during key derivation.",
    remediation: "Upgrade jose to 4.15.5 or later.",
    references: [],
    activity: [
      { id: "f1", at: iso(21), actor: "dependabot", kind: "created", message: "Finding opened from Dependabot alert #401" },
      { id: "f2", at: iso(14), actor: "morgan.t", kind: "assigned", message: "Assigned to morgan.t" },
    ],
  },
];

let state: Finding[] = seed;
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
}

function getSnapshot() {
  return state;
}

function pushEvent(f: Finding, kind: ActivityEvent["kind"], message: string): Finding {
  const event: ActivityEvent = {
    id: `${f.id}-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
    at: new Date().toISOString(),
    actor: "you",
    kind,
    message,
  };
  return { ...f, activity: [event, ...f.activity], updatedAt: event.at };
}

function update(id: string, mut: (f: Finding) => Finding) {
  state = state.map((f) => (f.id === id ? mut(f) : f));
  emit();
}

function updateMany(ids: string[], mut: (f: Finding) => Finding) {
  const set = new Set(ids);
  state = state.map((f) => (set.has(f.id) ? mut(f) : f));
  emit();
}

export const findingsStore = {
  setStatus(id: string, status: Status) {
    update(id, (f) => pushEvent({ ...f, status }, "status", `Status set to ${status}`));
  },
  setStatusMany(ids: string[], status: Status) {
    updateMany(ids, (f) => pushEvent({ ...f, status }, "status", `Status set to ${status} (bulk)`));
  },
  setPriority(id: string, rec: Rec) {
    update(id, (f) => pushEvent({ ...f, rec }, "priority", `Priority set to ${rec}`));
  },
  setPriorityMany(ids: string[], rec: Rec) {
    updateMany(ids, (f) => pushEvent({ ...f, rec }, "priority", `Priority set to ${rec} (bulk)`));
  },
  setOwner(id: string, owner: string) {
    update(id, (f) =>
      pushEvent({ ...f, owner: owner === "unassigned" ? null : owner }, "assigned", `Assigned to ${owner}`),
    );
  },
  snooze(id: string, days = 7) {
    update(id, (f) => {
      const until = new Date(Date.now() + days * 86400000).toISOString();
      return pushEvent({ ...f, status: "snoozed", snoozedUntil: until }, "status", `Snoozed for ${days}d`);
    });
  },
  addNote(id: string, note: string) {
    update(id, (f) => pushEvent(f, "note", note));
  },
};

export function useFindings(): Finding[] {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

export function useFinding(id: string): Finding | undefined {
  const all = useFindings();
  return all.find((f) => f.id === id);
}
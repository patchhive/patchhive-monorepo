import { V3_TEXT } from "@patchhivehq/ui-v3";

export const DEFAULT_PARAMS = {
  target_repo: "",
  language: "python",
  min_stars: "50",
  max_repos: "10",
  max_issues: "10",
  min_fixability_score: "60",
  concurrency: "3",
  search_query: "",
  cost_budget_usd: "0.50",
  retry_count: "3",
  labels: "bug",
};

export const DEFAULT_DRY_PARAMS = { ...DEFAULT_PARAMS, max_repos: "5", concurrency: "1" };

export const CHIP_TONES = {
  hot: "border-red-900/30 bg-red-900/10 text-red-800 dark:border-red-400/25 dark:bg-red-500/10 dark:text-red-300",
  warn: "border-amber-900/30 bg-amber-900/10 text-amber-800 dark:border-amber-400/25 dark:bg-amber-500/10 dark:text-amber-300",
  ok: "border-emerald-900/30 bg-emerald-900/10 text-emerald-800 dark:border-emerald-400/25 dark:bg-emerald-500/10 dark:text-emerald-300",
  neutral: "border-stone-800/20 bg-stone-800/5 text-stone-700 dark:border-stone-400/20 dark:bg-stone-400/5 dark:text-stone-300",
};

export function Chip({ children, tone = "neutral" }) {
  return <span className={`inline-flex items-center justify-center rounded-full border px-2.5 py-1 text-[10px] leading-none uppercase tracking-wider ${CHIP_TONES[tone] || CHIP_TONES.neutral}`}>{children}</span>;
}

export function Fact({ label, value }) {
  return <div className="surface-inset rounded-xl p-3"><div className={`text-[9px] uppercase tracking-[0.18em] ${V3_TEXT.mute}`}>{label}</div><div className={`mt-1 break-words font-display text-[18px] font-semibold tabular-nums ${V3_TEXT.strong}`}>{value ?? "—"}</div></div>;
}

export function splitList(value) {
  return String(value || "").split(/[\n,]+/).map((part) => part.trim()).filter(Boolean);
}

export function serializeRunParams(params, targetSelectionMode) {
  return {
    target_repo: targetSelectionMode === "direct" ? params.target_repo.trim() : "",
    target_selection_mode: targetSelectionMode,
    language: params.language.trim() || "python",
    min_stars: Number(params.min_stars) || 50,
    max_repos: Number(params.max_repos) || 10,
    max_issues: Number(params.max_issues) || 10,
    min_fixability_score: Math.min(100, Math.max(0, Number(params.min_fixability_score) || 0)),
    concurrency: Number(params.concurrency) || 1,
    search_query: params.search_query.trim(),
    cost_budget_usd: Number(params.cost_budget_usd) || 0,
    retry_count: Number(params.retry_count) || 3,
    labels: splitList(params.labels || "bug"),
  };
}

export function createStreamState() {
  return { agentStatuses: {}, done: null, issues: [], logs: [], phase: "idle", reattached: false, report: null, repos: [], runCost: 0, running: false };
}

// Saved run events replace live agent evidence when the browser was not
// attached to the operation that produced them.
export function runEventsToLogs(events) {
  return (Array.isArray(events) ? events : []).map((event) => ({
    agent: event.actor || "RepoReaper",
    msg: event.message || event.artifact?.label || "Saved run event",
    ts: event.created_at,
    type: event.level || "info",
  }));
}

const STATUS_LABELS = {
  assigned: "Fix in progress",
  cancelled: "Cancelled",
  done: "Complete",
  eligible: "Eligible",
  error: "Stopped with an error",
  failed: "Failed",
  fixed: "Fixed",
  held: "Held for review",
  idle: "Idle",
  no_candidates: "No matching issues",
  open: "Open",
  partial: "Completed with holds",
  passed: "Passed",
  queued: "Queued",
  rejected: "Rejected",
  running: "In progress",
  skipped: "Skipped",
  starting: "Starting",
  success: "Succeeded",
  working: "In progress",
};

const PHASE_LABELS = {
  cleanup: "Cleaning up",
  deliver: "Preparing delivery",
  discover: "Finding candidate work",
  fix: "Building fixes",
  idle: "Ready",
  judge: "Selecting relevant files",
  pr: "Opening pull requests",
  review: "Reviewing patches",
  score: "Scoring candidates",
  smith: "Reviewing patches",
  test: "Validating changes",
  validate: "Validating changes",
};

function titleCase(value) {
  return String(value || "")
    .replace(/[_-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

// Engine skip/hold reasons are stable machine tokens. `eligibility_reason` and
// `error_msg` already carry prose and must not be run through this.
const REASON_LABELS = {
  blocked: "Blocked by policy",
  cancelled: "Cancelled before completion",
  duplicate: "Skipped as a duplicate of another candidate",
  existing_pr: "Skipped because a pull request already exists for this issue",
  linked_pr_check_failed: "Skipped because the existing pull request check could not run",
  no_changes: "The generated patch changed no files",
  no_patch: "No patch was generated",
  patch_error: "Patch generation failed",
  policy: "Blocked by repository policy",
  suite_policy_unavailable: "Skipped because the HiveCore policy service could not be reached",
  watch_mode_disabled: "Ignored because watch mode is disabled",
};

export function reasonLabel(reason) {
  const value = String(reason || "").trim();
  if (!value) return "";
  if (/\s/.test(value)) return value;
  const confidence = value.match(/^confidence_(\d+)$/);
  if (confidence) return `Held because Smith review confidence was ${confidence[1]}%, below the configured minimum`;
  return REASON_LABELS[value.toLowerCase()] || titleCase(value);
}

export function statusLabel(status) {
  const value = String(status || "idle").toLowerCase();
  return STATUS_LABELS[value] || titleCase(value) || "Unknown";
}

// Agent roles are their own vocabulary. Routing them through statusLabel works
// only for as long as no role name collides with a run status.
const ROLE_LABELS = {
  gatekeeper: "Gatekeeper",
  judge: "Judge",
  reaper: "Reaper",
  scout: "Scout",
  smith: "Smith",
};

export function roleLabel(role) {
  const value = String(role || "").trim().toLowerCase();
  if (!value) return "";
  return ROLE_LABELS[value] || titleCase(value);
}

export function phaseLabel(phase) {
  const value = String(phase || "idle").toLowerCase();
  return PHASE_LABELS[value] || STATUS_LABELS[value] || titleCase(value) || "Working";
}

export function phaseHeadline(phase, dry = false) {
  const value = String(phase || "starting").toLowerCase();
  const subject = dry ? "Dry Stalk" : "RepoReaper";
  const actions = {
    cleanup: "is cleaning up the workspace.",
    deliver: "is preparing the validated work for delivery.",
    discover: "is finding candidate work.",
    fix: "is building the fixes.",
    judge: "is selecting the relevant files.",
    pr: "is opening the approved pull requests.",
    review: "is reviewing the proposed patches.",
    score: "is scoring the candidate issues.",
    smith: "is reviewing the proposed patches.",
    starting: "is starting the operation.",
    test: "is validating the changes.",
    validate: "is validating the changes.",
  };
  return `${subject} ${actions[value] || "is working on the operation."}`;
}

export function statusTone(status) {
  const value = String(status || "").toLowerCase();
  if (["fixed", "done", "success", "passed", "merged", "open"].includes(value)) return "ok";
  if (["failed", "error", "rejected", "closed"].includes(value)) return "hot";
  if (["running", "working", "partial", "held", "skipped", "queued"].includes(value)) return "warn";
  return "neutral";
}

export function money(value) {
  return `$${Number(value || 0).toFixed(4)}`;
}

export function formatDate(value) {
  if (!value) return "never";
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? String(value) : parsed.toLocaleString();
}

export async function readResponse(response, fallback = "Request failed") {
  const text = await response.text();
  let payload = {};
  try { payload = text ? JSON.parse(text) : {}; } catch { payload = {}; }
  if (!response.ok) throw new Error(payload.error || payload.message || `${fallback}: ${response.status}`);
  return payload;
}

export function normalizeCollection(value) {
  if (Array.isArray(value)) return value;
  if (value && typeof value === "object") return Object.values(value);
  return [];
}

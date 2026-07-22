import { V3_TEXT } from "@patchhivehq/ui-v3";

export const DEFAULT_PARAMS = {
  target_repo: "",
  language: "python",
  min_stars: "50",
  max_repos: "10",
  max_issues: "10",
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
    concurrency: Number(params.concurrency) || 1,
    search_query: params.search_query.trim(),
    cost_budget_usd: Number(params.cost_budget_usd) || 0,
    retry_count: Number(params.retry_count) || 3,
    labels: splitList(params.labels || "bug"),
  };
}

export function createStreamState() {
  return { agentStatuses: {}, done: null, issues: [], logs: [], phase: "idle", report: null, repos: [], runCost: 0, running: false };
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

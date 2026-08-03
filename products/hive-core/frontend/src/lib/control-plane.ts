import { apiFetch } from "./http";

const BASE = "/api/products/hive-core";

interface Envelope<T> {
  data?: T;
  error?: { message?: string };
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await apiFetch(`${BASE}${path}`, init);
  const body = (await response.json().catch(() => null)) as Envelope<T> | null;
  if (!response.ok) {
    throw new Error(body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`);
  }
  if (body?.data === undefined) throw new Error("HiveCore returned no control-plane data.");
  return body.data;
}

export interface SuiteSettings {
  operator_label: string;
  mission: string;
  default_topics: string;
  default_languages: string;
  repo_allowlist: string;
  repo_denylist: string;
  opt_out_notes: string;
  preferred_launch_product: string;
  notes: string;
  updated_at: string;
}

export interface ProductSetting {
  slug: string;
  title: string;
  icon: string;
  lane: string;
  role: string;
  repo: string;
  default_frontend_url: string;
  default_api_url: string;
  override_frontend_url: string;
  override_api_url: string;
  auth_mode: string;
  machine_auth_configured: boolean;
  service_token_configured: boolean;
  legacy_api_key_configured: boolean;
  enabled: boolean;
  notes: string;
  updated_at: string;
  /** Write-only replacement value; never returned by HiveCore. */
  service_token?: string;
}

export interface SettingsResponse {
  product: string;
  tagline: string;
  suite_settings: SuiteSettings;
  products: ProductSetting[];
}

export interface RepositoryPolicy {
  repository: string;
  trusted: boolean;
  operator_excluded: boolean;
  allowlisted: boolean;
  public_opt_out: boolean;
  source: string;
  notes: string;
  updated_at: string;
}

export type Observation<T> =
  | { state: "observed"; value: T }
  | { state: "failed" | "not_observed" | "not_applicable"; reason: string };

export interface RepositoryPoliciesResponse {
  policies: RepositoryPolicy[];
  public_opt_out_sync: Observation<Record<string, unknown>>;
}

export type PrReservationLifecycle =
  | { state: "reserved"; expires_at: string }
  | { state: "committed"; pr_url: string; expires_at: string }
  | { state: "released"; pr_url?: string; reason: string }
  | { state: "expired"; pr_url?: string; reason: string; expiration: string }
  | {
      state: "unknown";
      raw_status: string;
      pr_url?: string;
      reason?: string;
      expires_at?: string;
    };

export interface PrReservation {
  id: string;
  product: string;
  repository: string;
  run_id: string;
  action: string;
  lifecycle: PrReservationLifecycle;
  created_at: string;
  updated_at: string;
}

export interface ProductPrBudget {
  product: string;
  limit: number;
  used: number;
  remaining: number;
}

export interface PrBudgetStatus {
  suite_limit: number;
  suite_used: number;
  suite_remaining: number;
  products: ProductPrBudget[];
  reservations: PrReservation[];
  reconciliation: Observation<Record<string, unknown>>;
}

export type KernelEvidence<T> =
  | { state: "observed"; value: T; observed_at: string }
  | { state: "failed" | "not_observed" | "not_applicable"; reason: string };

export interface ResourcePolicy {
  github_min_remaining: number;
  suite_ai_daily_limit_cents: number;
  sandbox_slots: number;
  updated_at: string;
}

export interface PauseRecord {
  target: { scope: "suite" } | { scope: "product"; product_slug: string } | { scope: "mandate"; mandate_id: string } | { scope: "repository"; repository: string };
  lifecycle: { state: "running"; resumed_at: string } | { state: "paused"; paused_at: string; reason: string; drain: Record<string, unknown> } | { state: "unknown"; raw_state: string; raw_evidence: unknown };
  revision: number;
  created_at: string;
  updated_at: string;
}

export interface GovernanceStatus {
  topology: "unified_in_process" | "gateway_compatibility" | "unknown";
  pauses: PauseRecord[];
  smoke_authority: Record<string, KernelEvidence<{ run_id: string; finished_at: string }>>;
  resource_policy: KernelEvidence<ResourcePolicy>;
  github_rate: KernelEvidence<{ limit: number; remaining: number; reset_at: string }>;
  ai_spend: KernelEvidence<{ daily_limit_cents: number; spent_cents: number; reserved_cents: number; day: string }>;
  sandbox: KernelEvidence<{ slots: number; in_use: number }>;
  reputation: KernelEvidence<{ shipped: number; merged: number; closed_unmerged: number; rolling_decisions: number; rolling_rejections: number; slowdown_active: boolean }>;
}

export interface StartupCheck {
  level: "ok" | "info" | "warn" | "error";
  msg: string;
  code?: string;
  status?: string;
}

export interface HiveCoreHealth {
  status: string;
  version: string;
  product: string;
  auth_enabled: boolean;
  config_errors: number;
  db_ok: boolean;
  db_path: string;
  product_override_count: number;
  repository_policy_count: number;
  suite_pr_limit: number | null;
  mode: string;
}

export interface SuiteHealth {
  service: string;
  status: string;
  version: string;
  mode: string;
  enabled_products: number;
  db_ok: boolean;
  product_override_count: number;
}

export interface ProductActionEvent {
  id: string;
  product_slug: string;
  action_id: string;
  action_label: string;
  method: string;
  path: string;
  target_url: string;
  status: string;
  remote_status: number | null;
  request_json: unknown;
  response_json: unknown;
  error: string;
  created_at: string;
}

export type RuntimeObservation<T> =
  | { state: "observed"; value: T }
  | { state: "failed" | "not_observed" | "not_applicable"; reason: string };

export interface ProductContractCheck {
  id: string;
  label: string;
  path: string;
  ok: boolean;
  status: string;
  error: string;
}

export interface ProductRuntimeDetail {
  slug: string;
  title: string;
  enabled: boolean;
  frontend_url: string;
  api_url: string;
  auth_mode: string;
  machine_auth_configured: boolean;
  service_token_configured: boolean;
  legacy_api_key_configured: boolean;
  notes: string;
  status: string;
  health: {
    status: string;
    health_endpoint: RuntimeObservation<{ reported_status: RuntimeObservation<string>; latency_ms: number; config_errors: RuntimeObservation<number> }>;
    version: RuntimeObservation<string>;
    database_ok: RuntimeObservation<boolean>;
    startup_checks: RuntimeObservation<{ errors: number; warnings: number; infos: number }>;
    capabilities: RuntimeObservation<{ action_count: number }>;
    runs: RuntimeObservation<{ run_count: number }>;
    checked_at: string;
  };
  contract_checks: ProductContractCheck[];
}

export function fetchSettings(signal?: AbortSignal): Promise<SettingsResponse> {
  return request<SettingsResponse>("/settings", { signal });
}

export function saveSettings(settings: SettingsResponse): Promise<SettingsResponse> {
  return request<SettingsResponse>("/settings", {
    method: "PUT",
    body: JSON.stringify({
      suite_settings: settings.suite_settings,
      products: settings.products.map((product) => ({
        slug: product.slug,
        frontend_url: product.override_frontend_url,
        api_url: product.override_api_url,
        service_token: product.service_token || "",
        enabled: product.enabled,
        notes: product.notes,
      })),
    }),
  });
}

export function fetchRepositoryPolicies(signal?: AbortSignal): Promise<RepositoryPoliciesResponse> {
  return request<RepositoryPoliciesResponse>("/repository-policies", { signal });
}

export function saveRepositoryPolicies(value: RepositoryPoliciesResponse): Promise<RepositoryPoliciesResponse> {
  return request<RepositoryPoliciesResponse>("/repository-policies", {
    method: "PUT",
    body: JSON.stringify({
      policies: value.policies.map((policy) => ({
          repository: policy.repository,
          trusted: policy.trusted,
          operator_excluded: policy.operator_excluded,
          allowlisted: policy.allowlisted,
          notes: policy.notes,
        })),
    }),
  });
}

export function fetchPrBudgets(signal?: AbortSignal): Promise<PrBudgetStatus> {
  return request<PrBudgetStatus>("/pr-budgets", { signal });
}

export function fetchGovernance(signal?: AbortSignal): Promise<GovernanceStatus> {
  return request<GovernanceStatus>("/governance", { signal });
}

export function saveResourcePolicy(policy: ResourcePolicy): Promise<ResourcePolicy> {
  return request<ResourcePolicy>("/governance/resources", {
    method: "PUT",
    body: JSON.stringify(policy),
  });
}

export function pauseSuite(reason: string): Promise<PauseRecord> {
  return request<PauseRecord>("/governance/pause", {
    method: "POST",
    body: JSON.stringify({ target: { scope: "suite" }, reason }),
  });
}

export function resumeSuite(): Promise<PauseRecord> {
  return request<PauseRecord>("/governance/resume", {
    method: "POST",
    body: JSON.stringify({ target: { scope: "suite" }, reason: "" }),
  });
}

export function savePrBudgets(value: PrBudgetStatus): Promise<PrBudgetStatus> {
  return request<PrBudgetStatus>("/pr-budgets", {
    method: "PUT",
    body: JSON.stringify({
      suite_limit: Number(value.suite_limit),
      products: value.products.map((product) => ({
        product: product.product,
        limit: Number(product.limit),
      })),
    }),
  });
}

export function releasePrReservation(id: string): Promise<PrReservation> {
  return request<PrReservation>(`/pr-budgets/reservations/${encodeURIComponent(id)}/release`, {
    method: "POST",
    body: JSON.stringify({ reason: "Released manually by the HiveCore operator." }),
  });
}

export async function fetchDiagnostics(signal?: AbortSignal): Promise<{
  suite: SuiteHealth;
  hiveCore: HiveCoreHealth;
  startup: StartupCheck[];
}> {
  const [suiteResponse, coreResponse, startupResponse] = await Promise.all([
    apiFetch("/api/health", { signal }),
    apiFetch(`${BASE}/health`, { signal }),
    apiFetch(`${BASE}/startup/checks`, { signal }),
  ]);
  if (!suiteResponse.ok || !coreResponse.ok || !startupResponse.ok) {
    throw new Error(
      `Diagnostics returned HTTP ${suiteResponse.status}/${coreResponse.status}/${startupResponse.status}.`,
    );
  }
  const startup = (await startupResponse.json()) as { checks?: StartupCheck[] };
  return {
    suite: (await suiteResponse.json()) as SuiteHealth,
    hiveCore: (await coreResponse.json()) as HiveCoreHealth,
    startup: startup.checks ?? [],
  };
}

export function fetchRecentActions(signal?: AbortSignal): Promise<ProductActionEvent[]> {
  return request<ProductActionEvent[]>("/actions/recent", { signal });
}

export function fetchFleetRuntime(signal?: AbortSignal): Promise<ProductRuntimeDetail[]> {
  return request<ProductRuntimeDetail[]>("/products", { signal });
}

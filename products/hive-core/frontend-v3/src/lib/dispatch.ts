// Action dispatch through HiveCore.
//
// HiveCore is the dispatcher, not the deck: it holds the downstream service tokens,
// checks the action's safety metadata, refuses what it must, and records an event.
// The deck's job is to show what will happen before it happens, and to report what
// actually did — never to call a product directly.
//
// Now that the engine is mounted, its dispatch route lives under the suite prefix.

import { apiFetch } from "./http";

export type ActionEffect =
  | { kind: "read_only" }
  | { kind: "writes_local_state" }
  | { kind: "writes_external_state" }
  | { kind: "mutates_repository"; opens_pull_request: boolean };

export type ApprovalPolicy = "automatic" | "operator_required";

export interface DispatchableAction {
  id: string;
  label: string;
  description: string;
  method: string;
  path: string;
  effect: ActionEffect;
  approval: ApprovalPolicy;
  destructive: boolean;
  startsRun: boolean;
  requiredScopes: string[];
  credentialRequirements: string[];
}

export interface ProductActions {
  productKey: string;
  productName: string;
  actions: DispatchableAction[];
}

interface ApiAction {
  id: string;
  label: string;
  description?: string;
  method: string;
  path: string;
  effect?: ActionEffect;
  approval?: ApprovalPolicy;
  destructive?: boolean;
  starts_run?: boolean;
  required_scopes?: string[];
  credential_requirements?: string[];
}

interface ApiCapabilityReport {
  key: string;
  advertised: { display_name?: string; actions: ApiAction[] } | null;
}

function parseEffect(raw: ApiAction): ActionEffect {
  const effect = raw.effect;
  if (!effect) throw new Error(`Action ${raw.id} does not declare an effect.`);
  if (
    effect.kind === "read_only" ||
    effect.kind === "writes_local_state" ||
    effect.kind === "writes_external_state"
  ) {
    return { kind: effect.kind };
  }
  if (
    effect.kind === "mutates_repository" &&
    typeof effect.opens_pull_request === "boolean"
  ) {
    return effect;
  }
  throw new Error(`Action ${raw.id} declares an invalid effect.`);
}

function parseApproval(raw: ApiAction): ApprovalPolicy {
  if (raw.approval === "automatic" || raw.approval === "operator_required") {
    return raw.approval;
  }
  throw new Error(`Action ${raw.id} does not declare an approval policy.`);
}

function toAction(raw: ApiAction): DispatchableAction {
  return {
    id: raw.id,
    label: raw.label,
    description: raw.description ?? "",
    method: raw.method,
    path: raw.path,
    effect: parseEffect(raw),
    approval: parseApproval(raw),
    destructive: Boolean(raw.destructive),
    startsRun: Boolean(raw.starts_run),
    requiredScopes: raw.required_scopes ?? [],
    credentialRequirements: raw.credential_requirements ?? [],
  };
}

export function isMutatingAction(action: DispatchableAction): boolean {
  return action.effect.kind !== "read_only";
}

export function actionOpensPullRequest(action: DispatchableAction): boolean {
  return (
    action.effect.kind === "mutates_repository" && action.effect.opens_pull_request
  );
}

export function actionRequiresApproval(action: DispatchableAction): boolean {
  return action.approval === "operator_required";
}

export async function fetchDispatchableActions(
  signal?: AbortSignal,
): Promise<ProductActions[]> {
  const response = await apiFetch("/api/products/capabilities", { signal });
  if (!response.ok) throw new Error(`HTTP ${response.status} from /api/products/capabilities`);
  const rows = (await response.json()) as ApiCapabilityReport[];

  return rows
    .filter((row) => row.advertised && row.key !== "hive-core")
    .map((row) => ({
      productKey: row.key,
      productName: row.advertised?.display_name ?? row.key,
      actions: (row.advertised?.actions ?? []).map(toAction),
    }))
    .filter((row) => row.actions.length > 0);
}

/**
 * Why HiveCore will refuse an action, decided before dispatching rather than after.
 *
 * These mirror the engine's own guards. Duplicating them here is not the deck
 * second-guessing the backend — the backend still refuses independently — it is so
 * the operator sees the reason on the button instead of in an error toast.
 */
export function refusalReason(action: DispatchableAction): string | null {
  if (action.destructive) {
    return "Destructive actions are not dispatched.";
  }
  return null;
}

export interface DispatchOutcome {
  ok: boolean;
  status: string;
  message: string;
  remoteStatus: number | null;
  startedRun: boolean;
  eventId: string;
  approvalRequired: boolean;
  approvalId: string;
}

interface DispatchEnvelope {
  data?: {
    outcome?: "dispatched" | "approval_required";
    event?: {
      id?: string;
      status?: string;
      remote_status?: number | null;
      error?: string;
    };
    started_run?: boolean;
    approval?: { id?: string };
  };
  error?: { code?: string; message?: string };
}

/**
 * Dispatch through the mounted HiveCore engine.
 *
 * A non-2xx here is HiveCore refusing or failing. A 2xx with an event whose status
 * is "failed" means HiveCore dispatched and the *product* rejected it — different
 * facts, reported differently.
 */
export async function dispatchAction(
  productKey: string,
  actionId: string,
  payload: unknown = {},
): Promise<DispatchOutcome> {
  let response: Response;
  try {
    response = await apiFetch(
      `/api/products/hive-core/products/${productKey}/actions/${actionId}`,
      { method: "POST", body: JSON.stringify(payload) },
    );
  } catch {
    return {
      ok: false,
      status: "unreachable",
      message: "Could not reach the control plane.",
      remoteStatus: null,
      startedRun: false,
      eventId: "",
      approvalRequired: false,
      approvalId: "",
    };
  }

  const body = (await response.json().catch(() => null)) as DispatchEnvelope | null;

  if (!response.ok) {
    return {
      ok: false,
      status: body?.error?.code ?? "dispatch_refused",
      message: body?.error?.message ?? `HiveCore returned HTTP ${response.status}.`,
      remoteStatus: response.status,
      startedRun: false,
      eventId: "",
      approvalRequired: false,
      approvalId: "",
    };
  }

  if (body?.data?.outcome === "approval_required") {
    return {
      ok: false,
      status: "pending_approval",
      message: "This exact action and input are waiting in the approval inbox. Nothing was dispatched.",
      remoteStatus: null,
      startedRun: false,
      eventId: "",
      approvalRequired: true,
      approvalId: body.data.approval?.id ?? "",
    };
  }

  const event = body?.data?.event;
  const dispatched = event?.status === "dispatched";
  return {
    ok: dispatched,
    status: event?.status ?? "unknown",
    message: dispatched
      ? `Product accepted the action${body?.data?.started_run ? " and started a run" : ""}.`
      : (event?.error ?? "The product rejected the action."),
    remoteStatus: event?.remote_status ?? null,
    startedRun: Boolean(body?.data?.started_run),
    eventId: event?.id ?? "",
    approvalRequired: false,
    approvalId: "",
  };
}

/**
 * Provision a downstream service token through HiveCore.
 *
 * The suite-level provisioning route mints a token and deliberately discards it, so
 * nothing can dispatch with it. HiveCore's own route mints and stores it server-side
 * where its dispatcher reads from — which is the whole point of the broker.
 */
export async function provisionThroughHiveCore(productKey: string): Promise<DispatchOutcome> {
  const response = await apiFetch(
    `/api/products/hive-core/products/${productKey}/provision-service-token`,
    { method: "POST", body: JSON.stringify({}) },
  );
  const body = (await response.json().catch(() => null)) as
    | { data?: { message?: string }; error?: { message?: string } }
    | null;

  return {
    ok: response.ok,
    status: response.ok ? "provisioned" : "failed",
    message:
      (response.ok ? body?.data?.message : body?.error?.message) ??
      `HiveCore returned HTTP ${response.status}.`,
    remoteStatus: response.status,
    startedRun: false,
    eventId: "",
    approvalRequired: false,
    approvalId: "",
  };
}

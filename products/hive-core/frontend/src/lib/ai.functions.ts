// AI calls go through the Rust control plane, never straight from the browser.
//
// The Lovable export used TanStack Start server functions that talked to
// ai.gateway.lovable.dev with a provider key held in the frontend's Node process.
// PatchHive's rule is the opposite: no provider SDKs in the browser, and the local
// OpenAI-compatible gateway (PATCHHIVE_AI_URL) is preferred over raw provider
// endpoints. So these are thin POSTs to HiveCore, which owns the model choice, the
// key, and the grounding context.
//
// Both endpoints are served by HiveCore's pipeline::ai module. They produce drafts
// for an operator to read and edit — never a dispatch, never a write. The wire shape
// is snake_case because the Rust side is the contract, not the browser.

import { API } from "@/config";
import { apiFetch } from "./http";

const BASE = "/api/products/hive-core";

async function post<T>(path: string, body: unknown): Promise<T> {
  let response: Response;
  try {
    response = await apiFetch(path, {
      method: "POST",
      body: JSON.stringify(body),
    });
  } catch {
    throw new Error(
      `HiveCore is unreachable at ${API}. AI explanations are served by the control plane, not the browser.`,
    );
  }

  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as
      | { error?: { message?: string } }
      | null;
    throw new Error(payload?.error?.message ?? `HiveCore returned HTTP ${response.status}.`);
  }

  const payload = (await response.json()) as { data?: T } | T;
  return ((payload as { data?: T }).data ?? payload) as T;
}

/** Mirrors pipeline::ai::IncidentSummaryInput. */
export interface IncidentSummaryInput {
  product_name: string;
  severity: string;
  summary: string;
  opened_minutes_ago: number;
  closed: boolean;
  resolution?: string;
  logs?: string[];
}

export interface GeneratedText {
  text: string;
  /** The model HiveCore used, so a draft can be attributed. */
  model: string;
}

export function summarizeIncident(data: IncidentSummaryInput): Promise<GeneratedText> {
  return post<GeneratedText>(`${BASE}/incidents/summarize`, data);
}

/** Mirrors pipeline::ai::ExplainFailureInput. */
export interface ExplainFailureInput {
  product: string;
  capability: string;
  error_code: string;
  stage: string;
  message: string;
  logs: string[];
  inputs: Record<string, string | number | boolean>;
}

export function explainFailure(data: ExplainFailureInput): Promise<GeneratedText> {
  return post<GeneratedText>(`${BASE}/runs/explain`, data);
}

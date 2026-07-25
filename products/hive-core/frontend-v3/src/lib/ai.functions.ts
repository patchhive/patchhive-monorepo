// AI calls go through the Rust control plane, never straight from the browser.
//
// The Lovable export used TanStack Start server functions that talked to
// ai.gateway.lovable.dev with a provider key held in the frontend's Node process.
// PatchHive's rule is the opposite: no provider SDKs in the browser, and the local
// OpenAI-compatible gateway (PATCHHIVE_AI_URL) is preferred over raw provider
// endpoints. So these are thin POSTs to HiveCore, which owns the model choice, the
// key, and the grounding context.
//
// The endpoints are not implemented yet; until they are, these reject with a message
// that says so rather than failing silently.

import { API } from "@/config";
import { apiFetch } from "./http";

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

  if (response.status === 404) {
    throw new Error(
      `${path} is not implemented yet. The model call belongs in the Rust backend behind PATCHHIVE_AI_URL.`,
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

export interface IncidentSummaryInput {
  productName: string;
  severity: string;
  summary: string;
  openedMinutesAgo: number;
  closed: boolean;
  resolution?: string;
  logs?: string[];
}

export function summarizeIncident(data: IncidentSummaryInput): Promise<{ text: string }> {
  return post<{ text: string }>("/incidents/summarize", data);
}

export interface ExplainFailureInput {
  product: string;
  capability: string;
  errorCode: string;
  stage: string;
  message: string;
  logs: string[];
  inputs: Record<string, string | number | boolean>;
}

export function explainFailure(data: ExplainFailureInput): Promise<{ text: string }> {
  return post<{ text: string }>("/runs/explain", data);
}

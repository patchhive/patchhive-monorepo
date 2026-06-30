import { useEffect, useMemo, useState } from "react";
import {
  AI_PROVIDERS,
  DEFAULT_PROVIDER_MODELS,
  modelListForProvider,
} from "./providerCatalog.js";
import { filterPatchHiveTextModels } from "./modelFilters.js";

export function normalizeFallbackModels(models) {
  if (!models || typeof models !== "object") return DEFAULT_PROVIDER_MODELS;
  return { ...DEFAULT_PROVIDER_MODELS, ...models };
}

export function providerModelStatusCopy(
  provider,
  status,
  loading,
  localGatewayConfigured,
  globalKeyConfigured,
) {
  if (loading) return "Loading models...";
  if (status?.error) return status.error;
  if (status?.source === "patchhive-ai-local") return "Live models from PatchHive Local AI.";
  if (status?.source === "provider-api") return "Live models from provider API.";
  if (status?.source === "ollama") return "Live models from local Ollama.";
  if (status?.source === "static_fallback") return "Using fallback list because live discovery failed.";
  if (provider === "custom") return "Set a custom base URL, then pull models or type the model id manually.";
  if (provider === "openai" && localGatewayConfigured) return "Ready to discover models from PatchHive Local AI.";
  if (globalKeyConfigured) return "Ready to discover models with the saved global provider key.";
  return AI_PROVIDERS[provider]?.liveHint || "Using built-in provider model list.";
}

function buildModelEndpoint(apiBase, modelsPath, provider) {
  return `${String(apiBase || "").replace(/\/$/, "")}${modelsPath}/${encodeURIComponent(provider)}`;
}

function modelTestStatusCopy(status, testing) {
  if (testing) return "Testing model...";
  if (!status?.message) return "";
  if (status.ok) {
    const latency = Number.isFinite(Number(status.latency_ms)) ? ` in ${status.latency_ms}ms` : "";
    return `${status.message}${latency}`;
  }
  if (status.kind === "rate_limited") return `Provider rate limited this model test: ${status.message}`;
  if (status.kind === "auth_error") return `Provider credentials failed: ${status.message}`;
  if (status.kind === "timeout") return `Provider timed out: ${status.message}`;
  return status.message;
}

function filteredModelStatusText(filteredCount) {
  if (!filteredCount) return "";
  return `${filteredCount} non-text/provider utility models hidden.`;
}

export function useProviderModelDiscovery({
  apiBase,
  authToken = "",
  provider,
  model = "",
  onModelChange = () => {},
  providerKey = "",
  baseUrl = "",
  fallbackModels,
  modelsPath = "/models",
  localGatewayConfigured = false,
  globalKeyConfigured = false,
  autoLoad = true,
} = {}) {
  const mergedFallbackModels = useMemo(
    () => normalizeFallbackModels(fallbackModels),
    [fallbackModels],
  );
  const [liveModels, setLiveModels] = useState({});
  const [modelStatus, setModelStatus] = useState({});
  const [loadingModels, setLoadingModels] = useState({});
  const [testStatus, setTestStatus] = useState({});
  const [testingModels, setTestingModels] = useState({});

  const models = modelListForProvider(provider, liveModels, mergedFallbackModels);
  const loading = !!loadingModels[provider];
  const status = modelStatus[provider] || {};
  const testing = !!testingModels[provider];
  const test = testStatus[provider] || {};

  const loadModels = async ({ includeProviderKey = false } = {}) => {
    if (!apiBase || !provider) return;

    const key = includeProviderKey ? String(providerKey || "").trim() : "";
    const base = includeProviderKey ? String(baseUrl || "").trim() : "";
    const shouldPost = !!key || !!base;
    setLoadingModels((current) => ({ ...current, [provider]: true }));

    try {
      const headers = {
        ...(authToken ? { "X-API-Key": authToken } : {}),
        ...(shouldPost ? { "Content-Type": "application/json" } : {}),
      };
      const response = await fetch(buildModelEndpoint(apiBase, modelsPath, provider), {
        method: shouldPost ? "POST" : "GET",
        headers,
        body: shouldPost ? JSON.stringify({ api_key: key, base_url: base }) : undefined,
      });
      const data = await response.json().catch(() => ({}));
      const rawModels = Array.isArray(data.models) && data.models.length
        ? data.models
        : (mergedFallbackModels[provider] || []);
      const filtered = filterPatchHiveTextModels(rawModels);
      const nextModels = filtered.models;
      const filteredCount = filtered.dropped.length;

      setLiveModels((current) => ({ ...current, [provider]: nextModels }));
      setModelStatus((current) => ({
        ...current,
        [provider]: {
          source: data.source || "static",
          error: data.error
            || (rawModels.length && !nextModels.length
              ? "No pulled models matched PatchHive text/chat filters. Use manual model entry if needed."
              : ""),
          filteredCount,
        },
      }));

      if (nextModels.length && !nextModels.includes(model)) {
        onModelChange(nextModels[0] || "");
      }
    } catch (error) {
      setModelStatus((current) => ({
        ...current,
        [provider]: {
          source: "static_fallback",
          error: `Could not load models: ${error.message || error}`,
        },
      }));
    } finally {
      setLoadingModels((current) => ({ ...current, [provider]: false }));
    }
  };

  const testModel = async () => {
    if (!apiBase || !provider || !model) return;

    const key = String(providerKey || "").trim();
    const base = String(baseUrl || "").trim();
    setTestingModels((current) => ({ ...current, [provider]: true }));

    try {
      const response = await fetch(`${buildModelEndpoint(apiBase, modelsPath, provider)}/test`, {
        method: "POST",
        headers: {
          ...(authToken ? { "X-API-Key": authToken } : {}),
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ api_key: key, base_url: base, model }),
      });
      const data = await response.json().catch(() => ({}));
      setTestStatus((current) => ({
        ...current,
        [provider]: {
          ...data,
          ok: Boolean(data.ok),
          message: data.message || (response.ok ? "Model test finished." : "Model test failed."),
        },
      }));
    } catch (error) {
      setTestStatus((current) => ({
        ...current,
        [provider]: {
          ok: false,
          kind: "request_error",
          message: `Could not test model: ${error.message || error}`,
        },
      }));
    } finally {
      setTestingModels((current) => ({ ...current, [provider]: false }));
    }
  };

  useEffect(() => {
    if (!autoLoad) return;
    loadModels({ includeProviderKey: false });
    // Provider changes should auto-load global/local models. Per-provider typed keys
    // only travel after the user explicitly pulls the live list.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [provider, apiBase, authToken, localGatewayConfigured, globalKeyConfigured, autoLoad]);

  return {
    loadModels,
    loading,
    models,
    status,
    statusText: providerModelStatusCopy(
      provider,
      status,
      loading,
      localGatewayConfigured,
      globalKeyConfigured,
    ),
    filteredStatusText: filteredModelStatusText(status.filteredCount),
    testModel,
    testing,
    testStatus: test,
    testStatusText: modelTestStatusCopy(test, testing),
  };
}

import { useEffect, useMemo, useState } from "react";
import {
  AI_PROVIDERS,
  DEFAULT_PROVIDER_MODELS,
  modelListForProvider,
} from "./providerCatalog.js";

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

  const models = modelListForProvider(provider, liveModels, mergedFallbackModels);
  const loading = !!loadingModels[provider];
  const status = modelStatus[provider] || {};

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
      const nextModels = Array.isArray(data.models) && data.models.length
        ? data.models
        : (mergedFallbackModels[provider] || []);

      setLiveModels((current) => ({ ...current, [provider]: nextModels }));
      setModelStatus((current) => ({
        ...current,
        [provider]: {
          source: data.source || "static",
          error: data.error || "",
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
  };
}

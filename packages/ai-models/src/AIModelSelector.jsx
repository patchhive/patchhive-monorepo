import { useEffect, useMemo, useState } from "react";
import { Btn, Input, S, Sel } from "@patchhivehq/ui";
import {
  AI_PROVIDERS,
  DEFAULT_PROVIDER_MODELS,
  defaultModelForProvider,
  modelListForProvider,
  providerOptions,
} from "./providerCatalog.js";

function normalizeFallbackModels(models) {
  if (!models || typeof models !== "object") return DEFAULT_PROVIDER_MODELS;
  return { ...DEFAULT_PROVIDER_MODELS, ...models };
}

function statusCopy(provider, status, loading, localGatewayConfigured, globalKeyConfigured) {
  if (loading) return "Loading models...";
  if (status?.error) return status.error;
  if (status?.source === "patchhive-ai-local") return "Live models from PatchHive Local AI.";
  if (status?.source === "provider-api") return "Live models from provider API.";
  if (status?.source === "ollama") return "Live models from local Ollama.";
  if (status?.source === "static_fallback") return "Using fallback list because live discovery failed.";
  if (provider === "custom") return "Set a custom base URL, then refresh live models or type the model id manually.";
  if (provider === "openai" && localGatewayConfigured) return "Ready to discover models from PatchHive Local AI.";
  if (globalKeyConfigured) return "Ready to discover models with the saved global provider key.";
  return AI_PROVIDERS[provider]?.liveHint || "Using built-in provider model list.";
}

export default function AIModelSelector({
  apiBase,
  authToken = "",
  provider,
  model,
  onProviderChange,
  onModelChange,
  providerKey = "",
  baseUrl = "",
  fallbackModels,
  providers = AI_PROVIDERS,
  localGatewayConfigured = false,
  globalKeyConfigured = false,
  disabled = false,
}) {
  const mergedFallbackModels = useMemo(() => normalizeFallbackModels(fallbackModels), [fallbackModels]);
  const [liveModels, setLiveModels] = useState({});
  const [modelStatus, setModelStatus] = useState({});
  const [loadingModels, setLoadingModels] = useState({});

  const models = modelListForProvider(provider, liveModels, mergedFallbackModels);
  const loading = !!loadingModels[provider];
  const status = modelStatus[provider] || {};

  const loadModels = async ({ includeProviderKey = false } = {}) => {
    if (!apiBase || !provider) return;

    const key = includeProviderKey ? providerKey.trim() : "";
    const base = includeProviderKey ? baseUrl.trim() : "";
    const shouldPost = !!key || !!base;
    setLoadingModels(current => ({ ...current, [provider]: true }));

    try {
      const headers = {
        ...(authToken ? { "X-API-Key": authToken } : {}),
        ...(shouldPost ? { "Content-Type": "application/json" } : {}),
      };
      const response = await fetch(`${apiBase}/models/${provider}`, {
        method: shouldPost ? "POST" : "GET",
        headers,
        body: shouldPost ? JSON.stringify({ api_key: key, base_url: base }) : undefined,
      });
      const data = await response.json();
      const nextModels = Array.isArray(data.models) && data.models.length
        ? data.models
        : (mergedFallbackModels[provider] || []);

      setLiveModels(current => ({ ...current, [provider]: nextModels }));
      setModelStatus(current => ({
        ...current,
        [provider]: {
          source: data.source || "static",
          error: data.error || "",
        },
      }));

      if (!nextModels.includes(model)) {
        onModelChange(nextModels[0] || "");
      }
    } catch (error) {
      setModelStatus(current => ({
        ...current,
        [provider]: {
          source: "static_fallback",
          error: `Could not load models: ${error}`,
        },
      }));
    } finally {
      setLoadingModels(current => ({ ...current, [provider]: false }));
    }
  };

  useEffect(() => {
    loadModels({ includeProviderKey: false });
    // Provider changes should auto-load global/local models. Per-provider typed keys
    // only travel after the user explicitly refreshes the live list.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [provider, apiBase, authToken, localGatewayConfigured, globalKeyConfigured]);

  const handleProviderChange = nextProvider => {
    onProviderChange(nextProvider);
    const nextModel = liveModels[nextProvider]?.[0]
      || defaultModelForProvider(nextProvider, mergedFallbackModels);
    onModelChange(nextModel);
  };

  return (
    <>
      <div style={S.field}>
        <label style={S.label}>Provider</label>
        <Sel
          value={provider}
          onChange={handleProviderChange}
          opts={providerOptions(providers)}
          disabled={disabled}
        />
      </div>
      <div style={{ ...S.field, gridColumn:"1/-1" }}>
        <label style={S.label}>Model</label>
        <div style={{ display:"grid", gridTemplateColumns:"1fr auto", gap:8 }}>
          {provider === "custom"
            ? (
              <Input
                value={model}
                onChange={onModelChange}
                placeholder={models[0] || "model-id"}
                disabled={disabled || loading}
              />
            )
            : <Sel value={model} onChange={onModelChange} opts={models} disabled={disabled || loading} />}
          <Btn
            onClick={() => loadModels({ includeProviderKey: true })}
            color={AI_PROVIDERS[provider]?.color || "var(--accent)"}
            disabled={disabled || loading}
            style={{ whiteSpace:"nowrap", fontSize:10, padding:"6px 10px" }}
          >
            {loading ? "Loading" : "Refresh live"}
          </Btn>
        </div>
        <div style={{
          fontSize:10,
          color:status?.error ? "var(--gold)" : "var(--text-muted)",
          marginTop:4,
          lineHeight:1.4,
        }}>
          {statusCopy(provider, status, loading, localGatewayConfigured, globalKeyConfigured)}
        </div>
      </div>
    </>
  );
}

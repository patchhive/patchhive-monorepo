# @patchhivehq/ai-models

Shared AI provider and model-selection UI for PatchHive products.

This package keeps product frontends from each inventing their own provider list, fallback model catalog, model refresh button, and "live vs static" status copy. Products still own their backend model-discovery endpoints because backend behavior may depend on product-specific auth, local gateway settings, and provider safety rules.

## What It Provides

- `AIModelSelector`: provider + model selector with live refresh support.
- `AI_PROVIDERS`: shared provider metadata.
- `DEFAULT_PROVIDER_MODELS`: conservative fallback model lists.
- `defaultModelForProvider`: helper for initializing agent/config forms.

## Expected Product Endpoint

Products that use `AIModelSelector` should expose:

```text
GET /models/:provider
POST /models/:provider
```

`GET` should use saved/global credentials and local gateway configuration.

`POST` may accept:

```json
{
  "api_key": "provider key for one-time model discovery",
  "base_url": "optional OpenAI-compatible or Ollama base URL"
}
```

The endpoint should return:

```json
{
  "models": ["model-id"],
  "source": "provider-api",
  "error": ""
}
```

Provider keys should only travel from the browser to the local product backend, not directly from the browser to third-party provider APIs.

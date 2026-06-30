# @patchhivehq/ai-models

Shared AI provider and model-selection UI for PatchHive products.

This package keeps product frontends from each inventing their own provider list,
fallback model catalog, model refresh behavior, and "live vs static" status
copy. Products still own their backend model-discovery endpoints because backend
behavior may depend on product-specific auth, local gateway settings, and
provider safety rules.

## What It Provides

- `useProviderModelDiscovery`: UI-agnostic hook for v2 product frontends that
  render their own controls.
- `AIModelSelector`: legacy provider + model selector with live refresh support.
- `AI_PROVIDERS`: shared provider metadata.
- `DEFAULT_PROVIDER_MODELS`: conservative fallback model lists.
- `defaultModelForProvider`: helper for initializing agent/config forms.
- Pulled model lists are filtered to PatchHive-suitable text/chat models, hiding
  obvious embeddings, rerankers, STT/TTS/audio, image/video, moderation, and
  provider utility models from noisy provider catalogs.
- Free models are not hidden by default. Products may expose a `freeOnly`
  picker option to narrow noisy provider catalogs to model IDs marked as free
  while keeping manual model entry available.

Prefer `@patchhivehq/ai-models/model-discovery` for new v2 surfaces so they do
not inherit old UI components while the suite is migrating.

## Expected Product Endpoint

Products that use `useProviderModelDiscovery` or `AIModelSelector` should expose:

```text
GET /models/:provider
POST /models/:provider
POST /models/:provider/test
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

`POST /models/:provider/test` may accept:

```json
{
  "api_key": "provider key for one-time connectivity testing",
  "base_url": "optional OpenAI-compatible or Ollama base URL",
  "model": "model-id"
}
```

It should make a tiny completion request through the product's real provider
runtime and return an `ok` boolean plus a sanitized `kind` such as `ok`,
`auth_error`, `rate_limited`, `timeout`, or `provider_error`.

Provider keys should only travel from the browser to the local product backend, not directly from the browser to third-party provider APIs.

Products should keep manual model entry available even when the picker filters
provider catalogs. This lets operators force a valid provider-specific model id
when a catalog entry is named unusually or when a provider's `/models` endpoint
is incomplete.

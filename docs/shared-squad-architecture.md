# Shared Squad Architecture

RepoReaper's agent team should become the first implementation of a shared
PatchHive Squad pattern instead of remaining a one-off product feature.

A Squad is a product-owned set of AI-capable roles backed by shared provider,
model, credential, preset, readiness, and audit plumbing. Products decide what
their roles mean. The platform decides how those roles are configured, stored,
tested, and exposed to HiveCore.

## Why This Should Be Shared

RepoReaper needed a full team because it patches code and opens PRs. Future
products may need lighter AI-capable teams:

- ReviewBee could use a reviewer squad for deep PR-thread and diff analysis.
- TrustGate could use policy analysts for rule tuning and diff explanation.
- RefactorScout could use a cleanup squad for safe refactor plan generation.
- ReleaseSentry could use release analysts for notes, risk summaries, and
  release checklist generation.
- HiveCore could coordinate squads without knowing every product's private
  team-builder UI.

The shared layer should prevent every AI-capable product from rebuilding:

- provider and model setup
- live model pulling and model testing
- noisy provider model filtering
- encrypted per-agent secret storage
- active squad and preset persistence
- readiness checks
- product API shapes for HiveCore
- audit-friendly redaction and status reporting

## Ownership Boundary

Shared code should cover the common Squad substrate.

Good shared candidates:

- `SquadAgentConfig` shape: id, name, role, provider, model, base URL, optional
  provider key reference, optional bot identity override, status, current task,
  and stats.
- `SquadPreset` shape: name, agents, created time, updated time, product key.
- encrypted storage helpers for per-agent provider keys and token overrides
  using `patchhive_product_core::secrets::TokenProtector`.
- model discovery and model testing contracts.
- provider/model filtering policy for PatchHive-suitable text/chat models.
- readiness/startup checks for missing providers, missing squads, and missing
  encryption keys.
- shared API response redaction so browser responses never leak provider keys,
  GitHub tokens, or service tokens.
- shared HiveCore capability metadata for products that expose Squad setup.

Product-owned behavior should stay product-owned.

Do not share too early:

- RepoReaper's Scout/Judge/Reaper/Smith/Gatekeeper role logic.
- prompt text and scoring heuristics.
- patch generation, file selection, validation, PR creation, or cost policy.
- product-specific approval gates.
- product-specific run state machines.
- product-specific evidence and handoff decisions.

The rule: share the team plumbing, not the product brain.

## Proposed Core Modules

The first extraction target should be `crates/patchhive-product-core`.

Suggested Rust module shape:

```text
crates/patchhive-product-core/src/
  squad/
    mod.rs
    model.rs          # shared agent, preset, stats, redacted views
    storage.rs        # active squad + preset persistence helpers
    readiness.rs      # startup/readiness checks
    redaction.rs      # browser-safe response helpers
```

Frontend/provider support should stay in `packages/ai-models`:

```text
packages/ai-models/src/
  providerCatalog.js
  modelFilters.js
  useProviderModelDiscovery.js
```

`packages/ai-models` should stay UI-agnostic for v2 surfaces. Product UIs render
their own controls and call the shared hook. The old `AIModelSelector` can remain
only while legacy frontends exist.

## Shared API Shape

AI-capable products should eventually converge on:

```text
GET    /squad
POST   /squad
DELETE /squad/:agent_id
GET    /squad/presets
POST   /squad/presets
DELETE /squad/presets/:name
GET    /models/:provider
POST   /models/:provider
POST   /models/:provider/test
```

Products can keep compatibility aliases such as RepoReaper's existing
`/agents` and `/presets` while migrating.

### Agent Config

```json
{
  "id": "scout-01",
  "name": "PatchHive Scout",
  "role": "scout",
  "provider": "openrouter",
  "model": "openai/gpt-4.1-mini",
  "base_url": "https://openrouter.ai/api/v1",
  "api_key": "",
  "bot_token": "",
  "bot_user": "",
  "status": "idle",
  "current_task": "",
  "stats": {
    "fixed": 0,
    "skipped": 0,
    "errors": 0,
    "cost": 0.0
  }
}
```

Browser responses should either omit secret fields or return empty strings for
them. Saved storage may contain encrypted secret values. Plaintext provider keys
should only exist in memory long enough to save encrypted storage or perform a
local provider/model test.

## Credential Rules

Squad credentials are sensitive enough to be platform-owned:

- Per-product encryption keys are preferred, for example
  `REAPER_ENCRYPTION_KEY`.
- `PATCHHIVE_ENCRYPTION_KEY` can be a suite-wide fallback.
- Without an encryption key, per-agent provider keys and bot token overrides
  should remain memory-only and must not be persisted as plaintext.
- Adding an encryption key later should migrate existing plaintext rows where
  older versions allowed plaintext storage.
- Browser responses must never echo provider keys or GitHub token overrides.
- HiveCore should eventually configure squads through server-side calls, not by
  exposing product service tokens or provider secrets to browser code.

## Model Discovery Rules

The shared model-discovery path should remain provider-neutral:

- Pull live model lists through the local product backend, not directly from the
  browser to third-party provider APIs.
- Test selected models with a tiny completion request through the product's real
  provider runtime.
- Report sanitized `kind` values such as `ok`, `auth_error`, `rate_limited`,
  `timeout`, and `provider_error`.
- Filter noisy provider catalogs to PatchHive-suitable text/chat models.
- Keep **Agent-ready only** separate from price filtering. Products opt into it
  explicitly; capability metadata should drive text modality, context, output
  control, tool, and structured-output checks where the provider exposes them.
- Do not hide free models by default; expose a free-only narrowing option for
  catalogs such as OpenRouter where free and paid models are mixed together.
- Keep manual model entry outside both filters for provider catalogs with
  incomplete metadata or unusually named model IDs.

This matters for providers such as OpenRouter or NIM where a single `/models`
response can include hundreds of entries that PatchHive products should never
select, such as embeddings, rerankers, STT/TTS/audio, image/video, moderation,
and provider utility models.

## HiveCore Role

HiveCore should eventually become the squad cockpit:

- show which products have AI squads configured
- show provider/model readiness without exposing keys
- rotate or clear product-owned squad credentials through backend routes
- apply suite defaults to products that support squads
- dispatch product-owned AI actions only after approval gates are visible
- show Squad drift when one product does not expose the shared setup contract

HiveCore should not own product prompts or agent decisions. It should own
visibility, setup, policy, and orchestration.

## Migration Plan

1. Keep RepoReaper's current v2 Squad setup working.
2. Treat `packages/ai-models/model-discovery` as the first shared frontend
   piece.
3. When a second product needs AI roles, extract `SquadAgentConfig`,
   redaction, encrypted storage, and preset helpers into
   `patchhive-product-core::squad`.
4. Add compatibility adapters so RepoReaper's `/agents` and `/presets` can keep
   working while the shared `/squad` contract appears.
5. Add HiveCore Squad setup once two products expose the shared contract.
6. Retire product-specific team-builder UI paths after HiveCore can configure
   squads and each product v2 frontend can still manage its own active team.

## Current RepoReaper Status

RepoReaper already has the seed of the shared Squad architecture:

- active team and preset persistence
- encrypted per-agent secret storage when `REAPER_ENCRYPTION_KEY` or
  `PATCHHIVE_ENCRYPTION_KEY` is configured
- provider defaults in v2
- live model pulling
- model testing
- noisy model filtering
- guarded full hunts and no-write Dry Stalk

The next shared extraction should happen when another product actually needs AI
roles, so the shared core is shaped by at least two real products rather than by
RepoReaper alone.

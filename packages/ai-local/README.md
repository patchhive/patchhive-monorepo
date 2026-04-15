# @patchhive/ai-local

`@patchhive/ai-local` is the local AI gateway for PatchHive products.

It gives the suite one stable OpenAI-compatible endpoint while the actual model execution can come from official, user-owned provider paths such as Codex and GitHub Copilot. That keeps PatchHive products provider-agnostic without teaching every product how to handle auth, model discovery, and fallback logic on its own.

## What It Provides

- a localhost API for `/v1/models`, `/v1/chat/completions`, and `/v1/responses`
- health reporting with adapter auth hints and restart metadata
- provider fallback across available adapters
- a path toward a hybrid gateway with a Rust public edge and Node adapters underneath

## Why It Exists

- PatchHive products should integrate with one gateway contract, not many provider-specific auth flows.
- Local user subscriptions and local auth state should remain usable.
- The platform should stay compatible with official SDKs instead of hard-coding itself to a third-party gateway.

## Run Locally

```bash
npm install
npm run dev:ai-local

# or the Rust-edge hybrid gateway
npm run dev:ai-local-rust
```

Default base URL:

```bash
PATCHHIVE_AI_URL=http://127.0.0.1:8787/v1
```

## Configuration

Key environment variables include:

- `PATCHHIVE_AI_HOST`
- `PATCHHIVE_AI_PORT`
- `PATCHHIVE_AI_PROVIDER_ORDER`
- `PATCHHIVE_AI_TIMEOUT_MS`
- `PATCHHIVE_AI_CODEX_MODEL`
- `PATCHHIVE_AI_COPILOT_MODEL`
- `PATCHHIVE_AI_COPILOT_GITHUB_TOKEN`
- `PATCHHIVE_AI_COPILOT_USE_LOGGED_IN_USER`
- `PATCHHIVE_AI_COPILOT_HOME`
- `PATCHHIVE_AI_ENABLE_COPILOT`

## Repository Model

The PatchHive monorepo is the source of truth for `@patchhive/ai-local`. The standalone `patchhive/patchhive-ai-local` repository is an exported mirror of this directory.

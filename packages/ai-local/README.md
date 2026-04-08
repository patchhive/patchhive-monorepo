# @patchhive/ai-local

Localhost AI gateway for PatchHive products.

This repository is mirrored to `patchhive/patchhive-ai-local`, but the PatchHive monorepo remains the source of truth for development and releases.

It gives the monorepo one stable OpenAI-compatible endpoint while the actual engine can be:

- Codex via the official Codex SDK and the local user’s Codex login
- GitHub Copilot via the official Copilot SDK and the local user’s GitHub/Copilot auth

## Why this exists

- PatchHive products should not each learn Codex auth, Copilot auth, and fallback logic separately.
- RepoReaper and future products can point at one local base URL and stay provider-agnostic.
- The gateway keeps the long-term path official-SDK-first instead of depending on a third-party proxy forever.

## Long-Term Direction

The current implementation is Node-only because the official Codex and Copilot SDKs are Node-first.

The long-term plan is hybrid:

- Rust gateway for the public localhost API
- Node adapters for Codex and Copilot SDK execution

See:

- [Hybrid Architecture](/home/coemedia/Documents/code/patchhive/packages/ai-local/ARCHITECTURE.md)
- [Adapter Protocol v1](/home/coemedia/Documents/code/patchhive/packages/ai-local/contracts/adapter-v1.md)

## Endpoints

- `GET /health`
- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/responses`

The `/v1/*` routes are OpenAI-compatible enough for current PatchHive products.

`/health` is PatchHive-specific and includes adapter auth hints plus restart metadata such as `restart_count` and `last_restart_reason`.

The Node gateway supports both completion endpoints today.
The Rust gateway currently supports:

- `GET /health`
- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/responses`
- ordered provider fallback across the adapters it has available

## Run

```bash
npm install
npm run dev:ai-local

# or the hybrid Rust gateway
npm run dev:ai-local-rust
```

The gateway listens on `http://127.0.0.1:8787` by default, so PatchHive products should use:

```bash
PATCHHIVE_AI_URL=http://127.0.0.1:8787/v1
```

RepoReaper also accepts `OPENAI_BASE_URL`, but `PATCHHIVE_AI_URL` is the preferred PatchHive-wide name.

## Environment

- `PATCHHIVE_AI_HOST`
- `PATCHHIVE_AI_PORT`
- `PATCHHIVE_AI_PROVIDER_ORDER`
- `PATCHHIVE_AI_TIMEOUT_MS`
- `PATCHHIVE_AI_CODEX_MODEL`
- `PATCHHIVE_AI_COPILOT_MODEL`
- `PATCHHIVE_AI_COPILOT_GITHUB_TOKEN`
- `PATCHHIVE_AI_COPILOT_USE_LOGGED_IN_USER`
- `PATCHHIVE_AI_COPILOT_HOME`
- `PATCHHIVE_AI_COPILOT_CACHE_HOME`
- `PATCHHIVE_AI_COPILOT_CONFIG_DIR`
- `PATCHHIVE_AI_COPILOT_CLI_PATH`
- `PATCHHIVE_AI_WORKDIR`
- `PATCHHIVE_AI_ENABLE_COPILOT`

Defaults:

- provider order: `codex,copilot`
- port: `8787`
- Codex model: `gpt-5.4`
- Copilot model: `gpt-5`
- Copilot auth mode: logged-in user when no explicit token is provided

## Notes

- This package is designed for local and self-hosted use.
- Requests are non-streaming right now.
- When PatchHive routes through this gateway, the product should still keep a normal API-key fallback available for unattended jobs.

## Copilot Setup

Two supported patterns:

- Logged-in user mode:
  - `export PATCHHIVE_AI_COPILOT_HOME="$HOME/.patchhive/copilot"`
  - `npx copilot login`
- Explicit token mode:
  - `export PATCHHIVE_AI_COPILOT_GITHUB_TOKEN=...`
  - optional: `export PATCHHIVE_AI_COPILOT_USE_LOGGED_IN_USER=false`

If you are running in a sandboxed or read-only environment, set `PATCHHIVE_AI_COPILOT_HOME` to a writable directory so the Copilot CLI can persist auth state.

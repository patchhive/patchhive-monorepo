# Rust Gateway

This directory is the future home of the Rust implementation of the PatchHive local AI gateway.

It should:

- expose the product-facing localhost HTTP API
- manage adapter child processes
- route requests across providers
- keep PatchHive products insulated from provider-specific SDKs

It should not:

- implement Codex auth
- implement Copilot auth
- embed provider SDK logic directly

See:

- [Hybrid Architecture](/home/coemedia/Documents/code/patchhive/packages/ai-local/ARCHITECTURE.md)
- [Adapter Protocol v1](/home/coemedia/Documents/code/patchhive/packages/ai-local/contracts/adapter-v1.md)

## Current State

The first slice is now started:

- `GET /health`
- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/responses`
- spawns the Node Codex and Copilot adapters over stdin/stdout
- fallback routing in provider order, such as `codex -> copilot`
- automatic adapter restart on unexpected child-process exits or transport failures
- `/health` includes adapter restart counts and last restart reasons

What is still pending:

- better provider-specific error classification
- richer fallback policy controls beyond simple provider order

The Copilot adapter now supports:

- logged-in user auth via `npx copilot login`
- explicit token auth via `PATCHHIVE_AI_COPILOT_GITHUB_TOKEN`
- writable auth/state dirs via `PATCHHIVE_AI_COPILOT_HOME` and `PATCHHIVE_AI_COPILOT_CACHE_HOME`
- clearer `/health` hints when Copilot is unavailable

Run it with:

```bash
npm install
npm run dev:ai-local-rust
```

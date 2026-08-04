# Codex Adapter

This directory holds the Codex adapter for `@patchhive/ai-local`.

Its role is to wrap the official Codex path behind the shared adapter protocol so PatchHive products can route local requests through one stable gateway instead of learning Codex-specific auth and transport behavior themselves.

## Responsibilities

- start and reuse the local Codex client
- authenticate through the user's existing Codex or ChatGPT login
- probe that login through `codex login status` without reading or returning
  Codex credential files
- execute completion requests
- translate SDK results into the shared adapter protocol

Primary dependency:

- `@openai/codex-sdk`

Use `npm run auth:codex`, or `npm run auth:codex:device` on a headless host.
Codex owns its OAuth credentials and refresh lifecycle. The adapter exposes only
typed, redacted auth evidence and must not reimplement the OAuth flow.

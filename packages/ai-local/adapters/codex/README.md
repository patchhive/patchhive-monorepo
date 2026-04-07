# Codex Adapter

This directory is the future home of the Node adapter that wraps the official Codex SDK.

Responsibilities:

- start and reuse the local Codex client
- authenticate through the user's existing Codex / ChatGPT login
- execute completion requests
- translate SDK results into the shared adapter protocol

Primary dependency:

- `@openai/codex-sdk`

The adapter should communicate with the Rust gateway over the protocol described in:

- [Adapter Protocol v1](/home/coemedia/Documents/code/patchhive/packages/ai-local/contracts/adapter-v1.md)

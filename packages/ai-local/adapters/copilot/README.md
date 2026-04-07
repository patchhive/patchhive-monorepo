# Copilot Adapter

This directory is the future home of the Node adapter that wraps the official GitHub Copilot SDK.

Responsibilities:

- start and reuse the local Copilot client
- authenticate through the user's GitHub / Copilot login
- execute completion requests
- translate SDK results into the shared adapter protocol

Current auth/bootstrap knobs:

- `PATCHHIVE_AI_COPILOT_GITHUB_TOKEN`
- `PATCHHIVE_AI_COPILOT_USE_LOGGED_IN_USER`
- `PATCHHIVE_AI_COPILOT_HOME`
- `PATCHHIVE_AI_COPILOT_CACHE_HOME`
- `PATCHHIVE_AI_COPILOT_CONFIG_DIR`
- `PATCHHIVE_AI_COPILOT_CLI_PATH`

The adapter prefers logged-in user auth by default and returns bootstrap hints through the shared `health` payload when Copilot is unavailable.

Primary dependencies:

- `@github/copilot-sdk`
- `@github/copilot`

The adapter should communicate with the Rust gateway over the protocol described in:

- [Adapter Protocol v1](/home/coemedia/Documents/code/patchhive/packages/ai-local/contracts/adapter-v1.md)

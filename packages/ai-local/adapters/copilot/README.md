# Copilot Adapter

This directory holds the Copilot adapter for `@patchhive/ai-local`.

Its job is to wrap the official GitHub Copilot path behind the shared adapter protocol so PatchHive products can use Copilot through one stable localhost gateway.

## Responsibilities

- start and reuse the local Copilot client
- authenticate through the user's GitHub or Copilot login
- execute completion requests
- translate SDK results into the shared adapter protocol

## Key Auth Controls

- `PATCHHIVE_AI_COPILOT_GITHUB_TOKEN`
- `PATCHHIVE_AI_COPILOT_USE_LOGGED_IN_USER`
- `PATCHHIVE_AI_COPILOT_HOME`
- `PATCHHIVE_AI_COPILOT_CACHE_HOME`
- `PATCHHIVE_AI_COPILOT_CONFIG_DIR`
- `PATCHHIVE_AI_COPILOT_CLI_PATH`

Primary dependencies:

- `@github/copilot-sdk`
- `@github/copilot`

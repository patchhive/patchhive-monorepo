# Rust Gateway

This directory contains the Rust public edge for the PatchHive local AI gateway.

Its responsibility is to expose the product-facing localhost HTTP API, supervise adapter processes, and route requests across available providers while keeping PatchHive products insulated from provider-specific SDK behavior.

## Current Scope

- `GET /health`
- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/responses`
- adapter supervision over stdin and stdout
- ordered provider fallback such as `codex -> copilot`
- adapter restart tracking and health reporting

## Design Boundary

The Rust gateway should:

- expose the stable localhost HTTP interface
- manage adapter process lifecycle
- route requests and fallback behavior

It should not:

- implement Codex auth directly
- implement Copilot auth directly
- embed provider SDK logic

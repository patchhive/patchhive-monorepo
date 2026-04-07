# Hybrid Architecture

This package currently ships as a working Node-only localhost gateway.

The long-term target is a hybrid layout:

- Rust owns the product-facing gateway
- Node owns the official Codex and Copilot SDK adapters
- PatchHive products keep talking to one stable localhost API

That gives PatchHive a Rust service surface without giving up the official SDKs that currently exist only in the Node/TypeScript ecosystem.

## Target Layout

```text
packages/ai-local/
  src/                         current working Node gateway
  contracts/
    adapter-v1.md             JSON contract between Rust and Node adapters
  rust-gateway/
    README.md                 responsibility and implementation notes
  adapters/
    codex/
      README.md               Node adapter using @openai/codex-sdk
    copilot/
      README.md               Node adapter using @github/copilot-sdk
```

## Responsibilities

### Rust Gateway

Future home: `packages/ai-local/rust-gateway/`

Owns:

- `GET /health`
- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/responses`
- config loading
- request normalization
- provider routing and fallback
- retries and timeouts
- structured logs
- request IDs
- metrics
- adapter child-process lifecycle

Should be built with:

- `axum`
- `tokio`
- `serde`
- `serde_json`
- `tower`
- `tracing`

Should not own:

- Codex OAuth flow
- Copilot login flow
- provider-specific SDK logic

### Node Codex Adapter

Future home: `packages/ai-local/adapters/codex/`

Owns:

- local Codex SDK startup
- user login/session reuse through the official Codex auth flow
- model execution
- converting SDK results into the shared adapter protocol

Should depend on:

- `@openai/codex-sdk`

### Node Copilot Adapter

Future home: `packages/ai-local/adapters/copilot/`

Owns:

- local Copilot SDK startup
- GitHub/Copilot logged-in user flow
- model execution
- converting SDK results into the shared adapter protocol

Should depend on:

- `@github/copilot-sdk`
- `@github/copilot`

## Process Model

The Rust gateway should spawn adapters as child processes and keep them long-lived.

Recommended shape:

1. Rust boots.
2. Rust starts `codex-adapter` and `copilot-adapter`.
3. Each adapter announces readiness.
4. Rust sends JSON requests over stdin/stdout.
5. Rust tracks health, restart counts, and timeouts.
6. If an adapter fails, Rust either restarts it or falls through to the next provider.

Why child processes instead of separate local HTTP servers:

- fewer ports
- simpler local security story
- easier lifecycle management
- simpler logs
- avoids one more network hop

## External API

PatchHive products should keep using the same public localhost API:

- `GET /health`
- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/responses`

That means RepoReaper and future products do not care whether the gateway is Node-only today or Rust+Node later.

## Internal Adapter Protocol

The Rust gateway and Node adapters should speak newline-delimited JSON messages.

Contract lives in [adapter-v1.md](/home/coemedia/Documents/code/patchhive/packages/ai-local/contracts/adapter-v1.md).

Core methods:

- `initialize`
- `health`
- `list_models`
- `complete`
- `shutdown`

This keeps the seam small enough that we can swap implementations without moving the product-facing API.

## Request Flow

Example `chat/completions` request flow:

1. Product sends OpenAI-compatible request to Rust gateway.
2. Rust normalizes it into a provider-agnostic internal request.
3. Rust chooses provider order, such as `codex -> copilot`.
4. Rust sends `complete` to `codex-adapter`.
5. If Codex succeeds, Rust maps the result back to OpenAI-compatible JSON and returns it.
6. If Codex fails, Rust records the failure and tries `copilot-adapter`.

## Suggested Rust Types

```rust
struct CompletionRequest {
    model: Option<String>,
    messages: Vec<Message>,
    timeout_ms: Option<u64>,
    metadata: RequestMetadata,
}

struct CompletionResult {
    provider: String,
    model: String,
    text: String,
    usage: Option<TokenUsage>,
}

struct AdapterError {
    code: String,
    message: String,
    retryable: bool,
}
```

## Migration Stages

### Stage 1

Keep the current working Node gateway in `src/`.

### Stage 2

Extract the current Codex and Copilot logic into adapter-shaped Node modules without changing behavior.

### Stage 3

Build the Rust gateway in parallel behind a dev-only command such as:

```bash
cargo run --manifest-path packages/ai-local/rust-gateway/Cargo.toml
```

Status:

- implemented for `GET /health`
- implemented for `GET /v1/models`
- implemented for `POST /v1/chat/completions`
- implemented for `POST /v1/responses`
- implemented for multi-provider fallback
- implemented for Copilot auth/bootstrap hints and env-driven auth config
- implemented for automatic adapter restart and restart metadata in `/health`
- still pending for production-hardening work

### Stage 4

Switch the package entrypoint from the Node monolith to the Rust gateway once:

- health checks work
- adapter restart logic works
- `/v1/chat/completions` parity is verified
- RepoReaper runs unchanged against it

## What We Should Not Do

- Reimplement Codex auth in Rust
- Reimplement Copilot auth in Rust
- Make each PatchHive product know about multiple providers directly
- Expose the adapter processes as public network services

## First Implementation Slice

The first real step should be:

1. keep the current Node gateway as the compatibility baseline
2. define the adapter protocol
3. build the Rust gateway with only `GET /health` and `GET /v1/models`
4. point it at one Node adapter first, ideally Codex

That is the smallest slice that proves the seam without risking the whole package.

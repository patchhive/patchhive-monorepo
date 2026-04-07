# Adapter Protocol v1

This document defines the internal protocol between the future Rust gateway and the Node provider adapters.

Transport:

- child process stdin/stdout
- newline-delimited JSON
- one request or response per line

## Goals

- simple to implement in Rust and Node
- stable enough to version
- expressive enough for health, model listing, and completions

## Request Envelope

```json
{
  "jsonrpc": "2.0",
  "id": "req_123",
  "method": "complete",
  "params": {}
}
```

## Response Envelope

```json
{
  "jsonrpc": "2.0",
  "id": "req_123",
  "result": {}
}
```

## Error Envelope

```json
{
  "jsonrpc": "2.0",
  "id": "req_123",
  "error": {
    "code": "provider_unavailable",
    "message": "Codex is not logged in",
    "retryable": false
  }
}
```

## Methods

### `initialize`

Sent once after process spawn.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": "init_1",
  "method": "initialize",
  "params": {
    "adapter": "codex",
    "protocol_version": 1
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": "init_1",
  "result": {
    "adapter": "codex",
    "protocol_version": 1,
    "ready": true
  }
}
```

### `health`

Request:

```json
{
  "jsonrpc": "2.0",
  "id": "health_1",
  "method": "health",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": "health_1",
  "result": {
    "ok": true,
    "adapter": "copilot",
    "logged_in": true
  }
}
```

### `list_models`

Request:

```json
{
  "jsonrpc": "2.0",
  "id": "models_1",
  "method": "list_models",
  "params": {}
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": "models_1",
  "result": {
    "models": [
      "gpt-5.4",
      "gpt-5.4-mini",
      "gpt-5.3-codex"
    ]
  }
}
```

### `complete`

Request:

```json
{
  "jsonrpc": "2.0",
  "id": "complete_1",
  "method": "complete",
  "params": {
    "model": "gpt-5.4",
    "messages": [
      { "role": "system", "content": "You are a coding agent." },
      { "role": "user", "content": "Summarize this diff." }
    ],
    "timeout_ms": 120000,
    "metadata": {
      "request_id": "ph_req_123",
      "product": "repo-reaper"
    }
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": "complete_1",
  "result": {
    "provider": "codex",
    "model": "gpt-5.4",
    "text": "Summary text here",
    "usage": {
      "input_tokens": 100,
      "cached_input_tokens": 0,
      "output_tokens": 50
    }
  }
}
```

### `shutdown`

Optional graceful stop.

## Message Content Rules

- `messages` must stay close to OpenAI chat message shape
- adapters may flatten content for SDK-specific calls
- text-only support is enough for v1
- streaming is out of scope for v1

## Error Codes

Recommended initial codes:

- `provider_unavailable`
- `not_logged_in`
- `timeout`
- `invalid_request`
- `internal_error`

## Versioning

- any breaking protocol change bumps `protocol_version`
- adapters should reject unknown future versions explicitly

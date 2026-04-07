# PatchHive Product API Contract v1

This is the first shared contract target for PatchHive product backends.

It does not require immediate rewrites of existing products, but all new products should start here and existing products should move toward it over time.

## Response Envelope

Successful responses should follow:

```json
{
  "status": "ok",
  "data": {},
  "error": null,
  "meta": {
    "product": "repo-reaper",
    "version": "0.1.0",
    "request_id": "req_01K4Y8Z6Y7VQ8WQ4N7T4P8F6S2",
    "timestamp": "2026-04-07T21:30:00Z"
  }
}
```

Error responses should follow:

```json
{
  "status": "error",
  "data": null,
  "error": {
    "code": "authentication_required",
    "message": "GitHub token is missing.",
    "retryable": false,
    "details": {}
  },
  "meta": {
    "product": "repo-reaper",
    "version": "0.1.0",
    "request_id": "req_01K4Y8Z6Y7VQ8WQ4N7T4P8F6S2",
    "timestamp": "2026-04-07T21:30:00Z"
  }
}
```

## ID Formats

Use stable prefixes so HiveCore and logs can infer intent quickly:

- `request_id`: `req_<id>`
- `run_id`: `run_<id>`
- `job_id`: `job_<id>`
- `event_id`: `evt_<id>`

Guidance:

- Prefer UUIDv7 or ULID where available.
- IDs should be globally unique, not per-process counters.
- Long-running SSE or webhook flows should carry the same `run_id` throughout the lifecycle.

## Shared Error Shape

Every product should expose:

- `code`
  Stable machine-readable snake_case string.
- `message`
  Human-readable explanation.
- `retryable`
  Whether automatic retry is reasonable.
- `details`
  Optional structured payload.

Suggested common codes:

- `invalid_request`
- `authentication_required`
- `authorization_failed`
- `rate_limited`
- `provider_unavailable`
- `quality_gate_failed`
- `repo_opted_out`
- `repo_denied`
- `budget_exceeded`
- `concurrency_conflict`
- `internal_error`

## Async Run Lifecycle

All long-running product operations should converge on the same lifecycle vocabulary:

- `queued`
- `running`
- `completed`
- `failed`
- `cancelled`

Products may add phase detail inside metadata, but the top-level lifecycle should stay stable.

Recommended phase names:

- `discover`
- `triage`
- `plan`
- `patch`
- `validate`
- `submit`
- `cleanup`

## SSE / Event Shape

SSE and other event streams should expose a consistent payload shape:

```json
{
  "event_id": "evt_01K4Y92A3B1M3A9JX9Y3R5JH0D",
  "run_id": "run_01K4Y91P8V3M0M7XJZQ8Q4V4FP",
  "job_id": "job_01K4Y91T6V0Z5R7P9XJ1C2A4BQ",
  "status": "running",
  "phase": "discover",
  "timestamp": "2026-04-07T21:30:05Z",
  "data": {}
}
```

## Webhook / Async Callback Rules

- Webhooks and scheduled jobs should create the same `run_id` style as manual runs.
- Async callbacks should be idempotent when possible.
- State changes should be inspectable later through a normal history or runs endpoint.
- HiveCore should be able to poll or subscribe without product-specific translation glue.

## Initial Adoption Order

1. New products should start with this envelope and ID style.
2. RepoReaper should adopt shared IDs and error envelopes when it next touches its public API.
3. HiveCore should assume this contract and push lagging products toward it over time.

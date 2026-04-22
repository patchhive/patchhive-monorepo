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

## Standalone / HiveCore Boundary

Every product must remain independently runnable from its standalone repository. HiveCore is not a required runtime dependency.

When HiveCore is enabled, products should expose enough lifecycle metadata for HiveCore to become the suite front door and operator:

- discover what the product can do
- launch or deep-link into the product UI
- start product work through product-owned APIs
- list and inspect product-owned run history
- apply optional suite-level settings when the product supports it

HiveCore should consume these APIs. It should not reach into product databases or private implementation details.

## HiveCore Action Dispatch

HiveCore launches product work through advertised `/capabilities` actions instead of hard-coded product routes.

HiveCore accepts:

```http
POST /products/:slug/actions/:action_id
```

Request body options:

```json
{
  "payload": { "repo": "owner/repo" },
  "path_params": { "name": "daily" },
  "query": { "dry": "true" }
}
```

If the body does not include `payload`, `path_params`, or `query`, HiveCore treats the entire JSON object as the product payload.

Rules:

- HiveCore must fetch the target product's `/capabilities` and only dispatch actions the product advertises.
- HiveCore should use product-owned API keys stored in HiveCore settings; it should not expose those keys back to the frontend.
- HiveCore should record every dispatch attempt with target URL, action ID, remote status, response body, and error text.
- HiveCore should block advertised destructive actions until an explicit approval flow exists.
- Products own request validation, side effects, and run history after dispatch.

## Required Integration Endpoints

Every product should expose these endpoints in addition to any product-specific routes:

- `GET /health`
  Runtime health, version, auth state, database state, and product-specific summary fields.
- `GET /startup/checks`
  Startup/configuration checks using `StartupCheck` levels from `patchhive-product-core`.
- `GET /capabilities`
  Product metadata, HiveCore lifecycle support, exposed product actions, and useful links.
- `GET /runs`
  Normalized recent product work, backed by the product's own history store.
- `GET /runs/:id`
  Product-owned detail for one run. This may return the same payload as an existing `/history/:id` route.

Optional:

- `POST|PUT /settings/apply`
  Apply HiveCore-provided suite defaults when a product intentionally supports remote settings.

`/capabilities` should follow:

```json
{
  "schema_version": "patchhive.product.contract.v1",
  "product_slug": "signal-hive",
  "display_name": "SignalHive",
  "version": "0.1.0",
  "standalone": true,
  "hivecore": {
    "can_launch": true,
    "can_start_runs": true,
    "can_list_runs": true,
    "can_read_run_detail": true,
    "can_apply_settings": false
  },
  "routes": {
    "health": "/health",
    "startup_checks": "/startup/checks",
    "capabilities": "/capabilities",
    "runs": "/runs",
    "run_detail_template": "/runs/{id}"
  },
  "actions": [
    {
      "id": "scan",
      "label": "Run signal scan",
      "method": "POST",
      "path": "/scan",
      "description": "Discover maintenance signals.",
      "starts_run": true,
      "destructive": false
    }
  ],
  "links": [
    { "id": "history", "label": "History", "path": "/history" }
  ]
}
```

`/runs` should follow:

```json
{
  "schema_version": "patchhive.product.contract.v1",
  "product_slug": "signal-hive",
  "runs": [
    {
      "id": "scan_123",
      "status": "completed",
      "title": "patchhive/example",
      "summary": "5 signals found",
      "created_at": "2026-04-21T10:00:00Z",
      "updated_at": "",
      "detail_path": "/runs/scan_123",
      "raw": {}
    }
  ]
}
```

The normalized fields are for HiveCore. The `raw` object preserves product-specific detail so standalone UIs and deeper integrations do not lose context.

## Initial Adoption Order

1. New products should start with this envelope and ID style.
2. All products should expose `/capabilities` and `/runs` without requiring HiveCore.
3. HiveCore should use those endpoints first, then push lagging products toward richer action/start-run contracts over time.
4. RepoReaper should adopt shared IDs and error envelopes when it next touches its public API.

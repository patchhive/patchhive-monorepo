# PatchHive Unified Backend

`patchhive-backend` is the shared PatchHive suite runtime.

The canonical source lives in the PatchHive monorepo at `services/patchhive-backend/`.
The standalone `patchhive/patchhive-unified-backend` repository is an exported
mirror target, but development should happen here first.

The long-term goal is one backend Docker image that can run either the full PatchHive suite or a single standalone product. HiveCore should be the first frontend wired to this backend, then product frontends can move over product by product.

## Runtime Modes

Suite mode:

```bash
PATCHHIVE_PRODUCTS=all cargo run
```

Product mode:

```bash
PATCHHIVE_PRODUCTS=signal-hive cargo run
```

Multiple products:

```bash
PATCHHIVE_PRODUCTS=hive-core,signal-hive,trust-gate cargo run
```

The backend listens on `127.0.0.1:8100` by default. Override it with:

```bash
PATCHHIVE_BIND_ADDR=127.0.0.1:8120 cargo run
```

The shared SQLite database defaults to `patchhive-backend.db`. Override it with:

```bash
PATCHHIVE_DB_PATH=/tmp/patchhive-backend.db cargo run
```

## First Contract

This first skeleton is intentionally control-plane-first. It gives HiveCore a stable backend to connect to before product engines are migrated.

Routes:

- `GET /health`
- `GET /api/health`
- `GET /api/auth/status`
- `GET /api/auth/session`
- `GET /api/products`
- `GET /api/products/:product_key/health`
- `GET /api/setup/first-stack`
- `POST /api/setup/first-stack/pair`
- `GET /api/runs`
- `GET /api/events`

MergeKeeper, ReleaseSentry, DepTriage, and VulnTriage are mounted as in-process
product engines under `/api/products/merge-keeper/*`,
`/api/products/release-sentry/*`, `/api/products/dep-triage/*`, and
`/api/products/vuln-triage/*`. Other products still use their existing backend
engines until they are moved into this runtime or temporarily connected through
gateway routes.

## Product Registry

Product registration lives in `registry/products/*.toml`. The backend embeds
these manifests at compile time and exposes them through `GET /api/products` so
HiveCore does not need to hardcode product wiring.

Each manifest declares:

- `key`, `code`, `name`, and `role` for product identity.
- `module_path` for the eventual in-process Rust product module.
- `route_prefix` for the product-owned API namespace.
- `migration_stage` so HiveCore can tell shell, gateway, and integrated products apart.
- `[[capabilities]]` entries with `id`, `label`, `description`, and optional `mutating`.
- `[safety]` boundaries such as read-only status, external writes, repo mutation, approval requirements, credential scopes, and required evidence.
- Optional `[gateway]` settings with `default_url` and `env_var` while a product still runs in its existing backend.
- `[health]` settings with the suite endpoint, timeout budget, and expected healthy status.
- `[[routes]]` claims with method, path, and description.

Example:

```toml
key = "signal-hive"
code = "SH"
name = "SignalHive"
role = "maintenance signal reconnaissance"
module_path = "crate::products::signal_hive"
route_prefix = "/api/products/signal-hive"
migration_stage = "not-started"

[safety]
read_only = true
credential_scopes = ["github:repo:read", "github:issues:read"]
evidence_required = ["scan parameters", "repo sample list"]

[gateway]
default_url = "http://127.0.0.1:8010"
env_var = "SIGNAL_HIVE_GATEWAY_URL"

[health]
endpoint = "/api/products/signal-hive/health"
timeout_ms = 2000
healthy_status = 200

[[capabilities]]
id = "signal-scan"
label = "Signal scan"
description = "Scan repos for maintenance pressure."

[[routes]]
method = "POST"
path = "/api/products/signal-hive/scan"
description = "Start a maintenance signal scan."
```

MergeKeeper, ReleaseSentry, DepTriage, VulnTriage, FlakeSting, and ReviewBee are mounted in-process
from their product backend libraries. The manifest contract also drives gateway
dispatch and is the shape future in-process mounting should use.

Run the suite backend with only MergeKeeper enabled:

```bash
PATCHHIVE_PRODUCTS=merge-keeper cargo run
```

MergeKeeper product routes are served directly by the unified backend:

```text
GET  /api/products/merge-keeper/health
POST /api/products/merge-keeper/assess/github/pr
GET  /api/products/merge-keeper/runs
```

Run the suite backend with only ReleaseSentry enabled:

```bash
PATCHHIVE_PRODUCTS=release-sentry cargo run
```

ReleaseSentry product routes are served directly by the unified backend:

```text
GET  /api/products/release-sentry/health
POST /api/products/release-sentry/check/github/release
GET  /api/products/release-sentry/runs
```

Run the suite backend with only FlakeSting enabled:

```bash
PATCHHIVE_PRODUCTS=flake-sting cargo run
```

FlakeSting product routes are served directly by the unified backend:

```text
GET  /api/products/flake-sting/health
POST /api/products/flake-sting/scan/github/actions
GET  /api/products/flake-sting/runs
```

Run the suite backend with only DepTriage enabled:

```bash
PATCHHIVE_PRODUCTS=dep-triage cargo run
```

DepTriage product routes are served directly by the unified backend:

```text
GET  /api/products/dep-triage/health
POST /api/products/dep-triage/scan/github/dependencies
GET  /api/products/dep-triage/runs
```

Run the suite backend with only VulnTriage enabled:

```bash
PATCHHIVE_PRODUCTS=vuln-triage cargo run
```

VulnTriage product routes are served directly by the unified backend:

```text
GET  /api/products/vuln-triage/health
POST /api/products/vuln-triage/scan/github/findings
GET  /api/products/vuln-triage/runs
```

Run the suite backend with only ReviewBee enabled:

```bash
PATCHHIVE_PRODUCTS=review-bee cargo run
```

ReviewBee product routes are served directly by the unified backend:

```text
GET  /api/products/review-bee/health
POST /api/products/review-bee/review/github/pr
GET  /api/products/review-bee/runs
```

When launching the suite backend with product `.env` files, avoid shell-sourcing
unquoted JSON service-token records such as `*_SERVICE_TOKEN_HASH={...}`.
Shell parsing can flatten the JSON and make product-core treat the value as a
legacy token string. API-key login is unaffected, but HiveCore service-token
pairing should use quoted exports, product wrapper `dotenvy` loading, or freshly
rotated scoped service tokens.

## Gateway Dispatch

Gateway dispatch lets the unified backend expose stable suite URLs while the
actual product engine still runs in its existing backend.

SignalHive is the first gateway target:

```bash
SIGNAL_HIVE_GATEWAY_URL=http://127.0.0.1:8010 \
PATCHHIVE_BIND_ADDR=127.0.0.1:8120 \
PATCHHIVE_PRODUCTS=hive-core,signal-hive \
cargo run
```

Requests under `/api/products/signal-hive/*` are validated against the
SignalHive manifest route claims. Non-health requests first run the manifest
health check, then forward to the SignalHive backend with the product prefix
stripped. For example:

```text
GET  /api/products/signal-hive/health  -> GET  http://127.0.0.1:8010/health
POST /api/products/signal-hive/scan    -> POST http://127.0.0.1:8010/scan
```

Unclaimed routes return `route-not-claimed`; disabled products return
`product-disabled`; missing gateway targets return `gateway-unconfigured`;
unhealthy gateway targets return `product-unavailable`.

## Shared DB

The backend opens one shared SQLite database and initializes these first suite
tables:

- `suite_events` for backend and orchestration events.
- `suite_runs` for a suite-wide run index.
- `product_registry_overrides` for future runtime enablement and route overrides.
- `shared_config` for future global defaults.

Product modules should add namespaced tables as they move in, such as
`signal_hive_scans` or `trust_gate_reviews`, while shared run/event indexes stay
owned by the backend.

## HiveCore v2 Smoke Test

Run the unified backend:

```bash
PATCHHIVE_BIND_ADDR=127.0.0.1:8120 PATCHHIVE_PRODUCTS=hive-core,signal-hive cargo run
```

Then run HiveCore v2 from the monorepo with:

```bash
VITE_API_URL=http://127.0.0.1:8120/api npm --prefix products/hive-core/frontend-v2 run dev
```

HiveCore should enter without API-key bootstrap and show the unified backend product registry. Product engines still report as pending until they are migrated into this backend.

## Docker Direction

Standalone product repositories should eventually use the shared image with one product enabled:

```yaml
services:
  backend:
    image: patchhive/patchhive-backend:latest
    environment:
      PATCHHIVE_PRODUCTS: signal-hive
```

The full suite should use the same image with all products enabled:

```yaml
services:
  backend:
    image: patchhive/patchhive-backend:latest
    environment:
      PATCHHIVE_PRODUCTS: all
```

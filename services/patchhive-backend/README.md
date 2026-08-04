# PatchHive Unified Backend

`patchhive-backend` is the shared PatchHive suite runtime.

The canonical source lives in the PatchHive monorepo at `services/patchhive-backend/`.
The standalone `patchhive/patchhive-unified-backend` repository is an exported
mirror target, but development should happen here first.

The backend image can run either the full PatchHive suite or a selected subset
of products. HiveCore remains the distinct control-plane product and cockpit.

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

At startup, every enabled engine issues one scoped process-local service
credential. The target retains only its hash; the raw credential is held in
redacted runtime configuration and disappears on restart. HiveCore uses these
credentials for authenticated fleet snapshots and dispatch, while direct peer
clients use them for their mounted HTTP contracts. Calls still traverse normal
product authentication, declared scopes, rate limits, and telemetry. Do not
copy these runtime credentials into `.env` or HiveCore settings.

Standalone product processes remain independent. Cross-process integrations
use explicit `PATCHHIVE_<PEER>_URL` and
`PATCHHIVE_<PEER>_SERVICE_TOKEN` configuration; an operator API key is only a
compatibility alternative.

## Container Image

Build the unified image from the monorepo root because its Cargo manifest uses
the shared crates and every mounted product backend as path dependencies:

```bash
docker build \
  --file services/patchhive-backend/Dockerfile \
  --tag patchhive-backend:local \
  .
```

The runtime is non-root, persists the canonical suite database under
`/var/lib/patchhive`, includes `git`, and carries only the Docker client needed
by RepoReaper's opt-in Docker validation path. A local-only hardened run is:

```bash
docker run --rm \
  --publish 127.0.0.1:8100:8100 \
  --env-file .env \
  --volume patchhive-data:/var/lib/patchhive \
  --cap-drop ALL \
  --security-opt no-new-privileges \
  patchhive-backend:local
```

Do not mount the Docker socket unless Docker-based validation is explicitly
enabled. If it is required, mount only that socket and add its host group ID so
the non-root runtime can reach it:

```bash
docker run --rm \
  --publish 127.0.0.1:8100:8100 \
  --env-file .env \
  --volume patchhive-data:/var/lib/patchhive \
  --volume /var/run/docker.sock:/var/run/docker.sock \
  --group-add "$(stat -c '%g' /var/run/docker.sock)" \
  --cap-drop ALL \
  --security-opt no-new-privileges \
  patchhive-backend:local
```

The backend listens on `127.0.0.1:8100` by default. Override it with:

```bash
PATCHHIVE_BIND_ADDR=127.0.0.1:8120 cargo run
```

The shared SQLite database defaults to `patchhive-backend.db`. Override it with:

```bash
PATCHHIVE_DB_PATH=/tmp/patchhive-backend.db cargo run
```

When `PATCHHIVE_DB_PATH` is set, every in-process product uses that same
backend-owned database. Product-specific database variables remain standalone
fallbacks only. This prevents the launch working directory from creating a new
set of empty product databases.

### Consolidating existing product databases

Stop the unified backend and make SQLite backups before migrating live data.
Then run:

```bash
cargo run --manifest-path services/patchhive-backend/Cargo.toml \
  --bin consolidate-databases -- \
  --target /absolute/path/to/patchhive.db
```

The importer reads the canonical databases for every currently integrated
product, preserves SignalHive history, maps RefactorScout's formerly generic
tables to `refactor_scout_*`, merges shared schedules by their product-aware
keys, and records per-table source counts in
`database_consolidation_manifest`. Re-running it is safe: new rows are imported
by their existing keys without overwriting newer consolidated state, and the
manifest is refreshed. All product engines now run in-process; importer coverage
is based on preserved standalone schema needs, not migration state.

## First Contract

The suite runtime mounts every product router and gives HiveCore one stable,
middleware-preserving control-plane boundary.

Routes:

- `GET /health`
- `GET /api/health`
- `GET /api/auth/status`
- `GET /api/auth/session`
- `GET /api/products`
- `GET /api/products/runtime`
- `GET /api/products/runs`
- `GET /api/products/:product_key/health`
- `GET /api/runs`
- `GET /api/events`

All twelve products, including RepoReaper and HiveCore, are mounted as in-process
engines under their `/api/products/<product>/*` namespaces. Setup and pairing are
owned exclusively by the mounted HiveCore router.

## Product Registry

Product registration lives in `registry/products/*.toml`. Build scripts discover
and embed those manifests, generate backend initialization/router wiring, and
generate HiveCore's compiled catalog. `GET /api/products` exposes the same source
to browser consumers, so no handwritten 12-product inventory remains.

Each manifest declares:

- `key`, `code`, `name`, and `role` for product identity.
- `module_path` for the in-process Rust product module.
- `route_prefix` for the product-owned API namespace.
- `[[capabilities]]` entries with `id`, `label`, `description`, and optional `mutating`.
- `[safety]` boundaries such as read-only status, external writes, repo mutation, approval requirements, credential scopes, and required evidence.
- `[health]` settings with the suite endpoint, timeout budget, and expected healthy status.
- `[[routes]]` claims with method, path, and description.

Example:

```toml
key = "signal-hive"
code = "SH"
name = "SignalHive"
role = "maintenance signal reconnaissance"
module_path = "signal_hive"
route_prefix = "/api/products/signal-hive"

[safety]
read_only = true
credential_scopes = ["github:repo:read", "github:issues:read"]
evidence_required = ["scan parameters", "repo sample list"]

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

Every product is mounted in-process from its backend library. The manifest
contract drives generated wiring and control-plane metadata.

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

Run the suite backend with only TrustGate enabled:

```bash
PATCHHIVE_PRODUCTS=trust-gate cargo run
```

TrustGate product routes are served directly by the unified backend:

```text
GET  /api/products/trust-gate/health
POST /api/products/trust-gate/review
POST /api/products/trust-gate/review/github/pr
GET  /api/products/trust-gate/runs
```

Run the suite backend with only RepoMemory enabled:

```bash
PATCHHIVE_PRODUCTS=repo-memory cargo run
```

RepoMemory product routes are served directly by the unified backend:

```text
GET  /api/products/repo-memory/health
POST /api/products/repo-memory/ingest
GET  /api/products/repo-memory/memories
POST /api/products/repo-memory/context
GET  /api/products/repo-memory/failguard/candidates
```

Run the suite backend with only RefactorScout enabled:

```bash
REFACTOR_SCOUT_DB_PATH=/absolute/path/to/refactor-scout.db \
PATCHHIVE_PRODUCTS=refactor-scout cargo run
```

RefactorScout product routes are served directly by the unified backend:

```text
GET  /api/products/refactor-scout/health
POST /api/products/refactor-scout/scan/local
GET  /api/products/refactor-scout/runs
GET  /api/products/refactor-scout/history
```

Run the suite backend with only RepoReaper enabled:

```bash
PATCHHIVE_PRODUCTS=repo-reaper cargo run
```

RepoReaper product routes are served directly by the unified backend. Its
product-owned write token, approval boundary, test validation, PR budgets, and
repository policy checks remain in force inside the shared process:

```text
GET  /api/products/repo-reaper/health
POST /api/products/repo-reaper/run
POST /api/products/repo-reaper/dry-run
GET  /api/products/repo-reaper/runs
GET  /api/products/repo-reaper/agents
GET  /api/products/repo-reaper/pr-tracking
```

RepoReaper stores its state in `repo_reaper_*` tables under
`PATCHHIVE_DB_PATH`. To import a preserved standalone database without
re-running the full historical consolidation:

```bash
cargo run --bin consolidate-databases -- --only repo-reaper
```

When launching the suite backend with product `.env` files, avoid shell-sourcing
unquoted JSON service-token records such as `*_SERVICE_TOKEN_HASH={...}`.
Shell parsing can flatten the JSON and make product-core treat the value as a
legacy token string. API-key login is unaffected, but HiveCore service-token
pairing should use quoted exports, product wrapper `dotenvy` loading, or freshly
rotated scoped service tokens.

## Product Invocation

HiveCore observes capabilities, auth posture, health, startup checks, and runs
through the products' mounted HTTP routers. The background poller materializes
those observations in SQLite; suite runtime, run, token, dispatch-preview, and
conformance reads use that durable snapshot. This keeps authentication, rate
limiting, telemetry, and error behavior identical to standalone product calls
without browser request fan-out or direct handler bypasses.

## Shared DB

The backend uses `patchhive_product_core::sqlite::SqlitePool` for the shared
SQLite database and initializes these suite tables:

- `suite_events` for backend and orchestration events.
- `suite_runs` for a suite-wide run index.
- `product_registry_overrides` for future runtime enablement and route overrides.
- `shared_config` for future global defaults.

Product tables are product-namespaced, such as `signal_hive_scans` and
`trust_gate_reviews`, while shared run/event indexes stay owned by the backend.

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

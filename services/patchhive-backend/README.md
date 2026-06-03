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

Product run routes are not active yet. Existing product backends remain the source of truth until each product engine is moved into this runtime or temporarily connected through gateway routes.

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

[[capabilities]]
id = "signal-scan"
label = "Signal scan"
description = "Scan repos for maintenance pressure."

[[routes]]
method = "POST"
path = "/api/products/signal-hive/scans"
description = "Start a maintenance signal scan."
```

The current registry is metadata-only. Product modules are not mounted yet, but
the manifest contract is the shape that future in-process mounting and gateway
dispatch should use.

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

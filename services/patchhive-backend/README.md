# PatchHive Unified Backend

`patchhive-backend` is the shared PatchHive suite runtime.

The canonical source lives in the PatchHive monorepo at `services/patchhive-backend/`.
The standalone `patchhive/patchhive-unified-backend` repository can become an
exported mirror later, but development should happen here first.

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

# Product Starter Workflow

PatchHive now has a real product starter so new products do not begin with a manual copy of an existing app.

The starter lives at `templates/product-starter/`, and the actual copied scaffold lives under `templates/product-starter/scaffold/`.

## Why It Exists

By the time PatchHive had RepoReaper, SignalHive, and TrustGate, the same shell was already repeating:

- Rust backend auth wiring
- startup checks
- SQLite path setup
- React app shell
- API-key auth bootstrap
- frontend checks panel
- Docker files
- standalone CI

The starter keeps that repeated shape in one place so new products begin from a consistent shell and diverge only where the product logic actually changes.

## Create A New Product

Use:

```bash
./scripts/new-product.sh <product-slug>
```

Example:

```bash
./scripts/new-product.sh review-bee --icon "🐝" --tagline "Turn review churn into concrete follow-up work."
```

The script will:

1. copy the shared starter template into `products/<product-slug>`
2. pick the next available backend and frontend ports unless you override them
3. wire in the shared PatchHive auth, UI, and CI shell
4. generate an initial backend `Cargo.lock` unless you pass `--skip-lockfile`

Useful flags:

```bash
./scripts/new-product.sh repo-memory \
  --icon "🧠" \
  --backend-port 8040 \
  --frontend-port 5177 \
  --tagline "Give coding agents memory of how your repo actually works."
```

## What The Starter Includes

- `backend/` with shared auth, startup checks, SQLite setup, and placeholder overview route
- `frontend/` with PatchHive header/footer/tab shell, API-key login, overview panel, and checks panel
- `.env.example`
- `.gitignore`
- `docker-compose.yml`
- backend and frontend Dockerfiles
- standalone GitHub Actions CI
- starter README copy

## After Scaffolding

Do these early:

1. Replace the README and overview panel copy with the real product pitch.
2. Replace the placeholder `/overview` backend route with the actual product loop.
3. Adjust startup checks so they reflect the product's real dependencies.
4. Add or confirm the product theme key in `packages/ui/src/theme.js`.
5. Commit the scaffold before the product starts drifting.

## Standalone Lockfile Helper

If a product backend depends on shared git crates such as `patchhive-product-core`, exported standalone repos need a lockfile generated outside the monorepo's local cargo patch.

Use:

```bash
./scripts/refresh-product-lockfile.sh <product-slug>
```

Example:

```bash
./scripts/refresh-product-lockfile.sh trust-gate
```

This copies the product to a temporary directory outside the monorepo, regenerates `backend/Cargo.lock`, and copies the standalone-safe lockfile back into the product directory.

Use it:

- before the first standalone export
- after shared git crate dependency changes
- any time standalone CI says `cargo check --locked` wants to update the lockfile

## Standalone Template Repo

If you want the starter itself to have its own GitHub repo mirror, use:

```bash
./scripts/export-template.sh product-starter <remote-name> main
```

For PatchHive, `patchhive-product-starter` should still be treated as a mirror of `templates/product-starter`, not as the primary editing location.

# PatchHive Product Starter

Monorepo-first starter scaffold for new PatchHive products.

This directory is the source for the shared starter used by:

```bash
./scripts/new-product.sh <product-slug>
```

## What Lives Here

- `scaffold/` contains the actual files copied into a new product
- the scaffold includes:
  - shared Rust backend auth/startup shell
  - shared React frontend shell
  - Docker files
  - API-key auth bootstrap
  - standalone GitHub Actions CI

## How It Is Used

Inside the monorepo, create a new product with:

```bash
./scripts/new-product.sh review-bee --icon "🐝" --tagline "Turn review churn into concrete follow-up work."
```

If that new backend uses shared git crates such as `patchhive-product-core`, refresh its standalone-safe lockfile before the first export:

```bash
./scripts/refresh-product-lockfile.sh review-bee
```

## Standalone Repo

`patchhive-product-starter` should be treated as an exported mirror of this directory, not the primary editing location.

The actual scaffold files live under [scaffold/](/home/coemedia/Documents/code/patchhive/templates/product-starter/scaffold).

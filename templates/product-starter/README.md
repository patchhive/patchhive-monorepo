# PatchHive Product Starter

The PatchHive Product Starter is the monorepo-first scaffold for new PatchHive products.

It gives new products the parts that should already be solved on day one: a shared frontend shell, shared backend auth and startup wiring, Docker support, API-key bootstrap, and standalone CI.

## What Lives Here

- `scaffold/` contains the files copied into each new product
- the scaffold already includes:
  - shared Rust backend auth and startup wiring
  - shared React product shell wiring
  - Docker support
  - API-key bootstrap
  - standalone GitHub Actions CI

## Usage

Create a new product from the monorepo:

```bash
./scripts/new-product.sh <product-slug>
```

Example:

```bash
./scripts/new-product.sh review-bee --icon "🐝" --tagline "Turn review churn into a concrete merge checklist."
```

## Export Model

The PatchHive monorepo is the source of truth for the starter. The standalone `patchhive/patchhive-product-starter` repository is an exported mirror.

If the scaffold's shared Rust git dependencies change, refresh its standalone-safe lockfile with:

```bash
./scripts/refresh-template-lockfile.sh product-starter
```

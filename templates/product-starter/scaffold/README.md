# __PRODUCT_ICON__ __PRODUCT_TITLE__ by PatchHive

> __PRODUCT_TAGLINE__

__PRODUCT_TITLE__ starts from the shared PatchHive product starter. Replace this copy with the real pitch, operating model, and product boundary once the product direction settles.

## What It Does

- uses the shared PatchHive frontend shell and API-key auth flow
- uses the shared Rust auth and startup primitives from `patchhive-product-core`
- exposes a placeholder backend route and a basic frontend overview/checks loop
- gives new products a consistent starting point before the product-specific logic exists

## Quick Start

```bash
cp .env.example .env

# Backend
cd backend && cargo run

# Frontend
cd ../frontend && npm install && npm run dev
```

Backend: `http://localhost:__BACKEND_PORT__`
Frontend: `http://localhost:__FRONTEND_PORT__`

## Local Run Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend stores starter state in SQLite at `__ENV_PREFIX___DB_PATH`.
- Replace the overview route, startup checks, and frontend copy as the real product logic appears.

## Standalone Repo Notes

__PRODUCT_TITLE__ should be developed in the PatchHive monorepo first. When it gets its own repository later, that standalone repo should be treated as an exported mirror of this product directory rather than a second source of truth.

*__PRODUCT_TITLE__ by PatchHive — __PRODUCT_TAGLINE__*

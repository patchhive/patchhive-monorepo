# __PRODUCT_TITLE__ by PatchHive

__PRODUCT_TAGLINE__

__PRODUCT_TITLE__ starts from the shared PatchHive product starter. Replace this README with the product's real positioning, workflow, and setup notes once the product direction settles, but keep the same professional repo shape.

## What The Starter Gives You

- canonical specialist UI shell and API-key auth flow
- a mountable Rust engine plus thin standalone launcher using shared auth,
  startup, SQLite, environment, and rate-limit primitives
- placeholder workspace, checks, sources, health, and overview surfaces that
  must be replaced when the real product loop lands
- Docker setup and standalone CI from day one

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:__FRONTEND_PORT__`
Backend: `http://localhost:__BACKEND_PORT__`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Local Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend uses `PATCHHIVE_DB_PATH` in suite mode and
  `__ENV_PREFIX___DB_PATH` as a standalone compatibility override.
- Register and mount the engine in `services/patchhive-backend` before treating
  the product as integrated.
- If the product talks to GitHub, prefer a classic personal access token.
- Keep repository access public-only unless the product truly needs private repositories.
- Generate the first local API key from `http://localhost:__FRONTEND_PORT__`.
- If remote bootstrap is intentional, set `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP=true`.

## Repository Model

__PRODUCT_TITLE__ should be developed in the PatchHive monorepo first. The standalone repository, when exported, should be treated as a mirror of this directory rather than a second source of truth.

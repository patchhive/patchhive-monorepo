# DepTriage by PatchHive

DepTriage tells teams which dependency updates matter now and which ones can wait.

It is PatchHive's dependency-noise filter: a product that reads open dependency pull requests, optionally folds in Dependabot alerts, groups that activity by package, and turns the result into a ranked queue such as `update now`, `watch`, or `ignore for now`.

## Core Workflow

- read open dependency pull requests for a target repository
- optionally read matching Dependabot alerts
- group overlapping pull requests and alerts by package
- rank each package into a practical action bucket
- save scan history so teams can compare or revisit earlier triage decisions

DepTriage is intentionally read-only in the MVP. It does not merge dependency pull requests or rewrite update configuration.

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5180`
Backend: `http://localhost:8070`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## GitHub Access

DepTriage works best with a fine-grained personal access token.

- If you only want public repositories, keep the token public-only.
- Reading pull requests is enough for the base product loop.
- Dependabot alert reads need the matching security permission in GitHub.
- If Dependabot access is unavailable, DepTriage can still score dependency pull requests and show the limitation clearly.

## Local Notes

- The backend stores scan history in SQLite at `DEP_TRIAGE_DB_PATH`.
- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The product does not require AI for the MVP loop.
- Generate the first local API key from `http://localhost:5180`.

## Repository Model

The PatchHive monorepo is the source of truth for DepTriage development. The standalone `patchhive/deptriage` repository is an exported mirror of this directory.

# HiveCore by PatchHive

Control the PatchHive suite from one clear surface.

HiveCore is the PatchHive control plane. The first MVP does three things well:

- keeps a live registry of every PatchHive product
- polls each product's `/health`, `/startup/checks`, `/capabilities`, and `/runs` endpoints
- stores suite-wide defaults and per-product launch overrides in one place

This is intentionally narrower than full orchestration. HiveCore should earn that role by first making the suite visible, configurable, and operationally legible.

## What It Covers Today

- suite overview with quick launch links
- live product health polling across the PatchHive product catalog
- product-owned run history surfaced through each product's `/runs` contract
- capability-driven action dispatch through advertised product actions
- persistent global defaults for topics, languages, repo guardrails, and operator notes
- per-product frontend/API overrides for subdomains or remote deployments
- per-product API keys stored server-side for protected `/runs` reads and action dispatch
- shared PatchHive API-key bootstrap flow

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5183`
Backend: `http://localhost:8100`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Product Registry Defaults

HiveCore starts with built-in localhost defaults for the current PatchHive suite:

- RepoReaper: frontend `http://localhost:5173`, API `http://localhost:8000`
- SignalHive: frontend `http://localhost:5174`, API `http://localhost:8010`
- TrustGate: frontend `http://localhost:5175`, API `http://localhost:8020`
- RepoMemory: frontend `http://localhost:5176`, API `http://localhost:8030`
- ReviewBee: frontend `http://localhost:5177`, API `http://localhost:8040`
- MergeKeeper: frontend `http://localhost:5178`, API `http://localhost:8050`
- FlakeSting: frontend `http://localhost:5179`, API `http://localhost:8060`
- DepTriage: frontend `http://localhost:5180`, API `http://localhost:8070`
- VulnTriage: frontend `http://localhost:5181`, API `http://localhost:8080`
- RefactorScout: frontend `http://localhost:5182`, API `http://localhost:8090`
- HiveCore: frontend `http://localhost:5183`, API `http://localhost:8100`

If you run products on subdomains or remote hosts, save the new targets in HiveCore's Settings tab. Those overrides persist in the HiveCore SQLite database.

HiveCore does not read product databases. It uses product-owned APIs and saved product API keys, so each product remains independently runnable and keeps ownership of its own run history and validation.

## Local Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend stores state in SQLite at `HIVE_CORE_DB_PATH`.
- Generate the first local API key from `http://localhost:5183`.
- Save per-product API keys in Settings if you want HiveCore to show protected run history or dispatch product actions.
- If remote bootstrap is intentional, set `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP=true`.

## Repository Model

HiveCore should be developed in the PatchHive monorepo first. The standalone repository should mirror this directory rather than becoming a second source of truth.

# HiveCore by PatchHive

Control the PatchHive suite from one clear surface.

HiveCore is the PatchHive control plane. The first MVP does three things well:

- keeps a live registry of every PatchHive product
- polls each product's `/health`, `/startup/checks`, `/capabilities`, and `/runs` endpoints
- stores suite-wide defaults and per-product launch overrides in one place

This is intentionally narrower than full orchestration. HiveCore should earn that role by first making the suite visible, configurable, and operationally legible.

## Product Documentation

- GitHub-facing product doc: [docs/products/hive-core.md](../../docs/products/hive-core.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)
- First-stack readiness audit: [docs/hivecore-first-stack-readiness.md](../../docs/hivecore-first-stack-readiness.md)

## Core Workflow

- keep the PatchHive product catalog visible in one place
- poll product-owned health, startup checks, capabilities, runs, and run details
- store suite-wide defaults and per-product endpoint overrides
- store per-product access tokens server-side for protected reads and action dispatch
- dispatch only advertised product actions through the shared capability contract
- report contract drift instead of hiding product API differences

## What It Covers Today

- suite overview with quick launch links
- live product health polling across the PatchHive product catalog
- product-owned run history surfaced through each product's `/runs` contract
- server-side product run detail drill-downs through each product's `/runs/:id` contract
- per-product contract drift reporting for health, startup checks, capabilities, run lists, and run detail support
- capability-driven action dispatch through advertised product actions
- persistent global defaults for topics, languages, repo guardrails, and operator notes
- per-product frontend/API overrides for subdomains or remote deployments
- per-product access tokens stored server-side for protected `/runs` reads and action dispatch
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

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | Optional GitHub token reserved for future control-plane reads. |
| `HIVE_CORE_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `HIVE_CORE_SERVICE_TOKEN_HASH` | Optional service-token hash for HiveCore as a machine caller. |
| `HIVE_CORE_DB_PATH` | SQLite path for suite settings, product overrides, and action events. |
| `HIVE_CORE_PORT` | Backend port for split local runs. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Allows first-time key bootstrap from non-localhost clients. Keep unset for local use. |
| `RUST_LOG` | Rust logging level. |

To reuse the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root before starting the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh`. Once the hash is pre-seeded, HiveCore can be used through a subdomain without remote bootstrap.

Save per-product access tokens in Settings when you want HiveCore to read protected `/runs` data or dispatch advertised product actions. Dedicated service tokens are preferred; legacy product API keys still work during the transition. Those tokens stay server-side.

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

HiveCore does not read product databases. It uses product-owned APIs and saved product access tokens, so each product remains independently runnable and keeps ownership of its own run history and validation.

## Safety Boundary

HiveCore is a control plane, not a replacement runtime for products. It does not read private product databases, bypass product auth, or dispatch destructive actions without explicit product capability support and approval flow. Each product remains standalone.

## HiveCore Fit

HiveCore is the suite fit layer. It brings standalone products into one operator surface for health, launch links, shared defaults, run history, action dispatch, and contract drift. Deeper orchestration should build on shared product APIs, not private implementation shortcuts.

## Local Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- Generate the first local API key from `http://localhost:5183`.

## Standalone Repository

HiveCore should be developed in the PatchHive monorepo first. The standalone [`patchhive/hivecore`](https://github.com/patchhive/hivecore) repository should mirror this directory rather than becoming a second source of truth.

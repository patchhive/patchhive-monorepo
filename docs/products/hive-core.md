# HiveCore

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

HiveCore is the PatchHive control plane. It brings standalone PatchHive products into one operational interface for health, launch links, shared defaults, run history, capability visibility, and product handoffs.

## Product Role

HiveCore is not a replacement for standalone products. Its first job is to make the suite legible: what is running, what is healthy, what capabilities exist, what work has happened, and where product contracts have drifted.

## Core Workflow

1. Keep the PatchHive product catalog visible in one place
2. Poll product-owned health, startup checks, capabilities, runs, and run details
3. Store suite-wide defaults and per-product endpoint overrides
4. Provision and store per-product service tokens server-side for protected reads and action dispatch
5. Adapt to already-running first-stack products and pair with them automatically when suite bootstrap is configured
6. Delegate local Docker start/stop control to `patchhive-launcher` instead of doing host control from the browser
7. Dispatch only advertised product actions through the shared capability contract
8. Report contract drift instead of hiding product API differences

## What It Covers Today

- Suite overview with quick launch links
- Live product health polling across the PatchHive product catalog
- Product-owned run history surfaced through each product's `/runs` contract
- Server-side product run detail drill-downs through each product's `/runs/:id` contract
- Per-product contract drift reporting for health, startup checks, capabilities, run lists, and run detail support
- Capability-driven action dispatch through advertised product actions
- Persistent global defaults for topics, languages, repo guardrails, and operator notes
- Per-product frontend/API overrides for subdomains or remote deployments
- One-time per-product service-token provisioning from HiveCore Settings
- Setup tab for the first stack that can detect already-running products, start missing ones through `patchhive-launcher`, and auto-pair HiveCore with them
- Per-product service tokens stored server-side for protected `/runs` reads and action dispatch, with optional at-rest encryption via `HIVECORE_ENCRYPTION_KEY`
- Shared PatchHive API-key bootstrap flow

## Inputs

- GitHub token (optional, for future control-plane reads)
- Suite-wide defaults (topics, languages, repo guardrails, operator notes)
- Per-product launch overrides (frontend URL, API URL)
- Product service tokens (generated via HiveCore or pre-configured)
- PatchHive launcher URL (for Docker start/stop control)
- Suite bootstrap secret (for automatic service-token pairing)

## Outputs

- Unified dashboard showing all PatchHive products
- Health status for each product (`/health` endpoint)
- Startup check visibility (`/startup/checks` endpoint)
- Capability reporting (`/capabilities` endpoint)
- Run history and details (`/runs` and `/runs/:id` endpoints)
- Contract drift reports comparing advertised vs actual capabilities
- Action dispatch through product-advertised capabilities
- Service token management and rotation
- Setup and configuration interface

## Safety Boundary

HiveCore is a control plane, not a replacement runtime for products. It does not read private product databases, bypass product auth, or dispatch destructive actions without explicit product capability support and approval flow. Each product remains standalone and independently runnable.

## Local Development

```bash
cd products/hive-core
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5183`
- Backend: `http://localhost:8100`
- Database: `HIVE_CORE_DB_PATH`

Split local workflow:
```bash
cd products/hive-core/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
|----------|---------|
| `BOT_GITHUB_TOKEN` | Optional GitHub token reserved for future control-plane reads. |
| `HIVE_CORE_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `HIVE_CORE_SERVICE_TOKEN_HASH` | Optional service-token hash for HiveCore as a machine caller. |
| `HIVE_CORE_DB_PATH` | SQLite path for suite settings, product overrides, and action events. |
| `HIVE_CORE_PORT` | Backend port for split local runs. |
| `HIVECORE_ENCRYPTION_KEY` | Encrypts saved downstream product service tokens at rest in HiveCore SQLite and auto-migrates existing plaintext rows on boot. |
| `PATCHHIVE_LAUNCHER_URL` | Base URL for the local `patchhive-launcher` service that starts or stops the first stack. |
| `PATCHHIVE_SUITE_BOOTSTRAP_SECRET` | Shared bootstrap secret HiveCore can use to rotate or provision downstream product service tokens automatically. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Allows first-time key bootstrap from non-localhost clients. Keep unset for local use. |
| `RUST_LOG` | Rust logging level. |

To reuse the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root before starting the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh`. Once the hash is pre-seeded, HiveCore can be used through a subdomain without remote bootstrap.

HiveCore Settings can now provision or rotate a dedicated service token for each product by using a one-time operator API key against that product's `POST /auth/generate-service-token` or `POST /auth/rotate-service-token` route. When `PATCHHIVE_SUITE_BOOTSTRAP_SECRET` is configured across the suite, HiveCore can also do that automatically from the Setup tab without asking for operator credentials again. HiveCore stores only the returned service token, and encrypts it at rest when `HIVECORE_ENCRYPTION_KEY` is configured. Operator login credentials are not persisted. Legacy product API keys still work as an explicit fallback during the transition, but legacy service-token hashes are now limited to `runs:read` until they are rotated into scoped records.

## Product Registry Defaults

HiveCore starts with built-in localhost defaults for the current PatchHive suite:

| Product | Frontend | API |
|---------|----------|-----|
| RepoReaper | `http://localhost:5173` | `http://localhost:8000` |
| SignalHive | `http://localhost:5174` | `http://localhost:8010` |
| TrustGate | `http://localhost:5175` | `http://localhost:8020` |
| RepoMemory | `http://localhost:5176` | `http://localhost:8030` |
| ReviewBee | `http://localhost:5177` | `http://localhost:8040` |
| MergeKeeper | `http://localhost:5178` | `http://localhost:8050` |
| FlakeSting | `http://localhost:5179` | `http://localhost:8060` |
| DepTriage | `http://localhost:5180` | `http://localhost:8070` |
| VulnTriage | `http://localhost:5181` | `http://localhost:8110` |
| RefactorScout | `http://localhost:5182` | `http://localhost:8090` |
| HiveCore | `http://localhost:5183` | `http://localhost:8100` |

If you run products on subdomains or remote hosts, save the new targets in HiveCore's Settings tab. Those overrides persist in the HiveCore SQLite database.

HiveCore does not read product databases. It uses product-owned APIs and saved product service tokens, so each product remains independently runnable and keeps ownership of its own run history and validation.

## HiveCore Fit

HiveCore is the suite fit layer. It brings standalone products into one operator surface for health, launch links, shared defaults, run history, action dispatch, and contract drift. Deeper orchestration should build on shared product APIs, not private implementation shortcuts.

## Local Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- Generate the first local API key from `http://localhost:5183`.

## Standalone Repository

HiveCore should be developed in the PatchHive monorepo first. The standalone [`patchhive/hivecore`](https://github.com/patchhive/hivecore) repository should mirror this directory rather than becoming a second source of truth.

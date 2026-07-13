# HiveCore

<p align="center">
  <img src="../../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

HiveCore is the PatchHive control plane. It brings standalone PatchHive products into one operational interface for health, launch links, shared defaults, run history, capability visibility, action dispatch, and product handoffs.

---

## Product Role

HiveCore is not a replacement for standalone products. Its first job is to make the suite legible: what is running, what is healthy, what capabilities exist, what work has happened, and where product contracts have drifted.

Longer term, HiveCore should become the browser-facing suite backend for PatchHive: one operator auth flow, one shared product registry, one shared credential/config surface, and namespaced product APIs for all product frontends. The products remain distinct, but their v2 frontends should eventually talk to HiveCore instead of separate product backends. See [Suite backend direction](../suite-backend-direction.md).

```
RepoReaper  SignalHive  TrustGate  RepoMemory  ReviewBee  MergeKeeper
FlakeSting  DepTriage   VulnTriage RefactorScout  ReleaseSentry
       │         │           │          │              │
       └─────────┴───────────┴──────────┴──────────────┘
                            │
                            ▼
                     ┌────────────┐
                     │  HiveCore  │
                     └─────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
           Health      Settings     Overview
           Polling     (defaults,   (runtime
                       overrides,   products,
                       service      summary,
                       tokens)      contract
                                    drift)
```

---

## Core Workflow

```
Operator / Frontend
    │
    ├── GET /overview ─────────────────────────► Build runtime products
    │                                              │
    │                                              ├── Poll each product /health (3s timeout)
    │                                              ├── Poll each product /startup/checks
    │                                              ├── Poll each product /capabilities
    │                                              ├── Poll each product /runs
    │                                              └── Aggregate into contract drift + health snapshot
    │
    ├── GET /settings ───────────────────────────► Suite defaults + per-product overrides
    │
    ├── PUT /settings ───────────────────────────► Save suite settings + product overrides
    │
    ├── POST /products/:slug/provision-service-token ──► Provision O(1) product service token
    │                                              │
    │                                              ├── Fetch product /auth/status
    │                                              ├── POST /auth/generate-service-token or /rotate
    │                                              └── Persist returned token (encrypted if key set)
    │
    ├── POST /products/:slug/actions/:action_id ──► Dispatch advertised product action
    │                                              │
    │                                              ├── Fetch product /capabilities
    │                                              ├── Verify action exists + not destructive
    │                                              ├── Check service-token scopes match requirements
    │                                              └── Proxy HTTP request to product action path
    │
    └── GET /setup/first-stack ───────────────────► First-stack readiness
                                                   │
                                                   ├── Detect patchhive-launcher availability
                                                   ├── Check Docker + docker-compose
                                                   ├── Probe product ports and compose state
                                                   └── Report credential requirements
```

---

## Inputs

| Input | Source | Description |
|-------|--------|-------------|
| Suite settings | `PUT /settings` body | Operator label, mission, default topics/languages, repo allow/denylist, opt-out notes, preferred launch product |
| Product overrides | `PUT /settings` body | Per-product frontend URL, API URL, service token, legacy API key, enabled flag |
| Operator API key | `POST /auth/login` body | Bootstrap or verify operator identity |
| Service token | `POST /products/:slug/provision-service-token` body | One-time operator key or suite bootstrap secret for token provisioning |
| Launcher status | `patchhive-launcher` API (`PATCHHIVE_LAUNCHER_URL`) | Docker availability, compose state, port status for first-stack products |
| Product health data | Each product's `/health`, `/startup/checks`, `/capabilities`, `/runs`, `/runs/:id` | Polled by HiveCore on behalf of the operator |

---

## Outputs

| Output | Shape | Description |
|--------|-------|-------------|
| Overview response | `OverviewResponse` | All runtime products with health, capabilities, contract checks, recent runs, and aggregated summary |
| Settings response | `SettingsResponse` | Suite settings + all products with default/override URLs, auth mode, enabled state |
| Product runs snapshot | `ProductRunsSnapshotResponse` | A product's run list fetched through its `/runs` contract |
| Product run detail | `ProductRunDetailResponse` | A single run's detail fetched through the product's `/runs/:id` contract |
| Action event | `ProductActionEvent` | Record of a dispatched product action with request/response payloads |
| First-stack setup status | `FirstStackSetupResponse` | Launcher status, per-product credentials, pairing readiness, smoke run history, fleet launch jobs |
| Contract drift report | `Vec<ProductContractCheck>` | Per-endpoint pass/fail/lock with error messages across health, startup, capabilities, runs, and run detail |

---

## Safety Boundary

- HiveCore is a **control plane**, not a replacement runtime for products. It does not read private product databases, bypass product auth, or dispatch destructive actions.
- **Action dispatch is capability-driven:** only actions advertised by the product's `/capabilities` endpoint can be dispatched. Destructive actions are blocked server-side.
- **Service-token scoping:** dispatch checks that the saved service token's scopes cover the action's `required_scopes`. Legacy tokens limited to `runs:read` are rejected for action dispatch.
- **Self-actions blocked:** HiveCore refuses to dispatch actions to itself — native HiveCore routes handle HiveCore operations.
- **Disabled products are skipped:** HiveCore does not poll, fetch runs, or dispatch actions for disabled products.
- **Run detail path sanitized:** run IDs containing `/`, `?`, `#`, `{`, `}` are rejected before being placed into product path templates.
- **Partial failures are non-fatal:** If a product is offline or its API is unreachable, HiveCore reports `offline` health with the error message and continues polling remaining products.

Future control-plane safety work is specified in
[HiveCore repository safety and PR budgets](../hivecore-repository-safety-and-pr-budgets.md):
a verified public repository-owner opt-out, operator-managed trusted repos, and
atomic per-product plus suite-wide PR reservations. These controls are not
implemented by the current settings model.

---

## API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/capabilities` | Public | Advertises HiveCore's capabilities to other PatchHive products |
| `GET` | `/health` | Public | Service health, DB status, auth state, config errors, product override count |
| `GET` | `/startup/checks` | Public | Logged startup validation results |
| `GET` | `/auth/status` | Public | Whether auth is configured and enabled |
| `POST` | `/auth/login` | Public | Verify an API key |
| `POST` | `/auth/generate-key` | Localhost only | Generate first API key (one-shot) |
| `POST` | `/auth/generate-service-token` | Localhost/remote if configured | Generate first service token for machine callers |
| `POST` | `/auth/rotate-service-token` | Localhost/remote if configured | Rotate existing service token |
| `GET` | `/overview` | API key / Service token | Full suite overview with all runtime products, health, summary |
| `GET` | `/products` | API key / Service token | All runtime products as a flat list |
| `GET` | `/settings` | API key / Service token | Suite settings and product overrides |
| `PUT` | `/settings` | Service token only | Save suite settings and product overrides |
| `GET` | `/products/:slug/runs` | API key / Service token | Fetch a product's recent runs through its `/runs` contract |
| `GET` | `/products/:slug/runs/:id` | API key / Service token | Fetch a single run detail through the product's `/runs/:id` contract |
| `POST` | `/products/:slug/provision-service-token` | API key / Service token | Provision or rotate a product's service token server-side |
| `POST` | `/products/:slug/actions/:action_id` | API key / Service token | Dispatch an advertised product action |
| `GET` | `/actions/recent` | API key / Service token | Recent 30 action events |
| `GET` | `/runs` | API key / Service token | HiveCore's own action events as contract-compatible run summaries |
| `GET` | `/runs/:id` | API key / Service token | Single action event detail |
| `GET` | `/setup/first-stack` | API key / Service token | First-stack setup status from patchhive-launcher |
| `POST` | `/setup/first-stack/start` | API key / Service token | Start the first stack through launcher |
| `POST` | `/setup/first-stack/pair` | API key / Service token | Auto-detect and pair with already-running products |
| `POST` | `/setup/first-stack/smoke` | API key / Service token | Run all first-stack smoke tiers |
| `POST` | `/setup/smoke/:tier` | API key / Service token | Run a specific smoke tier by name |
| `POST` | `/setup/first-stack/stop` | API key / Service token | Stop and remove first-stack containers |
| `POST` | `/setup/fleet/start-ready` | API key / Service token | Start products that are ready to launch |
| `POST` | `/setup/fleet/start-all` | API key / Service token | Start all products in the first stack |
| `POST` | `/setup/products/:slug/start` | API key / Service token | Start a specific product |
| `POST` | `/setup/products/:slug/stop` | API key / Service token | Stop a specific product |
| `POST` | `/setup/products/:slug/restart` | API key / Service token | Restart a specific product |
| `GET` | `/setup/products/:slug/logs` | API key / Service token | Fetch logs for a setup product |
| `POST` | `/setup/products/:slug/env` | API key / Service token | Save environment variables for a setup product |
| `POST` | `/setup/credentials/github/validate` | API key / Service token | Validate a GitHub token against the GitHub API |

### Auth

- **API key authentication** is optional. Enabled by setting `HIVE_CORE_API_KEY_HASH`.
- **Service token auth** for HiveCore machine-to-machine calls. Enabled by setting `HIVE_CORE_SERVICE_TOKEN_HASH`.
- Public paths (no auth required): `/health`, `/auth/*`, `/capabilities`, `/startup/checks`.
- Service-only paths: `PUT /settings` requires a service token.
- Key generation limited to localhost bootstrap by default. Set `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP=true` to allow remote key generation.
- All authenticated requests use `X-API-Key` or `X-PatchHive-Service-Token` header.

### Error Responses

All errors are wrapped in the `ApiEnvelope` format:

```json
{
  "status": "error",
  "data": null,
  "error": {
    "code": "unknown_product",
    "message": "Unknown product.",
    "retryable": false,
    "details": {}
  },
  "meta": {
    "product": "hive-core",
    "version": "0.1.0",
    "request_id": "req_…",
    "timestamp": "2026-06-28T12:00:00Z"
  }
}
```

| Status | Error Codes | Meaning |
|--------|-------------|---------|
| 400 | `unsupported_action`, `product_unconfigured`, `product_service_token_missing`, `invalid_action_path`, `invalid_action_url`, `invalid_action_method`, `invalid_run_id`, `invalid_run_detail_url`, `operator_api_key_required`, `invalid_request` | Invalid request body, missing configuration, or malformed parameters |
| 401 | — | Missing or invalid API key / service token |
| 403 | `destructive_action_blocked`, `service_token_expired`, `service_token_rotation_required`, `service_token_scope_missing` | Action blocked by policy or insufficient service-token scopes |
| 404 | `unknown_product`, `run_not_found`, `unknown_action` | Resource not found |
| 409 | `product_disabled`, `run_detail_unsupported`, `service_auth_unsupported` | Product disabled or missing required capability |
| 500 | `internal_error` | Unexpected server error during persistence |
| 502 | `capabilities_unavailable`, `auth_status_unavailable`, `service_token_provision_failed`, `service_token_provision_rejected`, `service_token_missing` | Upstream product API unreachable or returned an error |
| 503 | — | Auth is not enabled and login was attempted |

---

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `HIVE_CORE_PORT` | `8100` | Backend HTTP port |
| `HIVE_CORE_DB_PATH` | `hive-core.db` | SQLite database file path |
| `HIVE_CORE_DB_POOL_SIZE` | — | SQLite connection pool size |
| `HIVE_CORE_API_KEY_HASH` | — | Argon2 hash for API key auth (optional) |
| `HIVE_CORE_SERVICE_TOKEN_HASH` | — | Argon2 hash for HiveCore service token (optional) |
| `HIVECORE_ENCRYPTION_KEY` | — | Encrypts saved downstream product service tokens at rest. Auto-migrates existing plaintext rows on boot |
| `PATCHHIVE_LAUNCHER_URL` | — | Base URL for the local `patchhive-launcher` service that controls Docker start/stop |
| `PATCHHIVE_SUITE_BOOTSTRAP_SECRET` | — | Shared bootstrap secret for automatic service-token provisioning/rotation across products |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | — | Set to `true` to allow API key generation from non-localhost clients |
| `BOT_GITHUB_TOKEN` | — | Optional GitHub fine-grained PAT for future control-plane GitHub reads. Metadata (read) scope is sufficient |
| `RUST_LOG` | `info` | Logging level |

Generate `HIVECORE_ENCRYPTION_KEY` with `openssl rand -hex 32`. Startup checks
reject short values and obvious placeholders; retain the same key across
restarts so existing encrypted service tokens remain readable.

To reuse the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root before starting the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh`.

---

## Technical Architecture

### Service Layout

```
products/hive-core/
├── backend/
│   └── src/
│       ├── main.rs                  ── Axum router, middleware, server init
│       ├── models.rs                ── Request/response types (ApiEnvelope, SuiteSettings,
│                                      ProductOverride, OverviewResponse, SettingsResponse,
│                                      ProductRunsSnapshotResponse, ProductRunDetailResponse,
│                                      FirstStackSetupResponse, ProductActionEvent, …)
│       ├── db.rs                    ── SQLite persistence (suite settings, product overrides,
│                                      action events, service token storage stats, health check)
│       ├── pipeline/
│       │   ├── mod.rs               ── Module exports
│       │   ├── routes.rs            ── All route handler wrappers delegating to sub-modules
│       │   ├── types.rs             ── Shared helpers: api_error, ProductStoredAuth,
│       │                              ProductAuthStatusBody, ProductProbeSnapshot,
│       │                              contract_check helpers, URL resolution
│       │   ├── overview.rs          ── Overview, products, product_runs, product_run_detail:
│       │                              builds runtime products by polling each product's contract
│       │                              endpoints, summarizes contract drift
│       │   ├── settings.rs          ── GET/PUT /settings: suite settings + product overrides
│       │   ├── dispatch.rs          ── Action dispatch: recent_actions, dispatch_product_action,
│       │                              service-token scope verification, path template filling
│       │   ├── provision.rs         ── Service token provisioning: contacts product auth endpoints,
│       │                              persists returned token, encrypts if key configured
│       │   ├── setup.rs             ── First-stack setup: launcher status, start/stop/restart
│       │                              products, fleet launch jobs, product env management,
│       │                              GitHub token validation
│       │   └── smoke.rs             ── Smoke test tiers for first-stack verification
│       ├── secrets.rs               ── TokenProtector for at-rest encryption/decryption
│       ├── startup.rs               ── Config validation checks, check caching, level summarization
│       └── state.rs                 ── AppState (reqwest Client, fleet launch state),
│                                      ProductDefinition catalog (12 built-in products)
├── frontend/                        ── HiveCore UI (React/Svelte, @patchhivehq/ui)
├── frontend-v2/                     ── UI v2 prototype
├── docker-compose.yml               ── Docker deployment
├── .env.example                     ── Configuration template
└── README.md                        ── Product README
```

### Dependencies

- **Axum** — HTTP server and routing
- **patchhive-product-core** — Auth macros, SQLite pool, startup checks, rate limiting, CORS, contract types
- **reqwest** — HTTP client for polling product APIs and dispatching actions
- **rusqlite** — SQLite driver
- **serde / serde_json** — Serialization
- **chrono** — Timestamp handling
- **uuid** — Event IDs
- **tokio** — Async runtime
- **tracing** — Structured logging

### Data Flow

```
                       ┌──────────────┐
                       │   SQLite DB  │
                       │  hive-core   │
                       │  .db         │
                       └──┬───────────┘
                          │
                     ┌────▼─────┐
                     │  db.rs   │
                     │ (CRUD)   │
                     └────┬─────┘
                          │
            ┌─────────────┼──────────────┐
            │             │              │
       ┌────▼───┐   ┌────▼───┐     ┌────▼───┐
       │overview│   │settings│     │dispatch│
       │.rs     │   │.rs     │     │.rs     │
       └───┬────┘   └────────┘     └───┬────┘
           │                           │
           │ HTTP reqwest              │ HTTP reqwest
           ▼                           ▼
    ┌──────────────┐           ┌──────────────┐
    │ Product APIs │           │ Product APIs │
    │ /health      │           │ /capabilities│
    │ /startup     │           │ action paths │
    │ /capabilities│           └──────────────┘
    │ /runs        │
    └──────────────┘
```

HiveCore stores:
- **Suite settings** — operator label, mission, default topics/languages, repo allow/denylist, notes
- **Product overrides** — per-product frontend URL, API URL, service token, legacy API key, enabled state, notes
- **Action events** — history of dispatched product actions with request/response payloads, timestamps, remote status codes

It does **not** store product run data. Product runs are fetched live from each product's API through the shared contract.

### Product Catalog

HiveCore ships with 12 built-in product definitions with localhost defaults:

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
| ReleaseSentry | `http://localhost:5184` | `http://localhost:8120` |

Product URLs can be overridden per environment in the Settings tab. Overrides persist in SQLite.

---

## Monitoring

### Health Endpoint (`GET /health`)

```json
{
  "status": "ok",
  "version": "0.1.0",
  "product": "HiveCore by PatchHive",
  "auth_enabled": true,
  "config_errors": 0,
  "db_ok": true,
  "db_path": "hive-core.db",
  "product_override_count": 12,
  "mode": "control-plane"
}
```

| Field | Meaning |
|-------|---------|
| `status` | `ok` if no config errors and DB is healthy; `degraded` otherwise |
| `config_errors` | Count of error-level startup checks |
| `db_ok` | Whether SQLite health check passed |
| `product_override_count` | Number of persisted product overrides in the database |
| `auth_enabled` | Whether `HIVE_CORE_API_KEY_HASH` is configured |
| `mode` | Always `"control-plane"` |

### Key Metrics

| Metric | Source | What it tells you |
|--------|--------|-------------------|
| `config_errors` | Startup checks | Count of failed startup validations (missing encryption key, unconfigured auth) |
| `db_ok` | SQLite | Whether the database is reachable |
| `product_override_count` | DB | Number of persisted product overrides (0 = using built-in defaults only) |
| `token_storage_stats` | DB | Count of encrypted vs plaintext service tokens |
| `auth_enabled` | Config | Whether operator API key auth is active |
| `contract_drift_count` | Per product | Number of contract checks that failed per product |

---

## Local Development

```bash
cd products/hive-core
cp .env.example .env
docker compose up --build
```

Defaults:
- Frontend: `http://localhost:5183`
- Backend: `http://localhost:8100`
- Database: `hive-core.db`

Split local workflow:
```bash
cd products/hive-core/backend
cargo run

cd ../frontend
npm install
npm run dev
```

Generate the first local API key from `http://localhost:5183` or via the `/auth/generate-key` endpoint.

For first-stack setup, start `patchhive-launcher` on port 8210 (`PATCHHIVE_LAUNCHER_URL=http://localhost:8210`). The Setup tab will detect already-running products, start missing ones, and auto-pair HiveCore with them.

---

## Deployment

The `docker-compose.yml` runs the backend as a single container with SQLite on a mounted volume. For production:

1. Set `HIVE_CORE_API_KEY_HASH` for operator API auth
2. Set `HIVE_CORE_SERVICE_TOKEN_HASH` for inter-product service token auth
3. Set `HIVECORE_ENCRYPTION_KEY` for at-rest encryption of downstream service tokens
4. Configure `HIVE_CORE_DB_PATH` to a persisted volume
5. Set `PATCHHIVE_LAUNCHER_URL` if using the Setup tab for Docker control
6. Bootstrap the API key via `POST /auth/generate-key` from localhost

---

## Troubleshooting

| Symptom | Likely Cause | Check |
|---------|-------------|-------|
| Product shows `offline` in overview | Product API is unreachable or slow | Verify the product is running; check its API URL in Settings; verify port matches; ensure no firewall blocks |
| Product shows `unconfigured` | Product API URL is empty | Set the API URL in Settings |
| Product shows `disabled` | Product is disabled in Settings | Enable the product in Settings |
| Auth errors on API calls | API key or service token not set or expired | Generate via `/auth/generate-key` or `/auth/rotate-service-token` |
| `db_ok: false` | SQLite file path wrong or disk full | Check `HIVE_CORE_DB_PATH` and verify filesystem space |
| `config_errors > 0` | Startup validation failures | Check `/startup/checks` endpoint for details; e.g., missing `HIVECORE_ENCRYPTION_KEY` with encrypted tokens in DB |
| Service token provisioning fails with `502` | Product auth endpoint unreachable | Verify product is running; check product's `/auth/status` endpoint |
| Service token provisioning fails with `operator_api_key_required` | Product requires auth but no operator key or bootstrap secret provided | Provide a one-time operator API key or configure `PATCHHIVE_SUITE_BOOTSTRAP_SECRET` |
| `destructive_action_blocked` on dispatch | Action has `destructive: true` | HiveCore does not dispatch destructive actions yet |
| `service_token_scope_missing` on dispatch | Saved service token lacks required scopes | Rotate the service token to obtain scoped replacement |
| `service_token_expired` on dispatch | Product reports the saved service token as expired | Rotate the service token |
| Product run detail returns `BAD_REQUEST` | Product's API URL not configured or service token missing | Configure the API URL and provision a service token in Settings |
| Setup tab shows launcher unavailable | `PATCHHIVE_LAUNCHER_URL` not set or launcher not running | Start patchhive-launcher on port 8210; set the env var |
| Encrypted tokens unreadable | `HIVECORE_ENCRYPTION_KEY` changed or not set | Restore the original encryption key — encrypted tokens cannot be recovered without it |
| First-stack pairing fails | Products running but not configured for suite bootstrap | Run `./scripts/set-suite-api-key.sh --stack first` and set `PATCHHIVE_SUITE_BOOTSTRAP_SECRET` |

---

## Related Products

| Product | Relationship |
|---------|-------------|
| **All PatchHive products** | Upstream/downstream — HiveCore polls health, capabilities, runs, and run detail from each product; dispatches actions through advertised capability contracts |
| **ReleaseSentry** | Downstream — HiveCore can dispatch release readiness checks via service token |
| **RepoReaper** | Downstream — HiveCore can dispatch dry-run actions and smoke tiers |
| **patchhive-launcher** | Sidecar — HiveCore delegates Docker start/stop control to launcher for the Setup tab |

---

## Current Status

| Area | Status |
|------|--------|
| Suite overview with product health polling | ✅ Implemented — polls `/health`, `/startup/checks`, `/capabilities`, `/runs` per product |
| Product run history surfacing | ✅ Implemented — fetches each product's `/runs` contract |
| Run detail drill-down | ✅ Implemented — fetches `/runs/:id` per product with capability gating |
| Contract drift reporting | ✅ Implemented — health, startup checks, capabilities, runs, run detail support |
| Suite settings (global defaults) | ✅ Implemented — topics, languages, repo guardrails, operator notes |
| Product overrides (URL, enabled, notes) | ✅ Implemented — per-product frontend/API URL overrides |
| Service token provisioning | ✅ Implemented — one-time operator key or suite bootstrap secret flow |
| Service token encryption at rest | ✅ Implemented — via `HIVECORE_ENCRYPTION_KEY` |
| Auth (API key + service token) | ✅ Implemented — bootstrap, login, generate, rotate |
| Capabilities advertisement | ✅ Implemented |
| Action dispatch (non-destructive) | ✅ Implemented — capability-driven, scope-checked |
| First-stack setup (launcher integration) | ✅ Implemented — detect, pair, start, stop, restart products |
| Smoke tiers | ✅ Implemented — tiered smoke test execution via `/setup/smoke/:tier` |
| Fleet launch (start-ready, start-all) | ✅ Implemented |
| Setup product env management | ✅ Implemented |
| GitHub token validation | ✅ Implemented — validates token against GitHub API |
| Frontend UI | ✅ Implemented (v1) |
| Frontend v2 | 🚧 In progress |
| Destructive action dispatch | ❌ Blocked — requires explicit approval flow |
| Cross-product orchestration (e.g., RepoReaper + ReleaseSentry handoff) | ❌ Future — will build on shared capability contracts |
| GitHub token for control-plane reads | ❌ Optional — env var exists but `BOT_GITHUB_TOKEN` is not yet wired into product operations |

---

## Standalone Repository

HiveCore should be developed in the PatchHive monorepo first. The standalone [`patchhive/hivecore`](https://github.com/patchhive/hivecore) repository should mirror this directory rather than becoming a second source of truth.

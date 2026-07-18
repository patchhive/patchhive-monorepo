# RefactorScout by PatchHive

RefactorScout surfaces evidence-ranked structural review candidates before code quality drift turns expensive.

It is a read-only scouting product inside PatchHive: a product that looks for cleanup work with a favorable safety-to-value ratio so teams can improve structure without bundling those changes into larger feature or bug-fix moments.

## Documentation

- Full product doc: [docs/products/refactor-scout.md](../../docs/products/refactor-scout.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

> This README is the getting-started entry point. The full product doc carries the API reference, technical architecture, complete configuration reference, monitoring, deployment, and troubleshooting.

### Where to find what

| If you need… | See in the full doc |
| --- | --- |
| API endpoints and request/response shapes | `#api-endpoints` |
| Service layout and dependencies | `#technical-architecture` |
| Every configuration variable | `#configuration` |
| Health checks and metrics | `#monitoring` |
| Production deployment steps | `#deployment` |
| Symptom → cause → fix | `#troubleshooting` |
| How it relates to other products | `#related-products` |
| What is / isn't built yet | `#current-status` |

## Core Workflow

- point RefactorScout at a local repository path inside an allowed root, or a public GitHub repo such as `owner/repo`
- walk the repo without mutating anything
- remove any temporary GitHub clone after the scan finishes
- rank structural review candidates such as oversized runtime modules, long functions, and repeated string usage
- save scan history so recurring cleanup pressure is visible over time
- copy or reload the ranked queue when it is time to schedule cleanup work

## Quick Start

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5182`
Frontend v2 prototype: `http://localhost:5201`
Backend: `http://localhost:8090`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

The UI v2 prototype is isolated from the production frontend while the suite direction is still being tested:

```bash
cd frontend-v2 && npm install && npm run dev
```

## Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | Optional fine-grained PAT reserved for future repo metadata reads. Metadata (read) is enough. |
| `REFACTOR_SCOUT_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `REFACTOR_SCOUT_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `REFACTOR_SCOUT_DB_PATH` | SQLite path for scan history. |
| `REFACTOR_SCOUT_PORT` | Backend port for split local runs. |
| `REFACTOR_SCOUT_ALLOWED_ROOTS` | Colon-separated filesystem roots that may be scanned. |
| `REFACTOR_SCOUT_ALLOW_REMOTE_FS` | Allows authenticated remote clients to trigger filesystem scans. Keep unset unless intentional. |
| `REFACTOR_SCOUT_CLONE_TIMEOUT_SECS` | Optional timeout for temporary public GitHub clones. Defaults to 120 seconds. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Allows first-time key bootstrap from non-localhost clients. Keep unset for local use. |
| `RUST_LOG` | Rust logging level. |

RefactorScout scans local filesystem paths and public GitHub repositories. Set `REFACTOR_SCOUT_ALLOWED_ROOTS` before pointing it at broader checkout directories. GitHub repo inputs are cloned into a temporary directory, scanned, and removed after the scan. By default, filesystem scans are limited to localhost callers even when API-key auth is enabled.

## Unified Backend

RefactorScout's engine is mounted in-process by `services/patchhive-backend`.
The standalone backend and unified runtime use the same library, routes, SQLite
history, auth, filesystem guardrails, and temporary-clone lifecycle.

When launching from the monorepo root, use an absolute database path so the
unified process reuses the product database instead of creating a new root-level
file:

```bash
REFACTOR_SCOUT_DB_PATH="$PWD/products/refactor-scout/refactor-scout.db" \
PATCHHIVE_PRODUCTS=refactor-scout \
cargo run --manifest-path services/patchhive-backend/Cargo.toml
```

The mounted API prefix is
`http://127.0.0.1:8100/api/products/refactor-scout`.

## UI v3 Parity Candidate

The current v3 candidate lives in `frontend-v3/` while the existing v1 and v2
frontends remain available for the final visual parity check:

```bash
cd products/refactor-scout/frontend-v3
npm install
npm run dev
```

It connects to the in-process API by default at
`http://127.0.0.1:8100/api/products/refactor-scout` and preserves local-path
and public-GitHub intake, ranked opportunity details, warnings, copyable
Markdown, saved dashboard views, history, startup diagnostics, filesystem
guidance, responsive layout, and the suite-wide theme preference.

## Safety Boundary

RefactorScout is trying to answer one narrow question well:

Where is the safest cleanup work hiding right now?

That means the product should prefer:

- read-only analysis before automation
- explicit filesystem allowlists
- small, explainable heuristics over magic scores
- queues that help humans schedule refactors, not surprise them with code changes

It does not rewrite code, apply codemods, open pull requests, or scan outside configured filesystem boundaries.

Future write-capable refactor PRs should stay behind an explicit action such as `Create refactor PR`: scan first, select a lead, branch in an isolated clone, run tests, pass TrustGate, then open a clearly attributed PatchHive PR. A normal scan should remain read-only.

## HiveCore Fit

HiveCore can surface RefactorScout health, capabilities, run history, and conservative cleanup opportunities. RefactorScout stays standalone and scan-first; HiveCore should not expand filesystem access beyond the product's own guardrails.

## Standalone Repository

RefactorScout should be developed in the PatchHive monorepo first. The standalone [`patchhive/refactorscout`](https://github.com/patchhive/refactorscout) repository is an exported mirror of this directory rather than a second source of truth.

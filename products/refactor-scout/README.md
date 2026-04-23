# RefactorScout by PatchHive

RefactorScout surfaces safe, high-value refactors before code quality drift turns expensive.

It is a read-only scouting product inside PatchHive: a product that looks for cleanup work with a favorable safety-to-value ratio so teams can improve structure without bundling those changes into larger feature or bug-fix moments.

## Product Documentation

- GitHub-facing product doc: [docs/products/refactor-scout.md](../../docs/products/refactor-scout.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

## Core Workflow

- point RefactorScout at a local repository path inside an allowed root
- walk the repo without mutating anything
- rank safe refactor leads like oversized files, oversized functions, and repeated string literals
- save scan history so recurring cleanup pressure is visible over time
- copy or reload the ranked queue when it is time to schedule cleanup work

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5182`
Backend: `http://localhost:8090`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | Optional GitHub token for future repo metadata reads. |
| `REFACTOR_SCOUT_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `REFACTOR_SCOUT_DB_PATH` | SQLite path for scan history. |
| `REFACTOR_SCOUT_PORT` | Backend port for split local runs. |
| `REFACTOR_SCOUT_ALLOWED_ROOTS` | Colon-separated filesystem roots that may be scanned. |
| `REFACTOR_SCOUT_ALLOW_REMOTE_FS` | Allows authenticated remote clients to trigger filesystem scans. Keep unset unless intentional. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Allows first-time key bootstrap from non-localhost clients. Keep unset for local use. |
| `RUST_LOG` | Rust logging level. |

RefactorScout scans local filesystem paths, so set `REFACTOR_SCOUT_ALLOWED_ROOTS` before pointing it at broader checkout directories. By default, filesystem scans are limited to localhost callers even when API-key auth is enabled.

## Safety Boundary

RefactorScout is trying to answer one narrow question well:

Where is the safest cleanup work hiding right now?

That means the product should prefer:

- read-only analysis before automation
- explicit filesystem allowlists
- small, explainable heuristics over magic scores
- queues that help humans schedule refactors, not surprise them with code changes

It does not rewrite code, apply codemods, open pull requests, or scan outside configured filesystem boundaries.

## HiveCore Fit

HiveCore can surface RefactorScout health, capabilities, run history, and conservative cleanup opportunities. RefactorScout stays standalone and local-scan-first; HiveCore should not expand filesystem access beyond the product's own guardrails.

## Standalone Repository

RefactorScout should be developed in the PatchHive monorepo first. The standalone [`patchhive/refactorscout`](https://github.com/patchhive/refactorscout) repository is an exported mirror of this directory rather than a second source of truth.

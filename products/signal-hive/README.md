# SignalHive by PatchHive

SignalHive shows teams the maintenance work they are not tracking well enough yet.

It is the read-only reconnaissance layer in PatchHive: a product that scans repository history, issue history, and lightweight code signals to surface stale work, duplicate reports, recurring bug patterns, and hidden backlog risk before those problems turn into delivery drag.

## Documentation

- Full product doc: [docs/products/signal-hive.md](../../docs/products/signal-hive.md)
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

- discover repositories from search terms, topics, languages, and repo controls
- inspect issue history for stale backlog pressure and likely duplicate reports
- detect recurring bug-like patterns and TODO or FIXME hotspots
- rank repositories into a maintenance queue with explainable score drivers
- save presets, schedules, trend history, and shareable reports

## UI v2 Prototype

The suite-wide UI v2 prototype for SignalHive lives in `frontend-v2/`. It is a React/Vite radar/atlas prototype used to prove the new PatchHive visual language before the production frontend is ported. The original static mockup is preserved as `frontend-v2/prototype-static.html`.

When using Docker Compose, the v2 prototype is exposed as the `frontend-v2` service on `http://localhost:5192`.

The current production frontend remains in `frontend/`. Keep old UI code there until the full suite has moved to v2 and the old UI is intentionally legacy or removed.

For local v2 development:

```bash
cd frontend-v2 && npm install && npm run dev
```

## Quick Start

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5174`
Backend: `http://localhost:8010`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `BOT_GITHUB_TOKEN` | — | Fine-grained PAT for SignalHive scans. Recommended scopes: Metadata (read), Issues (read), and Contents (read) when GitHub-backed code-search reads are needed. |
| `SIGNAL_API_KEY_HASH` | — | Optional pre-seeded app auth hash. The monorepo helper can set it for a stable suite password. |
| `SIGNAL_SERVICE_TOKEN_HASH` | — | Optional service-token hash for HiveCore or other PatchHive service callers. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | — | Allows first-time key bootstrap from non-localhost clients. Keep unset for local use. |
| `SIGNAL_DB_PATH` | `signal-hive.db` | SQLite path for scan history. |
| `SIGNAL_PORT` | `8010` | Backend port for split local runs. |
| `SIGNAL_MARKER_REPO_LIMIT` | `4` | Caps TODO/FIXME code-search scans to the top-ranked repos. |
| `RUST_LOG` | `info` | Rust logging level. |

SignalHive works best with a fine-grained GitHub token. For public-only scanning, start with `Metadata: Read` and `Issues: Read`; add `Contents: Read` only if your setup needs GitHub-backed TODO or FIXME code-search reads.

To keep the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root and restart the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh` with no extra flags. Once the hash is pre-seeded, logging in through a subdomain works normally without remote bootstrap.

To give HiveCore a dedicated machine credential instead of reusing the operator login secret, generate a service token from `POST /auth/generate-service-token` and save that token in HiveCore Settings.

## Safety Boundary

SignalHive is designed to answer one question well: where is maintenance pressure building before anyone acts on it?

It is the visibility-first entry point into the PatchHive suite. It does not open pull requests, mutate repositories, dispatch other products, or require AI for the first MVP loop. Allowlist, denylist, and opt-out controls are built into the product for safer discovery.

## HiveCore Fit

SignalHive should be the first source of candidate work for the suite. HiveCore can surface SignalHive health, capabilities, run history, schedules, and discovered maintenance pressure, then later hand approved candidates to TrustGate or RepoReaper through explicit product contracts.

## Standalone Repository

The PatchHive monorepo is the source of truth for SignalHive development. The standalone [`patchhive/signalhive`](https://github.com/patchhive/signalhive) repository is an exported mirror of this directory.

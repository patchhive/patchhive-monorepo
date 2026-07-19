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

## Canonical UI

SignalHive passed its final UI v3 parity audit on 2026-07-16. The canonical
frontend lives in `frontend/`; the former v1 and v2 trees were removed after
direct and discovery scans, scheduled runs, presets, repository controls,
history, trend/timeline evidence, reports, diagnostics, responsive behavior,
and light/dark behavior were accepted.

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
| `PATCHHIVE_GITHUB_TOKEN_RO` | — | Suite-wide classic PAT for GitHub reads. Use `public_repo` for public repositories or `repo` for private repositories. |
| `SIGNAL_API_KEY_HASH` | — | Optional pre-seeded app auth hash. The monorepo helper can set it for a stable suite password. |
| `SIGNAL_SERVICE_TOKEN_HASH` | — | Optional service-token hash for HiveCore or other PatchHive service callers. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | — | Allows first-time key bootstrap from non-localhost clients. Keep unset for local use. |
| `SIGNAL_DB_PATH` | `signal-hive.db` | SQLite path for scan history. |
| `SIGNAL_PORT` | `8010` | Backend port for split local runs. |
| `SIGNAL_MARKER_REPO_LIMIT` | `4` | Caps TODO/FIXME code-search scans to the top-ranked repos. |
| `RUST_LOG` | `info` | Rust logging level. |

SignalHive works best with a classic GitHub token. For public-only scanning, start with `Metadata: Read` and `Issues: Read`; add `Contents: Read` only if your setup needs GitHub-backed TODO or FIXME code-search reads.

To keep the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root and restart the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh` with no extra flags. Once the hash is pre-seeded, logging in through a subdomain works normally without remote bootstrap.

To give HiveCore a dedicated machine credential instead of reusing the operator login secret, generate a service token from `POST /auth/generate-service-token` and save that token in HiveCore Settings.

## Safety Boundary

SignalHive is designed to answer one question well: where is maintenance pressure building before anyone acts on it?

It is the visibility-first entry point into the PatchHive suite. It does not open pull requests, mutate repositories, dispatch other products, or require AI for the first MVP loop. Allowlist, denylist, and opt-out controls are built into the product for safer discovery.

## HiveCore Fit

SignalHive should be the first source of candidate work for the suite. HiveCore can surface SignalHive health, capabilities, run history, schedules, and discovered maintenance pressure, then later hand approved candidates to TrustGate or RepoReaper through explicit product contracts.

## Standalone Repository

The PatchHive monorepo is the source of truth for SignalHive development. The standalone [`patchhive/signalhive`](https://github.com/patchhive/signalhive) repository is an exported mirror of this directory.

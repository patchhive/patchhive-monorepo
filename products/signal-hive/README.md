# SignalHive by PatchHive

SignalHive shows teams the maintenance work they are not tracking well enough yet.

It is the read-only reconnaissance layer in PatchHive: a product that scans repository history, issue history, and lightweight code signals to surface stale work, duplicate reports, recurring bug patterns, and hidden backlog risk before those problems turn into delivery drag.

## Product Documentation

- GitHub-facing product doc: [docs/products/signal-hive.md](../../docs/products/signal-hive.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

## Core Workflow

- discover repositories from search terms, topics, languages, and repo controls
- inspect issue history for stale backlog pressure and likely duplicate reports
- detect recurring bug-like patterns and TODO or FIXME hotspots
- rank repositories into a maintenance queue with explainable score drivers
- save presets, schedules, trend history, and shareable reports

## Run Locally

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

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub token for repo, issue, and optional code-search reads. |
| `SIGNAL_API_KEY_HASH` | Optional pre-seeded app auth hash. The monorepo helper can set it for a stable suite password. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Allows first-time key bootstrap from non-localhost clients. Keep unset for local use. |
| `SIGNAL_DB_PATH` | SQLite path for scan history. |
| `SIGNAL_PORT` | Backend port for split local runs. |
| `SIGNAL_MARKER_REPO_LIMIT` | Caps TODO/FIXME code-search scans to the top-ranked repos. |
| `RUST_LOG` | Rust logging level. |

SignalHive works best with a fine-grained GitHub token. For public-only scanning, start with `Metadata: Read` and `Issues: Read`; add `Contents: Read` only if your setup needs GitHub-backed TODO or FIXME code-search reads.

To keep the same password across SignalHive, TrustGate, RepoReaper, and HiveCore, run `./scripts/set-suite-api-key.sh --stack first` from the monorepo root and restart the stack. For every PatchHive product, run `./scripts/set-suite-api-key.sh` with no extra flags. Once the hash is pre-seeded, logging in through a subdomain works normally without remote bootstrap.

## Safety Boundary

SignalHive is designed to answer one question well: where is maintenance pressure building before anyone acts on it?

It is the visibility-first entry point into the PatchHive suite. It does not open pull requests, mutate repositories, dispatch other products, or require AI for the first MVP loop. Allowlist, denylist, and opt-out controls are built into the product for safer discovery.

## HiveCore Fit

SignalHive should be the first source of candidate work for the suite. HiveCore can surface SignalHive health, capabilities, run history, schedules, and discovered maintenance pressure, then later hand approved candidates to TrustGate or RepoReaper through explicit product contracts.

## Standalone Repository

The PatchHive monorepo is the source of truth for SignalHive development. The standalone [`patchhive/signalhive`](https://github.com/patchhive/signalhive) repository is an exported mirror of this directory.

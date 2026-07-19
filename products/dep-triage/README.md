# DepTriage by PatchHive

DepTriage is PatchHive's dependency-noise filter. It reads open dependency pull
requests, optionally folds in Dependabot alerts, groups that activity by package,
and turns the result into a ranked queue — `update now`, `watch`, or `ignore for
now`. The canonical Lovable-derived v3 frontend lives in `frontend/` and targets
the in-process unified-backend route at `/api/products/dep-triage`.

## Documentation

- Full product doc: [docs/products/dep-triage.md](../../docs/products/dep-triage.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

> This README is the getting-started entry point. The full product doc carries the API
> reference, technical architecture, complete configuration reference, monitoring, deployment,
> and troubleshooting.

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

- read open dependency pull requests for a target repository
- optionally read matching Dependabot alerts
- group overlapping pull requests and alerts by package
- rank each package into a practical action bucket
- save scan history so teams can compare or revisit earlier triage decisions

## Quick Start

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

## Configuration

| Variable | Purpose |
| --- | --- |
| `PATCHHIVE_GITHUB_TOKEN_RO` | Suite-wide classic PAT with `public_repo` or `repo`; add `security_events` for Dependabot alert reads. |
| `DEP_TRIAGE_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `DEP_TRIAGE_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `DEP_TRIAGE_DB_PATH` | SQLite path for scan history. |
| `DEP_TRIAGE_PORT` | Backend port for split local runs. |
| `RUST_LOG` | Rust logging level. |

DepTriage works best with a classic GitHub token. Reading pull requests is enough for the base product loop. Dependabot alert reads need the matching security permission; without that access, DepTriage still scores dependency pull requests and reports the limitation clearly.

The first API key and service token are bootstrapped locally (generate from the UI, or via the localhost-only `/auth/generate-key` and `/auth/generate-service-token` endpoints). When starting the unified backend by sourcing `products/dep-triage/.env` in a shell, quote any JSON service-token value — unquoted JSON can be flattened by the shell and treated as a legacy token string. HiveCore service-token pairing should use a properly quoted/exported value, the product wrapper's `dotenvy` loading path, or a regenerated scoped service token before depending on service dispatch.

## Safety Boundary

DepTriage is intentionally read-only. It does not merge dependency pull requests, rewrite update configuration, dismiss alerts, or open follow-up issues. Its job is to turn dependency update noise into an explainable queue a human can act on.

## HiveCore Fit

HiveCore can surface DepTriage health, capabilities, run history, and ranked update pressure beside the rest of the suite. DepTriage remains independently runnable; HiveCore should consume its product-owned APIs instead of reading its SQLite database.

## Standalone Repository

The PatchHive monorepo is the source of truth for DepTriage development. The standalone [`patchhive/deptriage`](https://github.com/patchhive/deptriage) repository is an exported mirror of this directory.

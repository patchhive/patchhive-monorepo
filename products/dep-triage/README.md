# DepTriage by PatchHive

DepTriage tells teams which dependency updates matter now and which ones can wait.

It is PatchHive's dependency-noise filter: a product that reads open dependency pull requests, optionally folds in Dependabot alerts, groups that activity by package, and turns the result into a ranked queue such as `update now`, `watch`, or `ignore for now`.

## Product Documentation

- GitHub-facing product doc: [docs/products/dep-triage.md](../../docs/products/dep-triage.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

## Core Workflow

- read open dependency pull requests for a target repository
- optionally read matching Dependabot alerts
- group overlapping pull requests and alerts by package
- rank each package into a practical action bucket
- save scan history so teams can compare or revisit earlier triage decisions

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5180`
Frontend v2 prototype: `http://localhost:5199`
Backend: `http://localhost:8070`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

The UI v2 prototype is isolated from the production frontend while the suite
direction is still being tested:

```bash
cd frontend-v2 && npm install && npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` | Optional fine-grained PAT for dependency pull request and Dependabot reads. Recommended scopes: Metadata (read), Pull requests (read), Dependabot alerts (read). |
| `DEP_TRIAGE_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `DEP_TRIAGE_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `DEP_TRIAGE_DB_PATH` | SQLite path for scan history. |
| `DEP_TRIAGE_PORT` | Backend port for split local runs. |
| `RUST_LOG` | Rust logging level. |

DepTriage works best with a fine-grained GitHub token. Reading pull requests is enough for the base product loop. Dependabot alert reads need the matching security permission; without that access, DepTriage still scores dependency pull requests and reports the limitation clearly.

## Safety Boundary

DepTriage is intentionally read-only. It does not merge dependency pull requests, rewrite update configuration, dismiss alerts, or open follow-up issues. Its job is to turn dependency update noise into an explainable queue a human can act on.

## HiveCore Fit

HiveCore can surface DepTriage health, capabilities, run history, and ranked update pressure beside the rest of the suite. DepTriage remains independently runnable; HiveCore should consume its product-owned APIs instead of reading its SQLite database.

## Standalone Repository

The PatchHive monorepo is the source of truth for DepTriage development. The standalone [`patchhive/deptriage`](https://github.com/patchhive/deptriage) repository is an exported mirror of this directory.

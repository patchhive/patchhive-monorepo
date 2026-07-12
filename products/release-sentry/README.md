# ReleaseSentry by PatchHive

The canonical Lovable-derived specialist frontend lives in `frontend/` and targets the
in-process unified-backend route at `/api/products/release-sentry`.

ReleaseSentry checks whether a repo, product, or release candidate is actually ready to ship.
It is the release-readiness layer for PatchHive: not another changelog generator, but the
product that gathers the evidence behind a `ready`, `watch`, or `hold` call before humans or
HiveCore push a release forward.

## Documentation

- Full product doc: [docs/products/release-sentry.md](../../docs/products/release-sentry.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

> This README is the getting-started entry point. The full product doc carries the API
> reference, check logic, technical architecture, complete configuration reference, monitoring,
> deployment, and troubleshooting.

### Where to find what

| If you need… | See in the full doc |
| --- | --- |
| API endpoints and request/response shapes | `#api-endpoints` |
| Check logic and decision scoring | `#check-logic` |
| Service layout and dependencies | `#technical-architecture` |
| Every configuration variable | `#configuration` |
| Health checks and metrics | `#monitoring` |
| Production deployment steps | `#deployment` |
| Symptom → cause → fix | `#troubleshooting` |
| How it relates to other products | `#related-products` |
| What is / isn't built yet | `#current-status` |

## Core Workflow

- send a release-readiness check through `POST /check/github/release`
- detect changelog/version/tag drift for a target branch, version, or tag
- summarize CI and workflow health for the candidate branch
- detect open release-blocker issues by configurable blocker labels
- run common release-surface checks for manifests, Compose files, and CI/release workflows
- record saved run history through `/history`, `/runs`, and `/runs/:id` for HiveCore
- return a release decision with evidence: `ready`, `watch`, or `hold`

Example request:

```json
{
  "repo": "patchhive/patchhive2",
  "branch": "main",
  "target_version": "0.2.0",
  "target_tag": "v0.2.0",
  "changelog_path": "CHANGELOG.md",
  "workflow_run_limit": 20
}
```

## Product Boundary

ReleaseSentry answers release questions:

- Did CI pass on the branch or tag being considered?
- Are there unresolved release blockers?
- Did version, changelog, tag, and package/image state drift apart?
- Did dependency, security, or flake pressure become too risky to ignore?
- What changed since the last release?

ReleaseSentry does not replace RepoReaper, MergeKeeper, or HiveCore. It sits after
merge-readiness and before shipping.

## Quick Start

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5184`
Backend: `http://localhost:8120`
Suite backend route: `http://localhost:8100/api/products/release-sentry`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

The v1 and v2 frontends were removed after the v3 parity audit and live ready/watch/hold
acceptance gate passed on 2026-07-11.

### Unified Backend Mode

ReleaseSentry's backend logic is exported as a product module and can run inside
`services/patchhive-backend` without a separate product backend process:

```bash
PATCHHIVE_PRODUCTS=release-sentry \
PATCHHIVE_BIND_ADDR=127.0.0.1:8100 \
cargo run --manifest-path services/patchhive-backend/Cargo.toml

npm --prefix products/release-sentry/frontend run dev
```

The standalone backend remains available as a compatibility wrapper while the suite backend
migration is tested.

## Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub fine-grained PAT for release-readiness checks. Scopes: Metadata (read), Contents (read), Actions (read), Issues (read), Deployments/Releases (read). |
| `RELEASE_SENTRY_API_KEY_HASH` | Optional pre-seeded API-key auth hash. Otherwise generate the first local key from the UI. |
| `RELEASE_SENTRY_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore dispatch. |
| `RELEASE_SENTRY_DB_PATH` | SQLite path for run history. |
| `RELEASE_SENTRY_DB_POOL_SIZE` | SQLite connection pool size. |
| `RELEASE_SENTRY_PORT` | Backend port for split local runs (default `8120`). |
| `RUST_LOG` | Rust logging level. |
| `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP` | Set `true` to allow API-key generation from non-localhost. |

GitHub-backed checks should use a fine-grained token with Metadata (read), Contents (read),
Actions (read), Issues (read), and Deployments/Releases read access where available. Keep
repository access public-only unless release readiness for private repos is explicitly enabled.

## Safety Boundary

- ReleaseSentry is **read-only** — it recommends, it does not gate. The decision to ship stays with the operator or HiveCore.
- **Validation:** `repo` must match `owner/name` format. `workflow_run_limit` is clamped to 5–100. Empty `branch` falls back to the repository's `default_branch`.
- **Partial failures are non-fatal.** If a GitHub API call fails, the check becomes a `warn` with the error in evidence, and the remaining checks complete normally.
- **Changelog check** is `warn` (not `block`) when missing.
- **Tag check** passes if no target tag is specified; if provided but not found, the check warns.

## HiveCore Fit

ReleaseSentry is the second product engine mounted in-process inside `services/patchhive-backend`,
after MergeKeeper. HiveCore dispatches release checks through the service-token path
`POST /check/github/release` (service-only, reserved for HiveCore) and reads run history through
`GET /runs` and `GET /runs/:id` (compatibility aliases of `/history`). Service-token dispatch
requires `RELEASE_SENTRY_SERVICE_TOKEN_HASH`; capabilities are advertised via `GET /capabilities`.

## Standalone Repository

The PatchHive monorepo is the source of truth for ReleaseSentry development. The standalone
[`patchhive/release-sentry`](https://github.com/patchhive/release-sentry) repository is an
exported mirror of this directory. Generate the first local API key from `http://localhost:5184`.

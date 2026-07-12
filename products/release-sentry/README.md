# ReleaseSentry by PatchHive

The canonical Lovable-derived specialist frontend lives in `frontend/` and targets the
in-process unified-backend route at `/api/products/release-sentry`.

ReleaseSentry checks whether a repo, product, or release candidate is actually ready to ship.

It is the release-readiness layer for PatchHive: not another changelog generator, but the product that gathers the evidence behind a `ready`, `watch`, or `hold` call before humans or HiveCore push a release forward.

GitHub-facing product doc: [docs/products/release-sentry.md](../../docs/products/release-sentry.md)

Standalone mirror: [`patchhive/release-sentry`](https://github.com/patchhive/release-sentry)

## Product Boundary

ReleaseSentry answers release questions:

- Did CI pass on the branch or tag being considered?
- Are there unresolved release blockers?
- Did version, changelog, tag, and package/image state drift apart?
- Did dependency, security, or flake pressure become too risky to ignore?
- What changed since the last release?

ReleaseSentry does not replace RepoReaper, MergeKeeper, or HiveCore. It sits after merge-readiness and before shipping.

## MVP Shape

- Read-only GitHub release readiness checks through `POST /check/github/release`.
- Changelog/version/tag drift detection for a target branch, version, or tag.
- CI and workflow health summary for the candidate branch.
- Open release-blocker issue detection by configurable blocker labels.
- Common release surface checks for manifests, Compose files, and CI/release workflows.
- Saved run history through `/history`, `/runs`, and `/runs/:id` for HiveCore.
- A release decision with evidence: `ready`, `watch`, or `hold`.

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

## Run Locally

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

The v1 and v2 frontends were removed after the v3 parity audit and live
ready/watch/hold acceptance gate passed on 2026-07-11.

### Unified Backend Mode

ReleaseSentry's backend logic is exported as a product module and can run
inside `services/patchhive-backend` without a separate product backend process:

```bash
PATCHHIVE_PRODUCTS=release-sentry \
PATCHHIVE_BIND_ADDR=127.0.0.1:8100 \
cargo run --manifest-path services/patchhive-backend/Cargo.toml

npm --prefix products/release-sentry/frontend run dev
```

The standalone backend remains available as a compatibility wrapper while the
suite backend migration is tested.

## Local Notes

- The canonical frontend lives in `frontend/`; retired v1 and v2 implementations are no longer packaged or kept in the tree.
- The frontend uses `@patchhivehq/ui-v3` primitives and the shared product shell.
- The backend stores release readiness history in SQLite at `RELEASE_SENTRY_DB_PATH`.
- GitHub-backed checks should use a fine-grained token with Metadata (read), Contents (read), Actions (read), Issues (read), and Deployments/Releases read access where available.
- Keep repository access public-only unless release readiness for private repos is explicitly enabled.
- Generate the first local API key from `http://localhost:5184`.
- If remote bootstrap is intentional, set `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP=true`.

## Repository Model

ReleaseSentry should be developed in the PatchHive monorepo first. The standalone repository, when exported, should be treated as a mirror of this directory rather than a second source of truth.

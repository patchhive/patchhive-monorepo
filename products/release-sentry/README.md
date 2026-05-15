# ReleaseSentry by PatchHive

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

- Read-only GitHub release readiness checks.
- Changelog/version/tag drift detection.
- CI and workflow health summary for the candidate branch or tag.
- Dependency/security pressure imported from DepTriage and VulnTriage when available.
- A release decision with evidence: `ready`, `watch`, or `hold`.

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5184`
Backend: `http://localhost:8120`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Local Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend stores starter state in SQLite at `RELEASE_SENTRY_DB_PATH`.
- GitHub-backed checks should use a fine-grained token with Metadata (read), Contents (read), Pull requests (read), Actions (read), Commit statuses (read), and Deployments/Releases read access where needed.
- Keep repository access public-only unless release readiness for private repos is explicitly enabled.
- Generate the first local API key from `http://localhost:5184`.
- If remote bootstrap is intentional, set `PATCHHIVE_ALLOW_REMOTE_BOOTSTRAP=true`.

## Repository Model

ReleaseSentry should be developed in the PatchHive monorepo first. The standalone repository, when exported, should be treated as a mirror of this directory rather than a second source of truth.

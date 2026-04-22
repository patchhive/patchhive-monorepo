# TrustGate by PatchHive

TrustGate reviews diffs before they move forward.

It is PatchHive's trust and safety layer: a product that checks AI-generated or pull-request-backed diffs against repo-specific risk rules, then returns a simple recommendation such as `safe`, `warn`, or `block` with the reasons made explicit.

## Core Workflow

- review pasted unified diffs or fetch pull request diffs directly from GitHub
- apply repo-specific rules, starter rule packs, and saved report templates
- flag risky paths, suspicious patterns, missing tests, and oversized change sets
- publish the result back into GitHub through checks, statuses, and a maintained pull request comment
- save decision history and expose a print-friendly decision view

TrustGate is intentionally review-first. It does not rewrite code. Its job is to make risk visible before downstream automation or maintainers move a change forward.

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5175`
Backend: `http://localhost:8020`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## GitHub Access

TrustGate works without GitHub for pasted diff review, but GitHub integration is what makes it operational inside real pull request flow.

- Read access is enough for direct pull request diff fetches.
- Maintained pull request comments, commit statuses, and check-style output may require additional write permissions.
- `TRUST_GITHUB_WEBHOOK_SECRET` enables signed webhook intake.
- `TRUSTGATE_PUBLIC_URL` lets GitHub artifacts link back to saved TrustGate views.

## Local Notes

- The backend stores rule sets, templates, and review history in SQLite at `TRUST_DB_PATH`.
- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- Repo-specific templates control how TrustGate speaks in GitHub without changing the underlying review logic.
- Saved decisions can be reopened as web views, printed, or exported.
- `PATCHHIVE_REPO_MEMORY_URL` can optionally enrich reviews with remembered testing expectations, hotspots, and failure patterns.
- When RepoMemory is configured, `warn` and `block` results are submitted as FailGuard lesson candidates with finding evidence and affected paths.
- Generate the first local API key from `http://localhost:5175`.

## Repository Model

The PatchHive monorepo is the source of truth for TrustGate development. The standalone `patchhive/trustgate` repository is an exported mirror of this directory.

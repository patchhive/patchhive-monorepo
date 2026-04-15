# MergeKeeper by PatchHive

MergeKeeper turns pull request merge pressure into a clear readiness call.

It reads a pull request the way a strong reviewer or engineering lead would: what is still blocked, what is risky, what is merely waiting, and what is actually ready? Then it returns a simple state such as `ready`, `hold`, or `blocked` with the reasons behind that decision.

## Core Workflow

- fetch pull request metadata, reviewer decisions, review-thread pressure, and check health
- convert that into a single merge-readiness state
- show blockers and hold-level warnings clearly
- optionally fold in ReviewBee, TrustGate, and RepoMemory context
- keep the result visible inside GitHub through comments and check-style output

MergeKeeper is merge-readiness-first. It does not merge code for you in the MVP. Its job is to make the path to merge obvious and explainable.

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5178`
Backend: `http://localhost:8050`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## GitHub Access

MergeKeeper works best with a fine-grained personal access token.

- If you only want public repositories, keep the token public-only.
- Reading pull requests, reviews, and commit health is enough for the base product loop.
- Maintained GitHub comments or check-style output may need extra write permissions.
- `MERGE_KEEPER_GITHUB_WEBHOOK_SECRET` enables signed webhook refreshes.

## Integrations

MergeKeeper is stronger when the rest of PatchHive is available, but it still works on its own.

- `PATCHHIVE_REVIEW_BEE_URL` adds review churn context.
- `PATCHHIVE_TRUST_GATE_URL` adds safety and policy pressure.
- `PATCHHIVE_REPO_MEMORY_URL` adds repo-specific merge expectations.

If those services are not configured, MergeKeeper falls back to GitHub-only readiness logic.

## Local Notes

- The backend stores readiness history in SQLite at `MERGE_KEEPER_DB_PATH`.
- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- `MERGE_KEEPER_PUBLIC_URL` lets GitHub artifacts link back to a saved run.
- Generate the first local API key from `http://localhost:5178`.

## Repository Model

The PatchHive monorepo is the source of truth for MergeKeeper development. The standalone `patchhive/mergekeeper` repository is an exported mirror of this directory.

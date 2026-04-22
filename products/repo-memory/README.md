# RepoMemory by PatchHive

RepoMemory turns merged history and review pain into durable repo memory.

It captures what a repository has already learned from merged pull requests, reviewer feedback, recurring bug themes, and repeated hotspots so humans and agents do not keep rediscovering the same architectural expectations over and over.

## Core Workflow

- ingest merged pull requests, review feedback, issues, and file hotspots
- extract memory entries with evidence and confidence
- build reviewer and maintainer profile memories from repeated patterns
- store curated memories as signals, policies, or suppressed items
- queue, review, dismiss, and promote FailGuard lessons from bugs, outages, rejected patches, and painful reviews
- expose prompt-pack and context endpoints for other PatchHive products
- compare each ingest to the previous one so memory drift is visible over time

RepoMemory is intentionally context-first. It does not open pull requests or mutate repositories in the MVP.

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5176`
Backend: `http://localhost:8030`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## GitHub Access

RepoMemory works best with a fine-grained personal access token.

- If you only want public repositories, keep the token public-only.
- Reading merged pull requests, reviews, and issues is enough for the core MVP loop.
- Put the token in `BOT_GITHUB_TOKEN`.

## Cross-Product Use

RepoMemory is already useful on its own, but it also acts as infrastructure for the rest of PatchHive.

- RepoReaper can use it before patch generation.
- TrustGate can use it before diff review.
- MergeKeeper can use it for repo-specific merge expectations.
- FailGuard uses it to turn reviewed bad outcomes into pinned failure-pattern policy memories.

When enabled, downstream products can call RepoMemory through `PATCHHIVE_REPO_MEMORY_URL`.

## FailGuard Lessons

RepoMemory owns the FailGuard review loop:

- `GET /failguard/candidates` lists suggested lessons by repo and status.
- `POST /failguard/candidates` queues a bad outcome from an operator or another product.
- `POST /failguard/candidates/:id/promote` turns a reviewed candidate into a curated `failure_pattern` memory.
- `POST /failguard/candidates/:id/dismiss` rejects noisy or unhelpful candidates.
- `POST /failguard/lessons` still captures an already-approved lesson directly.

Promoted lessons carry path evidence, a prevention rule, and policy/pinned curation by default. TrustGate already consumes these memories through the RepoMemory context endpoint, so approved FailGuard lessons can become future warnings or blocks without making FailGuard a separate product.

## Local Notes

- The backend stores runs and memory entries in SQLite at `REPO_MEMORY_DB_PATH`.
- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The generated prompt pack is meant to be reused as context, not treated as infallible policy.
- Generate the first local API key from `http://localhost:5176`.

## Repository Model

The PatchHive monorepo is the source of truth for RepoMemory development. The standalone `patchhive/repomemory` repository is an exported mirror of this directory.

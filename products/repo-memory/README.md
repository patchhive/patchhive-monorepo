# RepoMemory by PatchHive

RepoMemory turns merged history and review pain into durable repo memory.

It captures what a repository has already learned from merged pull requests, reviewer feedback, recurring bug themes, and repeated hotspots so humans and agents do not keep rediscovering the same architectural expectations over and over.

## Core Workflow

- ingest merged pull requests, review feedback, issues, and file hotspots
- extract memory entries with evidence and confidence
- build reviewer and maintainer profile memories from repeated patterns
- store curated memories as signals, policies, or suppressed items
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

When enabled, downstream products can call RepoMemory through `PATCHHIVE_REPO_MEMORY_URL`.

## Local Notes

- The backend stores runs and memory entries in SQLite at `REPO_MEMORY_DB_PATH`.
- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The generated prompt pack is meant to be reused as context, not treated as infallible policy.
- Generate the first local API key from `http://localhost:5176`.

## Repository Model

The PatchHive monorepo is the source of truth for RepoMemory development. The standalone `patchhive/repomemory` repository is an exported mirror of this directory.

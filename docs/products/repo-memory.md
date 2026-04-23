# RepoMemory

<p align="center">
  <img src="../../patchhive3.png" width="120" alt="PatchHive logo" />
</p>

RepoMemory turns merged history, review feedback, recurring failures, and file
hotspots into durable repository knowledge that humans and PatchHive products
can reuse.

Standalone repo: [patchhive/repomemory](https://github.com/patchhive/repomemory)

## Product Role

RepoMemory is the durable context layer in PatchHive. It helps other products
understand what a repository has already taught the team: conventions, risky
areas, reviewer preferences, repeated failure modes, and useful prompt context.

## Core Workflow

1. Ingest merged pull requests, review comments, issues, and hotspot evidence.
2. Extract candidate memories with evidence and confidence.
3. Store curated memories as signals, policies, or suppressed items.
4. Build prompt packs and product-specific context slices.
5. Queue and review FailGuard lesson candidates.
6. Promote useful failure lessons into durable `failure_pattern` memories.

## Inputs

- GitHub token with read access.
- Repository history, merged pull requests, reviews, issues, and file hotspots.
- FailGuard candidates from operators, TrustGate, RepoReaper, or future products.

## Outputs

- Curated memory entries with evidence and confidence.
- Prompt packs for coding agents.
- Product-specific context for RepoReaper, TrustGate, ReviewBee, and MergeKeeper.
- FailGuard lessons promoted into durable future guardrails.
- Memory drift comparisons across ingests.

## Safety Boundary

RepoMemory is context-first. It should not mutate repositories or open pull
requests. Its generated prompt packs and memories should be treated as useful
context, not infallible policy.

## Local Development

```bash
cd products/repo-memory
cp .env.example .env
docker compose up --build
```

Defaults:

- Frontend: `http://localhost:5176`
- Backend: `http://localhost:8030`
- Database: `REPO_MEMORY_DB_PATH`

Split local workflow:

```bash
cd products/repo-memory/backend
cargo run

cd ../frontend
npm install
npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` | GitHub read token. |
| `REPO_MEMORY_API_KEY_HASH` | Optional preconfigured API-key hash. |
| `REPO_MEMORY_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `REPO_MEMORY_DB_PATH` | SQLite database path. |
| `REPO_MEMORY_PORT` | Backend port. |

## Cross-Product Fit

- RepoReaper can use RepoMemory before patch planning.
- TrustGate can use RepoMemory to apply remembered failure patterns and test
  expectations.
- MergeKeeper can use RepoMemory to understand repo-specific merge expectations.
- ReviewBee can use RepoMemory for reviewer-preference context.
- FailGuard uses RepoMemory as the storage and review loop for future guardrails.

## HiveCore Fit

HiveCore should surface RepoMemory health, run history, and context availability
without reading the RepoMemory database directly.


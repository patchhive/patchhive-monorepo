# 🧠 RepoMemory by PatchHive

> Turn merged history and review pain into durable repo memory.

RepoMemory is the durable memory layer for PatchHive. It ingests merged PRs, reviewer feedback, recurring bug signals, and file hotspots so humans and agents can reuse what the repo has already learned instead of rediscovering the same rules every week.

## What It Does

- ingests recent merged PRs for a target repo
- mines reviewer comments and review summaries for repeated conventions
- finds repeated bug terms in recent closed issues
- tracks high-context hotspots where fixes and review churn keep landing
- stores durable memory entries with evidence and confidence
- generates a prompt-pack agents can reuse before they touch the repo
- exposes a context endpoint other PatchHive products can query with changed paths and task summary
- keeps ingest history so teams can reopen prior memory snapshots

RepoMemory is intentionally context-first. It does not open PRs or mutate repositories in the MVP.

## Quick Start

```bash
cp .env.example .env

# Backend
cd backend && cargo run

# Frontend
cd ../frontend && npm install && npm run dev
```

Backend: `http://localhost:8030`
Frontend: `http://localhost:5176`

## Local Run Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend stores runs and extracted memory entries in SQLite at `REPO_MEMORY_DB_PATH`.
- `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` is required for GitHub-backed ingestion.
- RepoMemory does not require a live AI provider for the MVP loop.
- The generated prompt pack is meant to be copied into later agent flows, not treated as perfect truth.
- `PATCHHIVE_REPO_MEMORY_URL` lets other PatchHive products retrieve repo context from this service.
- If RepoMemory auth is enabled, downstream callers can use `PATCHHIVE_REPO_MEMORY_API_KEY` with an `X-API-Key` header.

## Standalone Repo Notes

RepoMemory is developed in the PatchHive monorepo first. The standalone `patchhive/repomemory` repo should be treated as an exported mirror of this product directory rather than a second source of truth.

## Local AI Gateway

RepoMemory does not need `PATCHHIVE_AI_URL` for the first MVP loop. It currently builds memory from GitHub history plus deterministic extraction heuristics.

That said, it fits naturally into the wider PatchHive platform and can later route memory summarization or embedding-style retrieval through `patchhive-ai-local` if that becomes valuable.

*RepoMemory by PatchHive — memory before automation*

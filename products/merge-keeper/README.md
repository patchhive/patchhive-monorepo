# 🪢 MergeKeeper by PatchHive

> Keep pull requests mergeable by turning GitHub merge pressure into a clear readiness call.

MergeKeeper reads a GitHub pull request the way a busy engineering lead would: is it open, approved, still asking for changes, waiting on checks, stuck in review threads, or quietly ready to merge? Instead of forcing teams to piece that together from five GitHub surfaces, it returns a simple state: `ready`, `hold`, or `blocked`.

## What It Does

- fetches GitHub PR metadata, current reviewer states, review threads, and commit health
- turns merge pressure into a single readiness decision: `ready`, `hold`, or `blocked`
- shows the concrete blockers and hold-level warnings behind that decision
- stores local run history so teams can reload prior readiness calls
- stays read-only in the MVP while still helping teams keep long-lived PRs mergeable

MergeKeeper is intentionally merge-readiness-first. It does not merge code or rewrite branches in the MVP. Its job is to make the path to merge obvious before PatchHive takes on more automation.

## Quick Start

```bash
cp .env.example .env

# Backend
cd backend && cargo run

# Frontend
cd ../frontend && npm install && npm run dev
```

Backend: `http://localhost:8050`
Frontend: `http://localhost:5178`

## Local Run Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend stores readiness history in SQLite at `MERGE_KEEPER_DB_PATH`.
- `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` is required for GitHub-backed PR readiness checks.
- MergeKeeper does not require `PATCHHIVE_AI_URL` for the MVP loop.
- The current MVP reads GitHub PR state, review pressure, and check health into a simple readiness call.

## Standalone Repo Notes

MergeKeeper should be developed in the PatchHive monorepo first. Its standalone repo should be treated as an exported mirror of this product directory rather than a second source of truth.

*MergeKeeper by PatchHive — keep pull requests mergeable by turning GitHub merge pressure into a clear readiness call.*

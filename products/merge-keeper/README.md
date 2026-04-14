# 🪢 MergeKeeper by PatchHive

> Keep pull requests mergeable by turning GitHub merge pressure into a clear readiness call.

MergeKeeper reads a GitHub pull request the way a busy engineering lead would: is it open, approved, still asking for changes, waiting on checks, stuck in review threads, or quietly ready to merge? Instead of forcing teams to piece that together from five GitHub surfaces, it returns a simple state: `ready`, `hold`, or `blocked`.

## What It Does

- fetches GitHub PR metadata, current reviewer states, review threads, and commit health
- turns merge pressure into a single readiness decision: `ready`, `hold`, or `blocked`
- shows the concrete blockers and hold-level warnings behind that decision
- can optionally layer ReviewBee, TrustGate, and RepoMemory context into the final readiness call
- can maintain a GitHub PR comment and publish a check-style readiness signal
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
- Prefer a fine-grained personal access token over a classic PAT whenever your GitHub setup allows it.
- If you only want MergeKeeper on public repos, keep repository access public-only and avoid private repo access.
- `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` is required for GitHub-backed PR readiness checks.
- MergeKeeper mainly needs read access to pull requests, reviews, and commit health. Maintained PR comments or check-style output may need extra write permissions depending on your setup.
- `MERGE_KEEPER_GITHUB_WEBHOOK_SECRET` enables signed GitHub webhook refreshes.
- `MERGE_KEEPER_PUBLIC_URL` lets maintained PR comments link back to a saved MergeKeeper run with `?run=<id>`.
- MergeKeeper does not require `PATCHHIVE_AI_URL` for the MVP loop.
- The base MVP reads GitHub PR state, review pressure, and check health into a simple readiness call.
- Manual runs can maintain a single GitHub PR comment and publish a check-style readiness signal.
- If `PATCHHIVE_REVIEW_BEE_URL` is set, MergeKeeper can fold open review churn into readiness.
- If `PATCHHIVE_TRUST_GATE_URL` is set, MergeKeeper can keep risky PRs on `hold` or `blocked` even when checks are green.
- If `PATCHHIVE_REPO_MEMORY_URL` is set, MergeKeeper can add repo-specific merge expectations and reviewer tendencies to the call.
- All three integrations are optional. MergeKeeper still works fine as a standalone GitHub-only readiness tool.

## Standalone Repo Notes

MergeKeeper should be developed in the PatchHive monorepo first. Its standalone repo should be treated as an exported mirror of this product directory rather than a second source of truth.

*MergeKeeper by PatchHive — keep pull requests mergeable by turning GitHub merge pressure into a clear readiness call.*

# 🐝 ReviewBee by PatchHive

> Turn reviewer churn into a concrete merge checklist.

ReviewBee reads pull request review comments, identifies which ones are actually actionable, groups similar requests together, and turns them into a clear checklist for the author. Instead of making engineers reread long review threads, it helps teams understand what still needs to change before a PR can merge.

## What It Does

- fetches GitHub PR reviews and review threads for a target PR
- filters out praise/noise and keeps the parts that still sound actionable
- clusters similar review feedback into concrete checklist items
- tracks resolved vs still-open review themes
- suggests follow-up prompts the author or an agent can use to clear review churn faster
- stores review history locally so teams can reload previous checklists
- can maintain a single ReviewBee PR comment so the checklist stays attached to the pull request
- can refresh itself from GitHub webhooks when new review activity lands

ReviewBee is intentionally review-first. It helps close PRs faster, but it does not edit code. Its GitHub-facing artifact is a maintained checklist comment, not a write-to-code action.

## Quick Start

```bash
cp .env.example .env

# Backend
cd backend && cargo run

# Frontend
cd ../frontend && npm install && npm run dev
```

Backend: `http://localhost:8040`
Frontend: `http://localhost:5177`

## Local Run Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend stores review history in SQLite at `REVIEW_BEE_DB_PATH`.
- `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` is required for GitHub-backed PR review analysis.
- `REVIEW_BEE_GITHUB_WEBHOOK_SECRET` enables signed GitHub webhook refreshes on PR review activity.
- `REVIEW_BEE_PUBLIC_URL` lets ReviewBee link maintained PR comments back to its own history view.
- ReviewBee does not require `PATCHHIVE_AI_URL` for the MVP loop.
- The current loop reads review threads and current resolution state, turns that into a merge checklist, and can keep one maintained PR comment in sync with the latest run.

## Standalone Repo Notes

ReviewBee should be developed in the PatchHive monorepo first. When it gets its own repository later, that standalone repo should be treated as an exported mirror of this product directory rather than a second source of truth.

*ReviewBee by PatchHive — turn reviewer churn into a concrete merge checklist.*

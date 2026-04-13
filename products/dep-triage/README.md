# 📦 DepTriage by PatchHive

> Tell teams which dependency updates matter now and which ones can wait.

DepTriage is PatchHive's dependency-noise filter. It reads open dependency PRs, optionally folds in Dependabot security alerts, and turns that flood into a ranked queue of `update now`, `watch`, and `ignore for now` calls. The goal is not to replace Dependabot or Renovate. The goal is to tell a team which updates are actually worth spending attention on right now.

## What It Does

- scans open dependency pull requests in a GitHub repo
- optionally reads open Dependabot alerts for the same repo
- groups overlapping PRs and alerts around the same package
- scores each dependency into `update now`, `watch`, or `ignore for now`
- saves scan history so teams can reopen prior triage calls later

DepTriage is intentionally read-only in the MVP. It does not merge dependency PRs, close alerts, or rewrite update configuration.

## Quick Start

```bash
cp .env.example .env

# Backend
cd backend && cargo run

# Frontend
cd ../frontend && npm install && npm run dev
```

Backend: `http://localhost:8070`
Frontend: `http://localhost:5180`

## Local Run Notes

- `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` is recommended so DepTriage can read dependency PRs with healthier rate limits and query Dependabot alerts.
- Dependabot alert reads require token access; if that fails, DepTriage will still score dependency PRs and surface a warning.
- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend stores scan history in SQLite at `DEP_TRIAGE_DB_PATH`.
- DepTriage does not require `PATCHHIVE_AI_URL` for the first MVP loop.

## Standalone Repo Notes

DepTriage should be developed in the PatchHive monorepo first. The standalone `patchhive/deptriage` repo should be treated as an exported mirror of this product directory rather than a second source of truth.

*DepTriage by PatchHive — Tell teams which dependency updates matter now and which ones can wait.*

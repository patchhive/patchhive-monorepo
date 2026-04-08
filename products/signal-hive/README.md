# 📡 SignalHive by PatchHive

> See the maintenance work your team is missing.

SignalHive is the read-only reconnaissance layer for PatchHive. It scans repository and issue history to surface stale risks, duplicate issues, recurring bug patterns, TODO hotspots, and hidden maintenance drag before it turns into downtime or delivery friction.

## What It Does

- discovers repositories from broad search terms, topics, and languages
- samples open issue history to find stale backlog risk
- flags likely duplicate issue reports
- clusters recurring bug-like issues into repeated failure patterns
- counts TODO and FIXME hotspots through GitHub code search
- respects allowlist, denylist, and opt-out controls during discovery
- saves reusable scan presets for recurring maintenance views
- ranks repositories into a maintenance queue your team can actually work from

SignalHive is intentionally read-only. It does not open pull requests, write code, or mutate repositories.

## Quick Start

```bash
cp .env.example .env
# Fill in BOT_GITHUB_TOKEN

# Backend
cd backend && cargo run

# Frontend
cd ../frontend && npm install && npm run dev
```

Backend: `http://localhost:8010`
Frontend: `http://localhost:5174`

## Local Run Notes

- The frontend uses `@patchhivehq/ui` from the public npm registry.
- The backend stores scan history in SQLite at `SIGNAL_DB_PATH`.
- This product is designed to be the visibility-first entry point into PatchHive.
- Repo discovery can be constrained with allowlist, denylist, and opt-out controls in the UI.

## Standalone Repo Notes

SignalHive is developed in the PatchHive monorepo first. When it gets its own repository later, that standalone repo should be treated as an exported mirror of this product directory rather than a second source of truth.

## Local AI Gateway

SignalHive does not require AI to produce its first signals, so `PATCHHIVE_AI_URL` is not part of the MVP loop.

That said, it still fits into the wider PatchHive platform and can eventually route summarization or scoring work through `patchhive-ai-local` when that becomes valuable.

*SignalHive by PatchHive — maintenance visibility before automation*

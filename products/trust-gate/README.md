# 🛡 TrustGate by PatchHive

> Review AI-generated diffs before they move forward.

TrustGate is the trust and safety layer for PatchHive. It reviews AI-generated diffs against repo-specific risk rules, flags dangerous or suspicious changes, and returns a simple recommendation: `safe`, `warn`, or `block`.

## What It Does

- ingests pasted unified diffs from AI-generated patches
- loads repo-specific risk rules for path restrictions, suspicious terms, and change limits
- flags risky file changes such as workflow, infrastructure, migration, or auth-adjacent edits
- warns when code changes land without tests
- highlights suspicious added lines such as secret-like material, `eval`, `unsafe`, or shell-heavy changes
- stores review history so teams can look back at prior decisions
- returns a clean recommendation that downstream automation can use

TrustGate is intentionally review-first. It does not write code or mutate repositories in the MVP.

## Quick Start

```bash
cp .env.example .env
# Optional: set BOT_GITHUB_TOKEN later if you want GitHub status-check integration

# Backend
cd backend && cargo run

# Frontend
cd ../frontend && npm install && npm run dev
```

Backend: `http://localhost:8020`
Frontend: `http://localhost:5175`

## Local Run Notes

- The frontend uses `@patchhivehq/ui` from the public npm registry.
- The frontend uses `@patchhivehq/product-shell` for the shared PatchHive API-key login flow.
- The backend stores rules and review history in SQLite at `TRUST_DB_PATH`.
- Repo-specific rule sets are the main product memory in the MVP.
- GitHub integration is optional for now; the core review loop works on pasted diffs alone.

## Standalone Repo Notes

TrustGate is developed in the PatchHive monorepo first. When it gets its own repository later, that standalone repo should be treated as an exported mirror of this product directory rather than a second source of truth.

## Local AI Gateway

TrustGate reviews AI-generated diffs, but it does not need a live AI provider to make its first decisions. `PATCHHIVE_AI_URL` is not part of the MVP loop.

That said, it still fits into the wider PatchHive platform and can eventually route richer policy summaries through `patchhive-ai-local` when that becomes valuable.

*TrustGate by PatchHive — trust before automation*

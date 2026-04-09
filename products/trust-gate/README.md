# 🛡 TrustGate by PatchHive

> Review AI-generated diffs before they move forward.

TrustGate is the trust and safety layer for PatchHive. It reviews AI-generated diffs against repo-specific risk rules, flags dangerous or suspicious changes, and returns a simple recommendation: `safe`, `warn`, or `block`.

## What It Does

- ingests pasted unified diffs from AI-generated patches
- fetches GitHub pull-request diffs directly from `owner/repo + PR number`
- accepts signed `pull_request` webhook payloads so review can happen automatically on PR updates
- pushes TrustGate recommendations back to GitHub as a check run when possible, with commit-status fallback
- maintains a single PR comment with the current TrustGate report so maintainers can read the review in-thread
- loads repo-specific risk rules for path restrictions, suspicious terms, and change limits
- offers reusable rule packs for app, library, infra, and agent-generated patch repos
- flags risky file changes such as workflow, infrastructure, migration, or auth-adjacent edits
- escalates missing-test findings more aggressively when risky source files or larger diffs are involved
- handles generated artifacts and large risky diffs more deliberately instead of treating them like ordinary source edits
- highlights suspicious added lines such as secret-like material, `eval`, `unsafe`, or shell-heavy changes
- stores review history so teams can look back at prior decisions
- returns a clean recommendation that downstream automation can use

TrustGate is intentionally review-first. It does not write code or mutate repositories in the MVP.

## Quick Start

```bash
cp .env.example .env
# Optional but recommended if you want private PR fetches and GitHub status/check output:
# BOT_GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxx
# TRUST_GITHUB_WEBHOOK_SECRET=replace-me
# TRUSTGATE_PUBLIC_URL=https://trustgate.your-domain.example

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
- GitHub integration is optional; the core review loop still works on pasted diffs alone.
- `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` enables private PR fetches plus GitHub report delivery.
- `TRUST_GITHUB_WEBHOOK_SECRET` enables signed `pull_request` webhook intake.
- `TRUSTGATE_PUBLIC_URL` lets GitHub checks/statuses link back to TrustGate review details.

## Standalone Repo Notes

TrustGate is developed in the PatchHive monorepo first. The standalone `patchhive/trustgate` repo should be treated as an exported mirror of this product directory rather than a second source of truth.

## Local AI Gateway

TrustGate reviews AI-generated diffs, but it does not need a live AI provider to make its first decisions. `PATCHHIVE_AI_URL` is not part of the MVP loop.

That said, it still fits into the wider PatchHive platform and can eventually route richer policy summaries through `patchhive-ai-local` when that becomes valuable.

*TrustGate by PatchHive — trust before automation*

# 🦂 FlakeSting by PatchHive

> Spot flaky CI patterns before unreliable checks erode team trust.

FlakeSting reads recent GitHub Actions history, looks for fail/pass swings in test-like jobs and steps, and turns that churn into a ranked flaky-CI queue. Instead of waiting for engineers to develop a vague sense that a repo's CI is "kind of unreliable," it highlights the exact jobs, steps, and runner environments that keep wobbling between red and green.

## What It Does

- fetches recent GitHub Actions workflow runs and workflow jobs for a target repository
- filters that history by branch and workflow name when you want a narrower view
- detects fail/pass swings in test-like jobs and steps instead of treating every CI failure as equally meaningful
- scores likely flaky signals as `suspect` or `quarantine`
- surfaces rerun pressure, runner/environment hints, and evidence links back to the underlying GitHub runs
- stores local scan history so teams can reload prior scans and compare recurring CI trust debt
- compares each scan to the last comparable one so teams can tell whether flaky pressure is rising, improving, or just shifting around
- lets operators sort the flaky queue and copy a scan summary for quick sharing
- works without AI in the MVP loop by leaning on GitHub Actions history and deterministic heuristics

FlakeSting is intentionally CI-trust-first. It does not rerun workflows, edit code, or rewrite CI config in the MVP. Its job is to make unstable checks visible before they quietly erode confidence in the rest of the delivery pipeline.

## Quick Start

```bash
cp .env.example .env

# Backend
cd backend && cargo run

# Frontend
cd ../frontend && npm install && npm run dev
```

Backend: `http://localhost:8060`
Frontend: `http://localhost:5179`

## Local Run Notes

- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- The backend stores scan history in SQLite at `FLAKE_STING_DB_PATH`.
- Prefer a fine-grained personal access token over a classic PAT whenever your setup allows it.
- If you only want FlakeSting on public repos, keep repository access public-only and avoid private repo access.
- `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` is strongly recommended so FlakeSting can read workflow runs and jobs with healthier rate limits.
- FlakeSting mainly needs read access to GitHub Actions workflow runs and jobs.
- FlakeSting does not require `PATCHHIVE_AI_URL` for the MVP loop.
- The current scan loop reads recent GitHub Actions runs, looks for fail/pass swings, and ranks unstable test jobs or steps before they become background noise.
- The current UI is designed for one repo at a time so the scoring, trend comparison, and evidence stay easy to trust while the product sharpens.

## Standalone Repo Notes

FlakeSting should be developed in the PatchHive monorepo first. Its standalone repo should be treated as an exported mirror of this product directory rather than a second source of truth.

*FlakeSting by PatchHive — Spot flaky CI patterns before unreliable checks erode team trust.*

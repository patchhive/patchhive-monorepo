# ReviewBee by PatchHive

ReviewBee turns reviewer churn into a concrete pull request checklist.

It reads review comments and review threads, separates actionable feedback from noise, groups similar requests into one follow-up item, and keeps the result attached to the pull request so authors can see what still matters.

## Product Documentation

- GitHub-facing product doc: [docs/products/review-bee.md](../../docs/products/review-bee.md)
- Product docs index: [docs/products/README.md](../../docs/products/README.md)

## Core Workflow

- fetch reviews and review threads for a target pull request
- keep the actionable parts and collapse repetition
- cluster feedback into a concrete checklist
- track what still appears unresolved
- optionally maintain a single GitHub comment with the current checklist
- refresh from signed webhooks when review activity changes

## Run Locally

### Docker

```bash
cp .env.example .env
docker compose up --build
```

Frontend: `http://localhost:5177`
Frontend v2 prototype: `http://localhost:5196`
Backend: `http://localhost:8040`

### Split Backend and Frontend

Run these in separate terminals:

```bash
cp .env.example .env

cargo run --manifest-path backend/Cargo.toml
npm --prefix frontend install && npm --prefix frontend run dev
npm --prefix frontend-v2 install && npm --prefix frontend-v2 run dev
```

Run the backend command from `products/review-bee` so it loads the product-root `.env`.

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` | Optional fine-grained PAT for pull request review reads. Analysis-only scopes: Metadata (read), Pull requests (read). Add Issues (write) when ReviewBee should maintain its PR comment artifact. |
| `REVIEW_BEE_GITHUB_WEBHOOK_SECRET` | Optional signed webhook secret for review refreshes. |
| `REVIEW_BEE_PUBLIC_URL` | Optional public URL for links from maintained comments back to saved runs. |
| `REVIEW_BEE_API_KEY_HASH` | Optional pre-seeded app auth hash. Otherwise generate the first local key from the UI. |
| `REVIEW_BEE_SERVICE_TOKEN_HASH` | Optional pre-seeded service-token hash for HiveCore or other PatchHive product callers. |
| `REVIEW_BEE_DB_PATH` | SQLite path for review history. |
| `REVIEW_BEE_PORT` | Backend port for split local runs. |
| `RUST_LOG` | Rust logging level. |

ReviewBee works best with a fine-grained GitHub token. Reading pull requests, reviews, and review threads is enough for the core product loop. Maintained checklist comments need the smallest write permission that supports PR comment updates in your environment.

## Safety Boundary

ReviewBee is intentionally review-first. It does not edit code, approve pull requests, resolve review threads, or merge anything. Its job is to make review work easier to understand and easier to clear.

## Current Analysis Scope

ReviewBee currently checks pull request review state, not the pull request diff itself. It fetches PR metadata, formal reviews, and review threads, then uses deterministic heuristics to turn actionable reviewer comments into checklist items.

When ReviewBee reports `clear`, it means it did not find actionable unresolved review feedback in the available PR review threads. It does not mean the PR is technically safe to merge, CI-clean, risk-free, or code-reviewed deeply. Merge readiness belongs in MergeKeeper, and code/diff risk belongs in TrustGate.

Current non-goals:

- inspect PR diffs for code quality
- validate CI/check status
- decide mergeability
- resolve GitHub review threads
- read top-level PR conversation comments
- prove that requested changes were implemented in code

## HiveCore Fit

HiveCore can surface ReviewBee health, capabilities, run history, and unresolved review pressure. MergeKeeper can eventually use ReviewBee output as one input to merge readiness, while ReviewBee keeps owning PR review analysis.

## Future Depth

ReviewBee can get stronger without becoming a general merge gate. Good next layers are top-level PR conversation comments, diff-aware context, CI/check status context, and explicit handoffs to TrustGate and MergeKeeper. The intent is to make the final PatchHive call stronger while keeping ReviewBee focused on review-feedback pressure.

## Standalone Repository

The PatchHive monorepo is the source of truth for ReviewBee development. The standalone [`patchhive/reviewbee`](https://github.com/patchhive/reviewbee) repository is an exported mirror of this directory.

## Local Notes

- The v2 prototype lives in `frontend-v2/` while the suite UI direction is being settled.

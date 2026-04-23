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
Backend: `http://localhost:8040`

### Split Backend and Frontend

```bash
cp .env.example .env

cd backend && cargo run
cd ../frontend && npm install && npm run dev
```

## Important Configuration

| Variable | Purpose |
| --- | --- |
| `BOT_GITHUB_TOKEN` or `GITHUB_TOKEN` | Optional GitHub token for pull request review reads. |
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

## HiveCore Fit

HiveCore can surface ReviewBee health, capabilities, run history, and unresolved review pressure. MergeKeeper can eventually use ReviewBee output as one input to merge readiness, while ReviewBee keeps owning PR review analysis.

## Standalone Repository

The PatchHive monorepo is the source of truth for ReviewBee development. The standalone [`patchhive/reviewbee`](https://github.com/patchhive/reviewbee) repository is an exported mirror of this directory.

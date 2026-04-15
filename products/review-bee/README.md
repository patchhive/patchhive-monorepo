# ReviewBee by PatchHive

ReviewBee turns reviewer churn into a concrete pull request checklist.

It reads review comments and review threads, separates actionable feedback from noise, groups similar requests into one follow-up item, and keeps the result attached to the pull request so authors can see what still matters.

## Core Workflow

- fetch reviews and review threads for a target pull request
- keep the actionable parts and collapse repetition
- cluster feedback into a concrete checklist
- track what still appears unresolved
- optionally maintain a single GitHub comment with the current checklist
- refresh from signed webhooks when review activity changes

ReviewBee is intentionally review-first. It does not edit code. Its job is to make review work easier to understand and easier to clear.

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

## GitHub Access

ReviewBee works best with a fine-grained personal access token.

- If you only want public repositories, keep the token public-only.
- Reading pull requests, reviews, and review threads is enough for the core product loop.
- If you want ReviewBee to maintain a checklist comment in GitHub, add the smallest write permission that supports PR comment updates in your environment.

## Local Notes

- The backend stores review history in SQLite at `REVIEW_BEE_DB_PATH`.
- The frontend uses `@patchhivehq/ui` and `@patchhivehq/product-shell`.
- `REVIEW_BEE_GITHUB_WEBHOOK_SECRET` enables signed webhook refreshes.
- `REVIEW_BEE_PUBLIC_URL` lets ReviewBee link maintained comments back to saved runs.
- Generate the first local API key from `http://localhost:5177`.

## Repository Model

The PatchHive monorepo is the source of truth for ReviewBee development. The standalone `patchhive/reviewbee` repository is an exported mirror of this directory.
